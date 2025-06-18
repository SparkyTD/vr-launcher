use std::collections::HashMap;
use tokio::process::Command;
use crate::steam::launch_modifiers::LaunchModifier;
use crate::steam::steam_interface::{ProtonVersion, SteamApp};

pub struct EnvironmentVariablesModifier {
    vars: HashMap<String, String>,
}

impl EnvironmentVariablesModifier {
    pub fn new(vars: HashMap<String, String>) -> Self {
        Self { vars }
    }
}

impl LaunchModifier for EnvironmentVariablesModifier {
    fn apply(&self, command: &mut Command, _app: &SteamApp, _compat_version: &ProtonVersion) -> anyhow::Result<()> {
        for (key, value) in &self.vars {
            command.env(&key, &value);
        }
        
        Ok(())
    }
}