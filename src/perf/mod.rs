use serde::Deserialize;

#[allow(unused)]
#[derive(Debug, Deserialize)]
#[serde(rename = "nvidia_smi_log")]
pub struct NvidiaSmiLog {
    pub timestamp: String,
    pub driver_version: String,
    pub cuda_version: String,
    #[serde(rename = "gpu")]
    pub gpus: Vec<Gpu>,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct Gpu {
    #[serde(rename = "@id")]
    pub id: String,
    pub product_name: String,
    pub product_brand: String,
    pub product_architecture: String,
    pub gsp_firmware_version: String,
    pub fan_speed: String,
    pub performance_state: String,
    pub fb_memory_usage: MemoryUsage,
    pub bar1_memory_usage: Bar1MemoryUsage,
    pub cc_protected_memory_usage: CcProtectedMemoryUsage,
    pub utilization: Utilization,
    pub encoder_stats: EncoderStats,
    pub fbc_stats: FbcStats,
    pub temperature: Temperature,
    pub supported_gpu_target_temp: SupportedGpuTargetTemp,
    pub gpu_power_readings: PowerReadings,
    pub clocks: Clocks,
    pub max_clocks: MaxClocks,
    pub processes: Processes,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct MemoryUsage {
    pub total: String,
    pub reserved: String,
    pub used: String,
    pub free: String,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct Bar1MemoryUsage {
    pub total: String,
    pub used: String,
    pub free: String,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct CcProtectedMemoryUsage {
    pub total: String,
    pub used: String,
    pub free: String,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct Utilization {
    pub gpu_util: String,
    pub memory_util: String,
    pub encoder_util: String,
    pub decoder_util: String,
    pub jpeg_util: String,
    pub ofa_util: String,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct EncoderStats {
    pub session_count: u32,
    pub average_fps: u32,
    pub average_latency: u32,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct FbcStats {
    pub session_count: u32,
    pub average_fps: u32,
    pub average_latency: u32,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct Temperature {
    pub gpu_temp: String,
    pub gpu_temp_tlimit: String,
    pub gpu_temp_max_threshold: String,
    pub gpu_temp_slow_threshold: String,
    pub gpu_temp_max_gpu_threshold: String,
    pub gpu_target_temperature: String,
    pub memory_temp: String,
    pub gpu_temp_max_mem_threshold: String,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct SupportedGpuTargetTemp {
    pub gpu_target_temp_min: String,
    pub gpu_target_temp_max: String,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct PowerReadings {
    pub power_state: String,
    pub average_power_draw: String,
    pub instant_power_draw: String,
    pub current_power_limit: String,
    pub requested_power_limit: String,
    pub default_power_limit: String,
    pub min_power_limit: String,
    pub max_power_limit: String,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct Clocks {
    pub graphics_clock: String,
    pub sm_clock: String,
    pub mem_clock: String,
    pub video_clock: String,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct MaxClocks {
    pub graphics_clock: String,
    pub sm_clock: String,
    pub mem_clock: String,
    pub video_clock: String,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct Processes {
    #[serde(rename = "process_info", default)]
    pub process_info: Vec<ProcessInfo>,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    #[serde(rename = "type")]
    pub process_type: String,
    pub process_name: String,
    pub used_memory: String,
}