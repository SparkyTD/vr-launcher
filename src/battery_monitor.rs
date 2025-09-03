use crate::adb::device_manager::DeviceManager;
use futures_util::FutureExt;
use num_enum::FromPrimitive;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use ts_rs::TS;

const CHARGE_HISTORY_SAMPLES: usize = 128;
const BATTERY_SCAN_INTERVAL_SEC: u64 = 60;

#[allow(dead_code)]
pub struct BatteryMonitor {
    active_device_serial: Arc<Mutex<Option<String>>>,
    active_device_ip: Arc<Mutex<Option<String>>>,
    is_active: Arc<AtomicBool>,
    monitor_thread: JoinHandle<()>,
    current_info: Arc<Mutex<Option<AndroidBatteryInfo>>>,
    charge_history: Arc<Mutex<Vec<u8>>>,
}

impl BatteryMonitor {
    pub fn new(ws_tx: Sender<String>, device_manager: Arc<Mutex<DeviceManager>>, stop_ch: Sender<()>) -> Self {
        let mut stop_rx = stop_ch.subscribe();
        let is_active = Arc::new(AtomicBool::new(true));
        let active_serial = Arc::new(Mutex::new(None));
        let active_device_ip = Arc::new(Mutex::new(None));

        let current_info = Arc::new(Mutex::new(None));
        let previous_percentage = Arc::new(Mutex::new(vec![]));

        Self {
            active_device_serial: active_serial.clone(),
            active_device_ip: active_device_ip.clone(),
            current_info: current_info.clone(),
            charge_history: previous_percentage.clone(),
            is_active: is_active.clone(),
            monitor_thread: tokio::spawn(async move {
                let mut force_update_rx = {
                    let device_manager = device_manager.lock().await;
                    let force_update_rx = device_manager.subscribe_to_force_battery_update();
                    drop(device_manager);
                    force_update_rx
                };
                
                loop {
                    if !is_active.load(Ordering::SeqCst) {
                        break;
                    }

                    //println!("Will try to query battery levels");

                    let device_manager = device_manager.lock().await;
                    if let Ok(Some(current_device)) = device_manager.get_current_device_async().await {
                        if let Ok(battery_output) = current_device.adb_shell_command(&["dumpsys", "battery"]) {
                            let dumpsys = String::from_utf8_lossy(&battery_output.stdout).to_string();
                            let power_info = AndroidBatteryStats::try_parse(&dumpsys).unwrap();

                            let mut percentage_history = previous_percentage.lock().await;
                            if percentage_history.len() > CHARGE_HISTORY_SAMPLES {
                                percentage_history.remove(0);
                            }
                            percentage_history.push(power_info.level);
                            //println!("Level: {}", power_info.level);

                            let battery_info = AndroidBatteryInfo {
                                stats: power_info,
                                history: percentage_history.clone(),
                            };

                            _ = ws_tx.send(format!("battery:{}", serde_json::to_string(&battery_info).unwrap()));

                            *current_info.lock().await = Some(battery_info);
                        } else {
                            eprintln!("Failed to get battery info");
                        }
                    }
                    drop(device_manager);

                    tokio::select! {
                        _ = stop_rx.recv() => {
                            println!("Battery monitor has received an interrupt signal");
                            break;
                        }
                        _ = force_update_rx.recv() => {
                            println!("Battery monitor received a force update signal");
                            continue;
                        }
                        _ = tokio::time::sleep(Duration::from_secs(BATTERY_SCAN_INTERVAL_SEC)) => {}
                    }
                }

                println!("  >> [BAT_MON] Task exiting");
            }),
        }
    }

    pub fn set_active_device_serial(&mut self, serial: String) {
        _ = self.active_device_serial.lock()
            .map(|mut s| s.replace(serial));
    }

    pub fn set_active_device_ip(&mut self, ip: String) {
        _ = self.active_device_ip.lock()
            .map(|mut i| i.replace(ip));
    }

    pub async fn get_battery_info_async(&self) -> Option<AndroidBatteryInfo> {
        let info = self.current_info.lock().await;

        info.clone()
    }
}

