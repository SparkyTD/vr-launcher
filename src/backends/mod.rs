use std::sync::{Arc, Mutex};
use crate::logging::log_channel::LogChannel;

pub mod wivrn;

pub trait VRBackend {
    fn start(&mut self, backend_log_channel: Arc<Mutex<LogChannel>>) -> anyhow::Result<BackendStartInfo>;
    fn stop(&mut self) -> anyhow::Result<()>;
}

pub struct BackendStartInfo {
    pub vr_device_serial: String,
    pub was_restarted: bool,
}