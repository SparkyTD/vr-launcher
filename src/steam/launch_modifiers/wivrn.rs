use crate::steam::launch_modifiers::LaunchModifier;
use crate::steam::steam_interface::{ProtonVersion, SteamApp};
use tokio::process;

pub struct WiVRnLaunchModifier;

impl WiVRnLaunchModifier {
    pub fn new() -> Self {
        Self {}
    }
}

impl LaunchModifier for WiVRnLaunchModifier {
    fn apply(&self, command: &mut process::Command, _app: &SteamApp, _compat_version: Option<&ProtonVersion>) -> anyhow::Result<()> {
        let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR")?;
        // command.env("XR_RUNTIME_JSON", "/usr/share/openxr/1/openxr_wivrn.json");
        // command.env("PRESSURE_VESSEL_FILESYSTEMS_RW", "$XDG_RUNTIME_DIR/wivrn/comp_ipc");
        command.env("PRESSURE_VESSEL_FILESYSTEMS_RW", format!("{}/wivrn/comp_ipc", xdg_runtime_dir));
        // /run/user/1000/wivrn/comp_ipc

        Ok(())
    }
}