#[derive(Debug, Serialize, Clone, TS)]
#[ts(export, export_to = "rust_bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct AndroidBatteryInfo {
    stats: AndroidBatteryStats,
    history: Vec<u8>,
}

#[derive(Debug, Serialize, Clone, TS)]
#[ts(export, export_to = "rust_bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct AndroidBatteryStats {
    power_source: BatteryChargeSource,
    is_weak_charger: bool,
    max_charge_current_ma: u32,
    max_charge_voltage_mv: u32,
    charge_counter: u32,
    status: BatteryStatus,
    health: BatteryHealth,
    present: bool,
    level: u8,
    scale: u8,
    voltage: u32,
    temperature: u32,
    technology: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, FromPrimitive, TS)]
#[repr(u16)]
pub enum BatteryHealth {
    #[default]
    Unknown = 1,
    Good = 2,
    Overheat = 3,
    Dead = 4,
    OverVoltage = 5,
    UnspecifiedFailure = 6,
    Cold = 7,
}

#[derive(Debug, Clone, PartialEq, Serialize, FromPrimitive, TS)]
#[repr(u16)]
pub enum BatteryStatus {
    #[default]
    Unknown = 1,
    Charging = 2,
    Discharging = 3,
    NotCharging = 4,
    Full = 5,
}

#[derive(Debug, Clone, Serialize, TS)]
pub enum BatteryChargeSource {
    Battery,
    AC,
    USB,
    Dock,
    Wireless,
}

impl AndroidBatteryStats {
    pub fn try_parse(dumpsys_output: &str) -> anyhow::Result<AndroidBatteryStats> {
        let mut battery_info = AndroidBatteryStats {
            power_source: BatteryChargeSource::Battery,
            is_weak_charger: false,
            max_charge_current_ma: 0,
            max_charge_voltage_mv: 0,
            charge_counter: 0,
            status: BatteryStatus::Unknown,
            health: BatteryHealth::Unknown,
            present: false,
            level: 0,
            scale: 0,
            voltage: 0,
            temperature: 0,
            technology: String::new(),
        };

        let mut source_ac = false;
        let mut source_usb = false;
        let mut source_wireless = false;
        let mut source_dock = false;

        for line in dumpsys_output.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key.to_lowercase().as_str() {
                    "ac powered" => source_ac = Self::parse_bool(value)?,
                    "usb powered" => source_usb = Self::parse_bool(value)?,
                    "wireless powered" => source_wireless = Self::parse_bool(value)?,
                    "dock powered" => source_dock = Self::parse_bool(value)?,
                    "weak charger" => battery_info.is_weak_charger = Self::parse_bool(value)?,
                    "max charging current" => battery_info.max_charge_current_ma = value.parse::<u32>()? / 1000,
                    "max charging voltage" => battery_info.max_charge_voltage_mv = value.parse::<u32>()? / 1000,
                    "charge counter" => battery_info.charge_counter = value.parse::<u32>()?,
                    "status" => battery_info.status = BatteryStatus::from(value.parse::<u16>()?),
                    "health" => battery_info.health = BatteryHealth::from(value.parse::<u16>()?),
                    "present" => battery_info.present = Self::parse_bool(value)?,
                    "level" => battery_info.level = value.parse::<u8>()?,
                    "scale" => battery_info.scale = value.parse::<u8>()?,
                    "voltage" => battery_info.voltage = value.parse::<u32>()?,
                    "temperature" => battery_info.temperature = value.parse::<u32>()?,
                    "technology" => battery_info.technology = value.to_string(),
                    _ => {} // Ignore unknown fields
                }
            }
        }

        match (source_ac, source_usb, source_wireless, source_dock) {
            (true, _, _, _) => battery_info.power_source = BatteryChargeSource::AC,
            (_, true, _, _) => battery_info.power_source = BatteryChargeSource::USB,
            (_, _, true, _) => battery_info.power_source = BatteryChargeSource::Wireless,
            (_, _, _, true) => battery_info.power_source = BatteryChargeSource::Dock,
            _ => {}
        }

        Ok(battery_info)
    }

    fn parse_bool(s: &str) -> anyhow::Result<bool> {
        match s.to_lowercase().as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(anyhow::anyhow!("Failed to parse bool: {:?}", s)),
        }
    }
}