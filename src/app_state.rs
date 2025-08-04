use crate::adb::device_manager::DeviceManager;
use crate::audio_api::PipeWireManager;
use crate::backends::wivrn::WiVRnBackend;
use crate::backends::{BackendType, VRBackend};
use crate::battery_monitor::BatteryMonitor;
use crate::command_parser::parse_linux_command;
use crate::logging::log_session::LogSession;
use crate::models::Game;
use crate::overlay::WlxOverlayManager;
use crate::steam::launch_modifiers::env_vars::EnvironmentVariablesModifier;
use crate::steam::launch_modifiers::steam::SteamLaunchModifier;
use crate::steam::launch_modifiers::wivrn::WiVRnLaunchModifier;
use crate::steam::launch_modifiers::LaunchModifier;
use crate::steam::launcher::CompatLauncher;
use crate::steam::steam_interface::{SteamApp, SteamInterface};
use crate::GameSession;
use anyhow::ensure;
use nix::libc::pid_t;
use std::collections::{HashMap, HashSet};
use std::env;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::System;
use tokio::sync::{broadcast, Mutex};
use tokio::time::Instant;

pub struct AppState {
    pub audio_api: PipeWireManager,
    pub steam_api: SteamInterface,
    pub launcher: Arc<CompatLauncher>,
    pub active_game_session: Option<GameSession>,
    pub sock_tx: broadcast::Sender<String>,
    pub device_manager: Arc<Mutex<DeviceManager>>,
    pub active_backend: Option<Box<dyn VRBackend + Send>>,
    pub backend_type: BackendType,
    pub battery_monitor: BatteryMonitor,
    pub overlay_manager: WlxOverlayManager,
    pub log_session: Option<LogSession>,
    pub launch_requests: HashSet<String>,
    pub socket_stop_tx: broadcast::Sender<()>,
}

pub type AppStateWrapper = Arc<Mutex<AppState>>;

impl AppState {
    pub async fn launch_game_async(&mut self, game: Game) -> anyhow::Result<()> {
        if let Some(_) = &self.active_game_session {
            return Err(anyhow::anyhow!("Another active game session is already running"));
        }

        let vr_backend_modifier = match game.vr_backend.to_lowercase().as_str() {
            "wivrn" => WiVRnLaunchModifier::new(),
            _ => return Err(anyhow::anyhow!("This VR backend is currently not supported!")),
        };
        let steam_modifier = SteamLaunchModifier::new();

        let mut modifiers: Vec<Box<dyn LaunchModifier>> = vec![
            Box::new(vr_backend_modifier),
            Box::new(steam_modifier),
        ];

        let steam_app = match (&game.steam_app_id, &game.command_line) {
            (Some(steam_id), None) => self.steam_api.get_installed_apps()?
                .into_iter()
                .find(|app| app.steam_id == *steam_id as u32)
                .ok_or(anyhow::anyhow!("Could not find Steam app with id {}", steam_id))?,
            (steam_id, Some(command_line)) => {
                let command = parse_linux_command(command_line)
                    .map_err(|err| anyhow::anyhow!("Could not parse launch command: {:?}", err))?;
                if command.env_vars.len() > 0 {
                    let modifier = EnvironmentVariablesModifier::new(command.env_vars);
                    modifiers.push(Box::new(modifier));
                }
                SteamApp {
                    steam_id: match steam_id {
                        Some(id) => *id as u32,
                        None => 0,
                    },
                    title: game.title.clone(),
                    is_vr_app: true,
                    app_folder: command.working_dir.clone().into(),
                    working_directory: command.working_dir.clone().into(),
                    executable: command.executable.clone().into(),
                    arguments: command.arguments,
                }
            }
            (None, None) => return Err(anyhow::anyhow!("Not enough information to launch the game!")),
        };

        let proton_version = match &game.proton_version {
            Some(version) => self.steam_api.get_proton_versions()?
                .into_iter()
                .find(|p| p.name == *version)
                .ok_or(anyhow::anyhow!("Missing proton version: {:?}!", version))?,
            None => return Err(anyhow::anyhow!("Running games without proton is currently not supported!")),
        };

        println!("Launching game: {:#?}", steam_app);

        // Create logging session
        self.start_log_session()?;

        // Set up backend
        if let Some(active_backend) = self.active_backend.as_mut() {
            active_backend.stop()?;
        }

        let mut backend: Box<dyn VRBackend + Send> = match game.vr_backend.as_str() {
            "wivrn" => {
                self.backend_type = BackendType::WiVRn;
                Box::new(WiVRnBackend::new())
            }
            _ => return Err(anyhow::anyhow!("This VR backend is currently not supported!")),
        };

        // Check if headset is currently mounted
        {
            let device_manager = self.device_manager.lock().await;
            let active_device = device_manager.get_current_device_async().await?
                .ok_or_else(|| anyhow::anyhow!("No active device found"))?;

            ensure!(
                active_device.is_hmd_mounted()?,
                "Please mount the headset before starting any games."
            );
            drop(device_manager);
        }

        // Start backend
        let backend_log_channel = self.log_session.as_mut().unwrap()
            .create_channel("vr_backend")?;
        let device_manager = self.device_manager.clone();
        let start_info = backend.start_async(backend_log_channel, device_manager).await?;
        self.battery_monitor.set_active_device_serial(start_info.vr_device_serial.clone());
        if let Some(device_ip) = start_info.vr_device_ip {
            self.battery_monitor.set_active_device_ip(device_ip);
        }

        // Wait for virtual audio devices to be registered
        let start = Instant::now();
        let (output_device, input_device) = loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            let output = self.audio_api.get_output_devices().iter()
                .find(|d| backend.is_matching_audio_device(d))
                .cloned();
            let input = self.audio_api.get_input_devices().iter()
                .find(|d| backend.is_matching_audio_device(d))
                .cloned();

            if start.elapsed().as_secs() > 3 && output.is_none() && input.is_none() {
                eprintln!("Couldn't find the virtual audio devices for this backend!");
                break (None, None);
            } else if output.is_some() && input.is_some() {
                break (output, input);
            }
        };

