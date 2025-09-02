#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct WifiInfo {
    pub ssid: String,
    pub bssid: String,
    pub mac: String,
    pub ip: String,
    pub standard: String,
    pub rssi: i16,
    pub link_speed: u64,
    pub rx_link_speed: u64,
    pub max_rx_link_speed: u64,
    pub tx_link_speed: u64,
    pub max_tx_link_speed: u64,
    pub frequency_mhz: u64,
}

impl WifiInfo {
    pub fn parse_from(str: &str) -> Option<WifiInfo> {
        for line in str.split("\n") {
            let mut info = WifiInfo::default();

            for prop in line.replace("mWifiInfo ", "").split(", ") {
                let parts = prop.split(": ").collect::<Vec<&str>>();
                let key = parts[0].to_lowercase();
                let value = parts[1];

                match key.as_str() {
                    "ssid" => info.ssid = value.trim_matches('"').to_string(),
                    "bssid" => info.bssid = value.to_string(),
                    "mac" => info.mac = value.to_string(),
                    "ip" => info.ip = value.to_string(),
                    "wi-fi standard" => info.standard = value.to_string(),
                    "rssi" => info.rssi = value.parse::<i16>().unwrap(),
                    "link speed" => info.link_speed = Self::parse_speed(value)?,
                    "tx Link speed" => info.tx_link_speed = Self::parse_speed(value)?,
                    "max supported tx link speed" => info.max_tx_link_speed = Self::parse_speed(value)?,
                    "rx Link speed" => info.rx_link_speed = Self::parse_speed(value)?,
                    "max supported rx link speed" => info.max_rx_link_speed = Self::parse_speed(value)?,
                    "frequency" => info.frequency_mhz = value.replace("MHz", "").parse::<u64>().unwrap(),
                    "isprimary" if value == "0" => return None,
                    _ => continue,
                }
            }

            return Some(info);
        }

        None
    }

    fn parse_speed(speed: &str) -> Option<u64> {
        let mut number = String::new();
        let mut unit = String::new();
        let mut num_read = false;
        for i in 0..speed.len() {
            match speed.chars().nth(i).unwrap() {
                ch if (ch.is_numeric() || ch == '-') && !num_read => number.push(ch),
                ch if ch.is_alphabetic() => {
                    num_read = true;
                    unit.push(ch)
                }
                _ => return None,
            }
        }

        let number = number.parse::<i64>().unwrap();
        if number < 0 {
            return None;
        }

        let mut number = number as u64;
        if unit == "Mbps" {
            number *= 1_000_000;
        }

        if unit == "Kbps" {
            number *= 1_000;
        }

        Some(number)
    }
}