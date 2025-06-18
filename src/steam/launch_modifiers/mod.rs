pub mod wivrn;
pub mod steam;
pub mod env_vars;

use tokio::process;
use crate::steam::steam_interface::{ProtonVersion, SteamApp};

pub trait LaunchModifier {
    fn apply(&self, command: &mut process::Command, app: &SteamApp, compat_version: &ProtonVersion) -> anyhow::Result<()>;
}