        if let (Some(output_device), Some(input_device)) = (output_device, input_device) {
            self.audio_api.set_default_output_device(&output_device);
            self.audio_api.set_default_input_device(&input_device);
        }

        self.active_backend.replace(backend);

        // Launch the game
        let game_log_channel = self.log_session.as_mut().unwrap()
            .create_channel("game")?;
        let process_handle = self.launcher.launch_app_compat(
            &steam_app,
            &proton_version,
            modifiers,
            self.sock_tx.clone(),
            game_log_channel,
        )?;
        
        println!("Started main game process. PID: {}", process_handle.get_pid());

        self.active_game_session.replace(GameSession {
            game,
            process_handle,
            start_time_epoch: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs(),
            vr_device_serial: start_info.vr_device_serial,
        });

        // Start the overlay
        if start_info.was_restarted {
            let backend_log_channel = self.log_session.as_mut().unwrap()
                .create_channel("overlay")?;
            self.overlay_manager.start(backend_log_channel)?;
        }

        let message = format!("active:{}", serde_json::to_string(self.active_game_session.as_ref().unwrap())?).into();
        _ = self.sock_tx.send(message);

        Ok(())
    }

    pub fn kill_active_game(&mut self) -> anyhow::Result<()> {
        let active_session = self.active_game_session.as_mut().unwrap();

        let mut sys = System::new_all();
        sys.refresh_all();

        for (pid, process) in sys.processes() {
            let environ = process.environ()
                .into_iter()
                .map(|s| s.clone().into_string().unwrap())
                .flat_map(|s| s.split_once('=').map(|(k, v)| (k.to_string(), v.to_string())))
                .collect::<HashMap<_, _>>();

            if let Some(process_token) = environ.get("SVRL_TOKEN") {
                if *process_token == active_session.process_handle.get_process_token().to_string() {
                    let pid = nix::unistd::Pid::from_raw(pid.as_u32() as pid_t);
                    _ = nix::sys::signal::kill(pid, Some(nix::sys::signal::Signal::SIGKILL));
                }
            }
        }

        self.game_process_died()
    }

    pub fn game_process_died(&mut self) -> anyhow::Result<()> {
        _ = self.active_game_session.take();
        _ = self.sock_tx.send("inactive".into());
        self.overlay_manager.stop()?;

        if let Some(active_backend) = self.active_backend.as_mut() {
            active_backend.stop()?;
        }

        Ok(())
    }

    pub fn start_log_session(&mut self) -> anyhow::Result<()> {
        if let Some(mut log_session) = self.log_session.take() {
            log_session.shutdown()?;
        }

        let logs_dir = env::current_dir()?
            .join("logs");
        let mut session = LogSession::new(logs_dir);
        session.archive_old_files()?;
        self.log_session.replace(session);

        Ok(())
    }

    pub async fn shutdown_async(&mut self) -> anyhow::Result<()> {
        if let Some(mut active_backend) = self.active_backend.take() {
            active_backend.stop()?;
        }

        if let Some(mut log_session) = self.log_session.take() {
            log_session.shutdown()?;
        }

        if let Some(_) = &self.active_game_session {
            self.kill_active_game()?;
        }
        
        let device_manager = self.device_manager.lock().await;
        device_manager.disconnect_tcpip()?;

        Ok(())
    }
}