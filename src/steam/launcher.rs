use crate::logging::log_channel::LogChannel;
use crate::steam::launch_modifiers::LaunchModifier;
use crate::steam::steam_interface::{ProtonVersion, SteamApp};
use std::sync::{Arc, Mutex};
use tokio::process;
use tokio::sync::broadcast::Sender;
use tokio::task::JoinHandle;
use uuid::Uuid;

pub struct CompatLauncher {}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ProcessHandle {
    pid: u32,
    process_token: Uuid,
    wait_handle: Option<JoinHandle<()>>,
}

impl ProcessHandle {
    pub fn get_process_token(&self) -> &Uuid {
        &self.process_token
    }

    #[allow(dead_code)]
    #[cfg(debug_assertions)]
    pub fn null() -> Self {
        Self {
            pid: 0,
            process_token: Uuid::new_v4(),
            wait_handle: None,
        }
    }
}

impl CompatLauncher {
    pub fn new() -> Self {
        Self {}
    }

    pub fn launch_app_compat(&self, app: &SteamApp, compat_version: &ProtonVersion, modifiers: Vec<Box<dyn LaunchModifier>>, sock_tx: Sender<String>, logger: Arc<Mutex<LogChannel>>) -> anyhow::Result<ProcessHandle> {
        let mut process = process::Command::new("python3");
        // Basic Proton Setup
        process.env("_", compat_version.executable_path.to_str().unwrap());
        process.arg(compat_version.executable_path.to_str().unwrap());
        process.arg("run");
        process.arg(&app.executable);
        process.current_dir(&app.working_directory);

        let process_token = Uuid::new_v4();
        process.env("SVRL_TOKEN", process_token.to_string());

        for modifier in modifiers {
            modifier.apply(&mut process, app, compat_version)?;
        }

        let mut child = process.spawn()?;
        let pid = child.id().unwrap();

        LogChannel::connect_tokio(logger, &mut child);

        Ok(ProcessHandle {
            pid,
            process_token,
            wait_handle: Some(tokio::task::spawn(async move {
                println!("Waiting for child process to exit (id={})", pid);
                let status = child.wait().await;
                println!("The child process has exited with status {:?}", status);
                _ = sock_tx.send("inactive".to_owned());
            }))
        })
    }
}