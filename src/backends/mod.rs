use std::sync::{Arc, Mutex};
use crate::logging::log_channel::LogChannel;

pub mod wivrn;

pub trait VRBackend {
    fn start(&mut self, backend_log_channel: Arc<Mutex<LogChannel>>) -> anyhow::Result<BackendStartInfo>;
    fn reconnect(&mut self) -> anyhow::Result<()>;
    fn stop(&mut self) -> anyhow::Result<()>;
    fn is_hmd_mounted(&self) -> anyhow::Result<bool>;
}

pub struct BackendStartInfo {
    pub vr_device_serial: String,
    pub was_restarted: bool,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum BackendType {
    Unknown,
    WiVRn,
    ALVR,
}