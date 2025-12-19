use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use crate::adb::device_manager::DeviceManager;
use crate::audio_api::AudioDevice;
use crate::logging::log_channel::LogChannel;
use crate::steam::launch_modifiers::LaunchModifier;
use crate::TokioMutex;

pub mod wivrn;
pub mod envision;

#[async_trait]
pub trait VRBackend: Send {
    async fn start_async(&mut self, backend_log_channel: Arc<Mutex<LogChannel>>, device_manager: Arc<TokioMutex<DeviceManager>>) -> anyhow::Result<BackendStartInfo>;
    async fn reconnect_async(&mut self, device_manager: Arc<TokioMutex<DeviceManager>>) -> anyhow::Result<()>;
    async fn is_ready(&self) -> anyhow::Result<bool>;
    fn stop(&mut self) -> anyhow::Result<()>;
    fn is_matching_audio_device(&self, device: &AudioDevice) -> bool;
    fn add_modifiers(&self, list: &mut Vec<Box<dyn LaunchModifier>>) -> anyhow::Result<()>;
}

#[allow(dead_code)]
pub struct BackendStartInfo {
    pub vr_device_serial: String,
    pub vr_device_ip: Option<String>,
    pub was_restarted: bool,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum BackendType {
    Unknown,
    WiVRn,
    Envision,
    ALVR,
}