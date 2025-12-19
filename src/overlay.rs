use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use crate::logging::log_channel::LogChannel;

const WLX_OVERLAY_BINARY: &str = "wlx-overlay-s";

pub struct WlxOverlayManager {
    overlay_process: Option<std::process::Child>,
}

impl WlxOverlayManager {
    pub fn new() -> Self {
        WlxOverlayManager {
            overlay_process: None,
        }
    }

    pub fn start(&mut self, logger: Arc<Mutex<LogChannel>>) -> anyhow::Result<()> {
        // Kill process if it already exists, to keep things fresh
        if let Some(process) = self.overlay_process.as_mut() {
            process.kill()?;
        }

        println!("Starting wlx-overlay-s server...");
        let mut overlay_process = Command::new(WLX_OVERLAY_BINARY)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .arg("--replace")
            .arg("--openxr")
            // .arg("--show")
            .spawn()?;
        LogChannel::connect_std(logger, &mut overlay_process);
        self.overlay_process.replace(overlay_process);

        std::thread::sleep(std::time::Duration::from_millis(500));
        match self.overlay_process.as_mut().unwrap().try_wait()? {
            Some(status) => {
                return Err(anyhow::anyhow!("Overlay process exited unexpectedly with status {}", status));
            }
            None => {}
        }
        println!("Started the wlx-overlay-s process");

        Ok(())
    }
    
    pub fn stop(&mut self) -> anyhow::Result<()> {
        if let Some(mut process) = self.overlay_process.take() {
            process.kill()?;
            process.wait()?;
            println!("Stopped wlx-overlay-s");
        }
        
        Ok(())
    }
}