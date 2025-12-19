use crate::adb::device_manager::DeviceManager;
use crate::audio_api::AudioDevice;
use crate::backends::envision::config::{EnvisionConfiguration, EnvisionUserProfile};
use crate::backends::envision::envision_launch_modifier::EnvisionLaunchModifier;
use crate::backends::{BackendStartInfo, VRBackend};
use crate::logging::log_channel::LogChannel;
use crate::steam::launch_modifiers::LaunchModifier;
use crate::TokioMutex;
use anyhow::Context;
use async_trait::async_trait;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use crate::backends::wivrn::wivrn_backend::WiVRnBackend;

pub struct EnvisionBackend {
    wivrn_backend: WiVRnBackend,
    envision_profile: EnvisionUserProfile,
}

#[async_trait]
impl VRBackend for EnvisionBackend {
    async fn start_async(&mut self, backend_log_channel: Arc<Mutex<LogChannel>>, device_manager: Arc<TokioMutex<DeviceManager>>) -> anyhow::Result<BackendStartInfo> {
        self.wivrn_backend.start_async(backend_log_channel, device_manager).await
    }

    async fn reconnect_async(&mut self, device_manager: Arc<TokioMutex<DeviceManager>>) -> anyhow::Result<()> {
        self.wivrn_backend.reconnect_async(device_manager).await
    }

    async fn is_ready(&self) -> anyhow::Result<bool> {
        self.wivrn_backend.is_ready().await
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.wivrn_backend.stop()
    }

    fn is_matching_audio_device(&self, device: &AudioDevice) -> bool {
        self.wivrn_backend.is_matching_audio_device(device)
    }

    fn add_modifiers(&self, list: &mut Vec<Box<dyn LaunchModifier>>) -> anyhow::Result<()> {
        list.insert(0, Box::new(EnvisionLaunchModifier::new(self.envision_profile.clone())));

        Ok(())
    }
}

impl EnvisionBackend {
    pub fn new(args: String) -> anyhow::Result<EnvisionBackend> {
        if args.is_empty() {
            anyhow::bail!("Envision requires a profile UUID to be specified as an argument");
        }

        let uuid = match Uuid::parse_str(&args) {
            Ok(uuid) => uuid,
            Err(_) => anyhow::bail!("Invalid Envision profile UUID was specified"),
        };

        let envision_config_path = env::home_dir().unwrap()
            .join(".config/envision/envision.json");
        let envision_config_str = std::fs::read_to_string(envision_config_path)
            .context("Failed to read Envision configuration file")?;
        let envision_config = serde_json::from_str::<EnvisionConfiguration>(&envision_config_str)
            .context("Failed to parse Envision configuration file")?;

        let envision_profile = envision_config.user_profiles.into_iter()
            .find(|p| p.uuid == uuid.to_string())
            .ok_or_else(|| anyhow::anyhow!("Failed to find Envision profile with the specified UUID"))?;

        let wivrn_server_path = PathBuf::from_str(&envision_profile.prefix)?
            .join("bin/wivrn-server");

        if !wivrn_server_path.exists() {
            anyhow::bail!("No WiVRn server was found in the selected Envision prefix folder");
        }

        Ok(EnvisionBackend {
            wivrn_backend: WiVRnBackend::new(wivrn_server_path.into())?,
            envision_profile,
        })
    }
}