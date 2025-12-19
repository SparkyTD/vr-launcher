use crate::adb::device_manager::DeviceManager;
use crate::logging::log_channel::{LogHandler, LogType};
use crate::TokioMutex;
use std::sync::Arc;
use crate::backends::wivrn::wivrn_backend::WiVRnBackend;

pub(super) struct WiVRnLogHandler {
    pub device_manager: Arc<TokioMutex<DeviceManager>>,
}

#[async_trait::async_trait]
impl LogHandler for WiVRnLogHandler {
    fn handle_message(&self, message: String, _log_type: LogType) {
        if message.contains("Exception in network thread: Socket shutdown") {
            let device_manager = self.device_manager.clone();
            tokio::spawn(async move {
                println!("WiVRn server exited unexpectedly, attempting to reconnect in 3 seconds...");
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                WiVRnBackend::reconnect_static_async(device_manager).await
            });
        }
    }
}