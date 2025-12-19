use crate::steam::launch_modifiers::LaunchModifier;
use crate::steam::steam_interface::{ProtonVersion, SteamApp};
use std::path::PathBuf;
use std::{env, fs};
use tokio::process;

pub struct WiVRnLaunchModifier {
    manifest_path: PathBuf,
}

impl WiVRnLaunchModifier {
    pub fn new(manifest_path: PathBuf) -> Self {
        Self { manifest_path }
    }
}

impl LaunchModifier for WiVRnLaunchModifier {
    fn apply(&self, command: &mut process::Command, app: &SteamApp, _compat_version: Option<&ProtonVersion>) -> anyhow::Result<()> {
        self.apply_env_vars(command, app)?;

        let openxr_target_path = env::home_dir().unwrap()
            .join(".config/openxr/1/active_runtime.json");
        fs::create_dir_all(&openxr_target_path.parent().unwrap())?;
        if openxr_target_path.exists() {
            fs::remove_file(&openxr_target_path)?;
        }

        std::os::unix::fs::symlink(&self.manifest_path, &openxr_target_path)?;

        Ok(())
    }
}

impl WiVRnLaunchModifier {
    pub fn apply_env_vars(&self, command: &mut process::Command, _app: &SteamApp) -> anyhow::Result<()> {
        let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR")?;
        command.env("PRESSURE_VESSEL_IMPORT_OPENXR_1_RUNTIMES", "1");
        command.env("PRESSURE_VESSEL_FILESYSTEMS_RW", vec![
            format!("{}/wivrn_comp_ipc", xdg_runtime_dir),
            format!("{}/wivrn/comp_ipc", xdg_runtime_dir),
            format!("{}/monado_comp_ipc", xdg_runtime_dir),
        ].join(":"));

        Ok(())
    }
}