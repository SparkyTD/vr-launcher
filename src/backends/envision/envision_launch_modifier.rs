use std::{env, fs};
use crate::backends::envision::config::EnvisionUserProfile;
use crate::steam::launch_modifiers::LaunchModifier;
use crate::steam::steam_interface::{ProtonVersion, SteamApp};
use std::path::PathBuf;
use std::str::FromStr;
use tokio::process::Command;
use crate::backends::wivrn::wivrn_launch_modifier::WiVRnLaunchModifier;

pub struct EnvisionLaunchModifier {
    envision_profile: EnvisionUserProfile,
}

impl EnvisionLaunchModifier {
    pub fn new(envision_profile: EnvisionUserProfile) -> Self {
        Self {
            envision_profile,
        }
    }
}

impl LaunchModifier for EnvisionLaunchModifier {
    fn apply(&self, command: &mut Command, app: &SteamApp, _compat_version: Option<&ProtonVersion>) -> anyhow::Result<()> {
        // let ovr_comp_path = PathBuf::from_str(&self.envision_profile.ovr_comp.path)?;
        // let ovr_comp_root = WalkDir::new(ovr_comp_path)
        //     .into_iter()
        //     .filter_map(|e| e.ok())
        //     .filter(|e| e.file_type().is_file())
        //     .find(|e| e.file_name() == "openvrpaths.vrpath")
        //     .map(|e| e.into_path())
        //     .and_then(|p| p.parent().map(|p| p.to_owned()))
        //     .ok_or_else(|| anyhow::anyhow!("Failed to find OVR runtime root directory"))?;

        let openxr_config_path = PathBuf::from_str(&self.envision_profile.prefix)?
            .join("share/openxr/1/openxr_wivrn.json");

        let openxr_target_path = env::home_dir().unwrap()
            .join(".config/openxr/1/active_runtime.json");

        let wivrn_launch_modifier = WiVRnLaunchModifier::new(openxr_config_path.clone());
        wivrn_launch_modifier.apply_env_vars(command, app)?;

        fs::create_dir_all(&openxr_target_path.parent().unwrap())?;

        if openxr_target_path.exists() {
            fs::remove_file(&openxr_target_path)?;
        }
        std::os::unix::fs::symlink(&openxr_config_path, &openxr_target_path)?;

        // command.env("VR_OVERRIDE", &ovr_comp_root);
        // command.env("XR_RUNTIME_JSON", &openxr_target_path);

        Ok(())
    }
}