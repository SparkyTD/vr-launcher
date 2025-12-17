use crate::adb::device_manager::DeviceManager;
use crate::audio_api::AudioDevice;
use crate::backends::{BackendStartInfo, VRBackend};
use crate::logging::log_channel::{LogChannel, LogHandler, LogType};
use crate::TokioMutex;
use async_trait::async_trait;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use sysinfo::System;

// const WIVRN_SERVER_BINARY: &str = "/opt/github/WiVRn/build-dashboard/server/wivrn-server";
const WIVRN_SERVER_BINARY: &str = "/home/sparky/.local/share/envision/prefixes/076a4450-a365-4fd0-b9df-1e8672792c8f/bin/wivrn-server";

pub struct WiVRnBackend {
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
                process.kill_and_wait().map_err(|e| anyhow::anyhow!("{:?}", e))?;
            }
        }

        if needs_new_server_process {
            // Start the WiVRn server
            println!("Starting WiVRn server...");
            let mut server_process = Command::new(WIVRN_SERVER_BINARY)
                // .arg("--no-instructions")
                // .arg("--no-manage-active-runtime")
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
}

impl WiVRnBackend {
    pub fn new() -> WiVRnBackend {
        WiVRnBackend {
            server_process: None,
            logger: None,
        }
    }

    async fn reconnect_static_async(device_manager: Arc<TokioMutex<DeviceManager>>) -> anyhow::Result<()> {
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
}

struct WiVRnLogHandler {
    device_manager: Arc<TokioMutex<DeviceManager>>,
}

#[async_trait::async_trait]
impl LogHandler for WiVRnLogHandler {
    fn handle_message(&self, message: String, _log_type: LogType) {
        if message.contains("Exception in network thread: Socket shutdown") {
            let device_manager = self.device_manager.clone();
            tokio::spawn(async move {
                println!("WiVRn server exited unexpectedly, attempting to reconnect in 3 seconds...");
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                WiVRnBackend::reconnect_static_async(device_manager).await
            });
        }
    }
}