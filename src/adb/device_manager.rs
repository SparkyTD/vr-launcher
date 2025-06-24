use crate::adb::adb_device::AdbVrDevice;
use crate::TokioMutex;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};
use udev::{Enumerator, MonitorBuilder};

pub struct DeviceManager {
    current_device: Arc<TokioMutex<Option<AdbVrDevice>>>,
    force_update_tx: Sender<()>,
    _monitor_thread: JoinHandle<()>,
}

impl DeviceManager {
    pub fn new(stop_tx: Sender<()>) -> anyhow::Result<Self> {
        let current_device = Arc::new(TokioMutex::new(Self::find_connected_device()?));
        let mut stop_rx = stop_tx.subscribe();
        let (force_update_tx, _) = tokio::sync::broadcast::channel(1);

        Ok(Self {
            current_device: current_device.clone(),
            force_update_tx: force_update_tx.clone(),
            _monitor_thread: tokio::task::spawn(async move {
                let socket = match MonitorBuilder::new()
                    .and_then(|builder| builder.match_subsystem_devtype("usb", "usb_device"))
                    .and_then(|builder| builder.listen()) {
                    Ok(socket) => socket,
                    Err(e) => {
                        eprintln!("Failed to create USB monitor: {}", e);
                        return;
                    }
                };

                loop {
                    tokio::select! {
                        _ = stop_rx.recv() => {
                            println!("Device monitor has received an interrupt signal");
                            break;
                        }
                        event_result = Self::wait_for_event(&socket) => {
                            match event_result {
                                Ok(Some(event)) => {
                                    let action = event.action().and_then(|str| str.to_str());
                                    let dev_path = event.devpath();
    
                                    let mut current_device = current_device.lock().await;
    
                                    match action {
                                        Some("bind") => {
                                            if let Ok(device) = AdbVrDevice::try_from(&event.device()) {
                                                println!("  VR Device Connected: {:?}", device);
                                                current_device.replace(device);
                                                _ = force_update_tx.send(());
                                            }
                                        }
                                        Some("unbind") => {
                                            let device_path = current_device
                                                .as_ref()
                                                .map(|d| d.dev_path.as_str());
    
                                            match device_path {
                                                Some(disconn_dev_path) if disconn_dev_path == dev_path => {
                                                    if let Some(device) = current_device.as_ref() {
                                                        if device.dev_path == disconn_dev_path {
                                                            println!("  VR Device Disconnected: {:?}", device);
                                                            device.is_usb_connected.store(false, Ordering::SeqCst);
                                                            _ = force_update_tx.send(());
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                Ok(None) => {
                                    continue;
                                }
                                Err(e) => {
                                    eprintln!("Error waiting for USB event: {}", e);
                                    sleep(Duration::from_millis(100)).await;
                                }
                            }
                        }
                    }
                }
            }),
        })
    }

    pub fn subscribe_to_force_battery_update(&self) -> broadcast::Receiver<()> {
        self.force_update_tx.subscribe()
    }
    
    async fn wait_for_event(socket: &udev::MonitorSocket) -> Result<Option<udev::Event>, Box<dyn std::error::Error + Send + Sync>> {
        use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
        use tokio::io::unix::AsyncFd;
        use tokio::io::Interest;

        // Create a duplicated fd for this specific wait operation
        let fd = socket.as_raw_fd();
        let duplicated_fd = unsafe { libc::dup(fd) };
        if duplicated_fd == -1 {
            return Err("Failed to duplicate file descriptor".into());
        }

        let owned_fd = unsafe { OwnedFd::from_raw_fd(duplicated_fd) };
        let async_fd = AsyncFd::with_interest(owned_fd, Interest::READABLE)?;

        let mut guard = async_fd.readable().await?;
        let event = socket.iter().next();
        guard.clear_ready();

        Ok(event)
    }

    pub async fn get_current_device_async(&self) -> anyhow::Result<Option<AdbVrDevice>> {
        let current_device = self.current_device.lock().await;
        Ok(current_device.clone())
    }

    fn find_connected_device() -> anyhow::Result<Option<AdbVrDevice>> {
        let mut enumerator = Enumerator::new()?;
        enumerator.match_subsystem("usb")?;

        for device in enumerator.scan_devices()? {
            if let Ok(vr_device) = AdbVrDevice::try_from(&device) {
                return Ok(Some(vr_device));
            }
        }

        Ok(None)
    }
}