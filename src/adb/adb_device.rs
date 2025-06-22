use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use num_enum::TryFromPrimitive;
use udev::Device;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AdbVrDevice {
    pub is_usb_connected: Arc<AtomicBool>,
    pub dev_type: VrDeviceType,
    pub product_id: u16,
    pub manufacturer: String,
    pub product_name: String,
    pub usb_serial: String,
    pub dev_path: String,
    pub ip_address: Option<String>,
}

impl TryFrom<&Device> for AdbVrDevice {
    type Error = Box<dyn std::error::Error>;

    fn try_from(device: &Device) -> Result<Self, Self::Error> {
        let vendor_id = device
            .attribute_value("idVendor")
            .and_then(|value| value.to_str())
            .and_then(|value| u16::from_str_radix(value, 16).ok())
            .and_then(|id| VrDeviceType::try_from(id).ok());
        let product_id = device
            .attribute_value("idProduct")
            .and_then(|value| value.to_str())
            .and_then(|value| u16::from_str_radix(value, 16).ok());
        let device_serial = device
            .attribute_value("serial")
            .and_then(|value| value.to_str());
        let device_manufacturer = device
            .attribute_value("manufacturer")
            .and_then(|value| value.to_str());
        let device_name = device
            .attribute_value("product")
            .and_then(|value| value.to_str());
        let dev_path = device.devpath().to_str()
            .ok_or(anyhow::anyhow!("Invalid device path"))?;

        if let (
            Some(vendor_id),
            Some(product_id),
            Some(device_serial),
            Some(device_manufacturer),
            Some(device_name),
        ) = (vendor_id, product_id, device_serial, device_manufacturer, device_name)
        {
            let ip_output = String::from_utf8(Command::new("adb")
                .args(&[
                    "-s", &device_serial,
                    "shell", "ip", "addr", "show", "wlan0"
                ])
                .output()?.stdout)?;
            let ip_address = ip_output
                .lines()
                .into_iter()
                .find(|line| line.contains("inet ") && line.contains("scope global"))
                .and_then(|line| line.trim().split(' ').nth(1))
                .and_then(|line| line.split('/').next());

            Ok(AdbVrDevice {
                is_usb_connected: Arc::new(AtomicBool::new(true)),
                dev_type: vendor_id,
                product_id,
                manufacturer: device_manufacturer.into(),
                product_name: device_name.into(),
                usb_serial: device_serial.into(),
                dev_path: dev_path.into(),
                ip_address: ip_address.map(|ip| ip.into()),
            })
        } else {
            Err(anyhow::anyhow!("Unable to parse this device as a valid VR device").into())
        }
    }
}

impl AdbVrDevice {
    pub fn adb_shell_command(&self, command_args: &[&str]) -> anyhow::Result<std::process::Output> {
        let conn_id = self.get_conn_id()?;

        Ok(Command::new("adb")
            .args(&["-s", &conn_id])
            .arg("shell")
            .args(command_args)
            .output()?
        )
    }

    pub fn try_open_tcp_tunnel(&self, port: u32) -> anyhow::Result<()> {
        let conn_id = self.get_conn_id()?;
        Command::new("adb")
            .args(&["-s", &conn_id])
            .arg("reverse")
            .arg(format!("tcp:{}", port))
            .arg(format!("tcp:{}", port))
            .output()?;

        Ok(())
    }

    pub fn is_hmd_mounted(&self) -> anyhow::Result<bool> {
        let result = self.adb_shell_command(&[
            "dumpsys", "power",
        ])?;

        for line in String::from_utf8(result.stdout)?.lines() {
            let line = line.trim();
            if line.is_empty() || !line.contains('=') {
                continue;
            }

            let parts = line.split('=').collect::<Vec<_>>();
            if parts[0] == "mWakefulness" && parts.len() == 2 {
                return Ok(parts[1] == "Awake");
            }
        }

        Ok(false)
    }

    pub(crate) fn try_connect_tcpip(&self, port: u32) -> anyhow::Result<()> {
        if let Some(ip) = self.ip_address.as_ref() {
            Command::new("adb")
                .arg("connect")
                .arg(format!("{}:{}", ip, port))
                .output()?;
        }

        Ok(())
    }

    fn get_conn_id(&self) -> anyhow::Result<String> {
        match (self.is_usb_connected.load(Ordering::SeqCst), self.ip_address.as_ref()) {
            (true, _) => Ok(self.usb_serial.clone()),
            (false, Some(ip)) => {
                if let Ok(_) = self.try_connect_tcpip(5555) {
                    Ok(format!("{}:{}", ip, 5555))
                } else {
                    Err(anyhow::anyhow!("Unable to connect to VR device via LAN").into())
                }
            },
            _ => Err(anyhow::anyhow!("Unable to connect to VR device").into()),
        }
    }
}

#[derive(Debug, Clone, TryFromPrimitive)]
#[repr(u16)]
pub enum VrDeviceType {
    Sony = 0x054c,
    HTC = 0x0bb4,
    Lenovo = 0x17ef,
    Microsoft = 0x045e,
    Oculus = 0x2833,
    Valve = 0x28de,
}