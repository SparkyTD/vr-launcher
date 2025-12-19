use crate::adb::device_manager::DeviceManager;
use crate::audio_api::AudioDevice;
use crate::backends::{BackendStartInfo, VRBackend};
use crate::logging::log_channel::LogChannel;
use crate::steam::launch_modifiers::LaunchModifier;
use crate::TokioMutex;
use async_trait::async_trait;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use anyhow::bail;
use sysinfo::System;
use crate::backends::wivrn::wivrn_launch_modifier::WiVRnLaunchModifier;
use crate::backends::wivrn::wivrn_log_handler::WiVRnLogHandler;

pub struct WiVRnBackend {
    server_binary_path: PathBuf,
    server_process: Option<std::process::Child>,
    pub logger: Option<Arc<Mutex<LogChannel>>>,
}

#[async_trait]
impl VRBackend for WiVRnBackend {
    async fn start_async(&mut self, backend_log_channel: Arc<Mutex<LogChannel>>, device_manager: Arc<TokioMutex<DeviceManager>>) -> anyhow::Result<BackendStartInfo> {
        let mut needs_new_server_process = false;
        if self.server_process.as_mut().is_none_or(|s| s.try_wait().is_ok_and(|p| p.is_some())) {
            needs_new_server_process = true;

            // Kill any existing processes
            let mut sys = System::new_all();
            sys.refresh_all();
            for process in sys.processes_by_name(OsStr::new("wivrn-server")) {
                println!("Killing existing WiVRn Server process {}: {:?}", process.pid(), process.name());
                match process.kill_and_wait() {
                    Ok(_) => (),
                    Err(e) => println!("Failed to kill existing WiVRn Server process: {:?}", e),
                }
            }
        }

        if needs_new_server_process {
            // Start the WiVRn server
            println!("Starting WiVRn server [{}]...", self.server_binary_path.display());
            let mut server_process = Command::new(self.server_binary_path.as_os_str())
                // .arg("--no-instructions")
                // .arg("--no-manage-active-runtime")
                .arg("--early-active-runtime")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            self.logger.replace(backend_log_channel.clone());
            LogChannel::connect_std(backend_log_channel.clone(), &mut server_process);

            {
                let mut backend_log_channel = backend_log_channel.lock()
                    .expect("Failed to lock backend log channel");
                backend_log_channel.set_log_handler(Box::new(WiVRnLogHandler {
                    device_manager: device_manager.clone(),
                }));
                drop(backend_log_channel);
            }

            self.server_process.replace(server_process);
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            match self.server_process.as_mut().unwrap().try_wait()? {
                Some(status) => {
                    let log_channel = backend_log_channel.lock()
                        .map_err(|e| anyhow::anyhow!("Failed to lock wiVRn log channel: {}", e))?;
                    let last_error_line = log_channel.get_stderr_lines().last();
                    return Err(anyhow::anyhow!("WiVRn server exited unexpectedly with status {}: {:?}",
                        status,
                        last_error_line.unwrap_or(&"Unknown error".into())
                    ));
                }
                None => {}
            }
            println!("Started WiVRn server");
        }

        // Find the serial of the connected Quest 2 device
        self.reconnect_async(device_manager.clone()).await?;

        let device_manager = device_manager.lock().await;
        let active_device = device_manager.get_current_device_async().await?
            .ok_or_else(|| anyhow::anyhow!("No active device found"))?;

        Ok(BackendStartInfo {
            vr_device_serial: active_device.usb_serial,
            vr_device_ip: active_device.ip_address,
            was_restarted: needs_new_server_process,
        })
    }

    async fn reconnect_async(&mut self, device_manager: Arc<TokioMutex<DeviceManager>>) -> anyhow::Result<()> {
        if self.server_process.is_none() {
            return Ok(());
        }

        Self::reconnect_static_async(device_manager).await
    }

    async fn is_ready(&self) -> anyhow::Result<bool> {
        let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR")?;
        let path = PathBuf::from_str(&format!("{}/wivrn/comp_ipc", xdg_runtime_dir))?;

        Ok(path.exists())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        if let Some(mut server_process) = self.server_process.take() {
            server_process.kill()?;
        }

        Ok(())
    }

    fn is_matching_audio_device(&self, device: &AudioDevice) -> bool {
        device.description.to_lowercase().contains("wivrn")
    }

    fn add_modifiers(&self, list: &mut Vec<Box<dyn LaunchModifier>>) -> anyhow::Result<()> {
        let manifest_path = Self::locate_wivrn_manifest(self.server_binary_path.clone())?;
        list.insert(0, Box::new(WiVRnLaunchModifier::new(manifest_path)));

        Ok(())
    }
}

impl WiVRnBackend {
    pub fn new(server_binary: Option<PathBuf>) -> anyhow::Result<WiVRnBackend> {
        Ok(WiVRnBackend {
            server_binary_path: Self::locate_server_binary_path(server_binary)?,
            server_process: None,
            logger: None,
        })
    }

    pub async fn reconnect_static_async(device_manager: Arc<TokioMutex<DeviceManager>>) -> anyhow::Result<()> {
        // Forward socket connection
        println!("Forwarding socket connection...");
        let device_manager = device_manager.lock().await;
        let active_device = device_manager.get_current_device_async().await?
            .ok_or_else(|| anyhow::anyhow!("No active device found"))?;
        active_device.try_open_tcp_tunnel(9757)?;

        // Start the WiVRn client
        println!("Starting WiVRn client...");
        active_device.adb_shell_command(&[
            "am", "start",
            "-a", "android.intent.action.VIEW",
            "-d", "wivrn+tcp://127.0.0.1:9757",
            "package:org.meumeu.wivrn.github",
        ])?;

        Ok(())
    }

    fn locate_wivrn_manifest(server_path: PathBuf) -> anyhow::Result<PathBuf> {
        // Development build
        let manifest_path = server_path
            .parent().unwrap()
            .parent().unwrap()
            .join("openxr_wivrn-dev.json");
        if manifest_path.exists() {
            return Ok(manifest_path)
        }

        // Envision build
        let manifest_path = server_path
            .parent().unwrap()
            .parent().unwrap()
            .join("share/openxr/1/openxr_wivrn.json");
        if manifest_path.exists() {
            return Ok(manifest_path)
        }

        // Default system install
        let manifest_path = PathBuf::from_str("/usr/share/openxr/1/openxr_wivrn.json")?;
        if manifest_path.exists() {
            return Ok(manifest_path)
        }

        bail!("WiVRn manifest file not found")
    }

    fn locate_server_binary_path(server_binary: Option<PathBuf>) -> anyhow::Result<PathBuf> {
        // Check if the path has been explicitly specified
        if let Some(server_binary) = server_binary {
            if server_binary.exists() && server_binary.is_file() {
                return Ok(server_binary)
            }
        }

        // Otherwise, try to evaluate the path of the "wivrn-server" command
        if let Ok(path) = which::which("wivrn-server") {
            return Ok(path);
        }

        bail!("The wivrn server binary was not found")
    }
}