use std::path::Path;
use std::process::Stdio;
use crate::logging::log_channel::LogChannel;
use crate::steam::launch_modifiers::LaunchModifier;
use crate::steam::steam_interface::{ProtonVersion, SteamApp};
use std::sync::{Arc, Mutex};
use anyhow::bail;
use tokio::process;
use tokio::sync::broadcast::Sender;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use uuid::Uuid;
use crate::app_state::AppStateWrapper;

pub struct CompatLauncher {
    app_state: Arc<RwLock<Option<AppStateWrapper>>>,
}

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
    
    pub fn get_pid(&self) -> u32 {
        self.pid
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
        Self { app_state: Arc::new(RwLock::new(None)) }
    }

    pub async fn set_app_state_async(&self, app_state: AppStateWrapper) {
        let mut app_state_lock = self.app_state.write().await;
        *app_state_lock = Some(app_state);
    }

    pub fn launch_app_compat(&self, app: &SteamApp, compat_version: &ProtonVersion, modifiers: Vec<Box<dyn LaunchModifier>>, sock_tx: Sender<String>, logger: Arc<Mutex<LogChannel>>) -> anyhow::Result<ProcessHandle> {
        // Check if app paths exist
        if !app.working_directory.exists() {
            bail!("The specified working directory does not exist.");
        }

        if !app.app_folder.exists() {
            bail!("The specified installation directory does not exist.");
        }

        if !Path::new(&app.app_folder.join(&app.executable)).exists() {
            bail!("The specified app executable does not exist.");
        }

        if !compat_version.executable_path.exists() {
            bail!("The specified compat tool's path does not exist.");
        }

        let mut process = process::Command::new("python3");

        // Process output
        process.stdout(Stdio::piped());
        process.stderr(Stdio::piped());

        // Basic Proton Setup
        process.env("_", compat_version.executable_path.to_str().unwrap());
        process.arg(compat_version.executable_path.to_str().unwrap());
        process.arg("run");
        process.arg(&app.executable);
        process.args(&app.arguments);
        process.current_dir(&app.working_directory);

        let process_token = Uuid::new_v4();
        process.env("SVRL_TOKEN", process_token.to_string());

        for modifier in modifiers {
            modifier.apply(&mut process, app, compat_version)?;
        }

        let mut child = process.spawn()?;
        let pid = child.id().unwrap();

        LogChannel::connect_tokio(logger, &mut child);

        let app_state_clone = self.app_state.clone();
        Ok(ProcessHandle {
            pid,
            process_token,
            wait_handle: Some(tokio::task::spawn(async move {
                println!("Waiting for child process to exit (id={})", pid);
                let status = child.wait().await;
                println!("The child process has exited with status {:?}", status);
                _ = sock_tx.send("inactive".to_owned());
                let app_state = app_state_clone.write().await;
                let app_state = app_state.as_ref().unwrap();
                let mut app_state = app_state.lock().await;
                _ = app_state.game_process_died();
            })),
        })
    }
}