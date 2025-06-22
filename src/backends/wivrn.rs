use crate::adb::device_manager::DeviceManager;
use crate::backends::{BackendStartInfo, VRBackend};
use crate::logging::log_channel::LogChannel;
use crate::TokioMutex;
use async_trait::async_trait;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

pub struct WiVRnBackend {
    server_process: Option<std::process::Child>,
    pub logger: Option<Arc<Mutex<LogChannel>>>,
}

#[async_trait]
impl VRBackend for WiVRnBackend {
    async fn start_async(&mut self, backend_log_channel: Arc<Mutex<LogChannel>>, device_manager: Arc<TokioMutex<DeviceManager>>) -> anyhow::Result<BackendStartInfo> {
        let mut needs_new_server_process = false;
        if self.server_process.is_none() {
            needs_new_server_process = true;
        } else if let Ok(Some(_)) = self.server_process.as_mut().unwrap().try_wait() {
            needs_new_server_process = true;
        }

        if needs_new_server_process {
            // Start the WiVRn server
            println!("Starting WiVRn server...");
            let mut server_process = Command::new("wivrn-server")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            self.logger.replace(backend_log_channel.clone());
            LogChannel::connect_std(backend_log_channel, &mut server_process);

            self.server_process.replace(server_process);
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            match self.server_process.as_mut().unwrap().try_wait()? {
                Some(status) => {
                    return Err(anyhow::anyhow!("WiVRn server exited unexpectedly with status {}", status));
                }
                None => {}
            }
            println!("Started WiVRn server");
        }

        // Find the serial of the connected Quest 2 device
        self.reconnect_async(device_manager.clone()).await?;

        let device_manager = device_manager.lock().await;
        let active_device = device_manager.get_current_device()?
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

        // Forward socket connection
        println!("Forwarding socket connection...");
        let device_manager = device_manager.lock().await;
        let active_device = device_manager.get_current_device()?
            .ok_or_else(|| anyhow::anyhow!("No active device found"))?;
        active_device.try_open_tcp_tunnel(9757)?;

        // Start the WiVRn client
        println!("Starting WiVRn client...");
        active_device.adb_shell_command(&[
            "am", "start",
            "-a", "android.intent.action.VIEW",
            "-d", "wivrn+tcp://127.0.0.1:9757",
            "package:org.meumeu.wivrn.github"
        ])?;

        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        if let Some(mut server_process) = self.server_process.take() {
            server_process.kill()?;
        }

        Ok(())
    }
}

impl WiVRnBackend {
    pub fn new() -> WiVRnBackend {
        WiVRnBackend {
            server_process: None,
            logger: None,
        }
    }
}