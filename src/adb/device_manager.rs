use crate::adb::adb_device::AdbVrDevice;
use crate::TokioMutex;
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use std::os::fd::{AsRawFd, BorrowedFd};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::task::JoinHandle;
use udev::{Enumerator, MonitorBuilder};

pub struct DeviceManager {
    current_device: Arc<TokioMutex<Option<AdbVrDevice>>>,
    _monitor_thread: JoinHandle<()>,
}

impl DeviceManager {
    pub fn new() -> anyhow::Result<Self> {
        let current_device = Arc::new(TokioMutex::new(Self::find_connected_device()?));
        Ok(Self {
            current_device: current_device.clone(),
            _monitor_thread: tokio::task::spawn(async move {
                let socket = MonitorBuilder::new().unwrap()
                    .match_subsystem_devtype("usb", "usb_device").unwrap()
                    .listen().unwrap();

                let fd = socket.as_raw_fd();
                loop {
                    let borrowed_fd = unsafe { BorrowedFd::borrow_raw(fd) };
                    let mut poll_fds = [PollFd::new(borrowed_fd, PollFlags::POLLIN)];

                    match poll(&mut poll_fds, PollTimeout::try_from(-1).unwrap()) {
                        Ok(n) if n > 0 => {
                            if let Some(event) = socket.iter().next() {
                                let action = event.action()
                                    .and_then(|str| str.to_str());
                                let dev_path = event.devpath();

                                let mut current_device = current_device.lock().await;

                                match action {
                                    Some("bind") => {
                                        if let Ok(device) = AdbVrDevice::try_from(&event.device()) {
                                            println!("  VR Device Connected: {:?}", device);
                                            current_device.replace(device);
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
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Ok(_) => {
                            continue;
                        }
                        Err(e) => {
                            eprintln!("Poll error: {}", e);
                            break;
                        }
                    }
                }
            }),
        })
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

