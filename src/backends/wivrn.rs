use crate::backends::{BackendStartInfo, VRBackend};
use crate::logging::log_channel::LogChannel;
use rusb::UsbContext;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

const ALLOWED_VENDOR_IDS: &[u16] = &[0x2833];

pub struct WiVRnBackend {
    server_process: Option<std::process::Child>,
    pub logger: Option<Arc<Mutex<LogChannel>>>,
}

impl VRBackend for WiVRnBackend {
    fn start(&mut self, logger: Arc<Mutex<LogChannel>>) -> anyhow::Result<BackendStartInfo> {
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

            self.logger.replace(logger.clone());
            LogChannel::connect_std(logger, &mut server_process);

            self.server_process.replace(server_process);
            thread::sleep(std::time::Duration::from_secs(2));
            match self.server_process.as_mut().unwrap().try_wait()? {
                Some(status) => {
                    return Err(anyhow::anyhow!("WiVRn server exited unexpectedly with status {}", status));
                }
                None => {}
            }
            println!("Started WiVRn server");
        }

        // Find the serial of the connected Quest 2 device
        let android_device = self.find_vr_device()?;
        self.reconnect()?;

        Ok(BackendStartInfo {
            vr_device_serial: android_device.serial,
            vr_device_ip: android_device.ip_address,
            was_restarted: needs_new_server_process,
        })
    }

    fn reconnect(&mut self) -> anyhow::Result<()> {
        if self.server_process.is_none() {
            return Ok(());
        }

        // Find the serial of the connected Quest 2 device
        let android_device = self.find_vr_device()?;
        println!("Android device found: {:?} ({})", android_device.name, android_device.serial);

        // Forward socket connection
        println!("Forwarding socket connection...");
        Command::new("adb")
            .args(&["-s", &android_device.serial, "reverse", "tcp:9757", "tcp:9757"])
            .spawn()?
            .wait()?;

        // Start the WiVRn client
        println!("Starting WiVRn client...");
        Command::new("adb")
            .args(&[
                "-s", &android_device.serial,
                "shell", "am", "start",
                "-a", "android.intent.action.VIEW",
                "-d", "wivrn+tcp://127.0.0.1:9757",
                "package:org.meumeu.wivrn.github"
            ])
            .spawn()?
            .wait()?;

        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        if let Some(mut server_process) = self.server_process.take() {
            server_process.kill()?;
        }

        Ok(())
    }

    fn is_hmd_mounted(&self) -> anyhow::Result<bool> {
        let vr_device = self.find_vr_device()?;
        let result = Command::new("adb")
            .args(&[
                "-s", &vr_device.serial,
                "shell", "dumpsys", "power"
            ])
            .output()?;
        for line in String::from_utf8(result.stdout)?.lines() {
            let line = line.trim();
            if line.is_empty() || !line.contains('=') {
                continue;
            }

            let parts = line.split('=').collect::<Vec<_>>();
            if parts[0] == "mWakefulness" && parts.len() == 2 {
                return Ok(parts[1] == "Awake");
            }
        }

        Ok(false)
    }
}

impl WiVRnBackend {
    pub fn new() -> WiVRnBackend {
        WiVRnBackend {
            server_process: None,
            logger: None,
        }
    }

    fn find_vr_device(&self) -> anyhow::Result<AndroidDevice> {
        let context = rusb::Context::new()?;
        for device in context.devices()?.iter() {
            let desc = device.device_descriptor()?;
            if let Ok(handle) = device.open() {
                let serial = handle.read_serial_number_string_ascii(&desc)?;
                let product = handle.read_product_string_ascii(&desc)?;
                let vid = desc.vendor_id();
                if !ALLOWED_VENDOR_IDS.contains(&vid) {
                    continue;
                }

                let ip_output = Command::new("adb")
                    .args(&[
                        "-s", &serial,
                        "shell", "ip", "addr", "show", "wlan0",
                    ])
                    .output()?;

                let ip_output = String::from_utf8(ip_output.stdout)?;
                let ip_address = ip_output
                    .lines()
                    .find(|line| line.contains("inet") && line.contains("scope global"))
                    .map(|line| line.split_ascii_whitespace().nth(1).unwrap())
                    .map(|ip| ip.split('/').nth(0).unwrap().to_string());

                return Ok(AndroidDevice {
                    serial,
                    name: product,
                    ip_address,
                });
            }
        }

        Err(anyhow::anyhow!("VR device not found!"))
    }
}

struct AndroidDevice {
    name: String,
    serial: String,
    ip_address: Option<String>,
}