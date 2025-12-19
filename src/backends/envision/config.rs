use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvisionConfiguration {
    #[serde(rename = "selected_profile_uuid")]
    pub selected_profile_uuid: String,
    #[serde(rename = "user_profiles")]
    pub user_profiles: Vec<EnvisionUserProfile>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvisionUserProfile {
    pub uuid: String,
    pub name: String,
    #[serde(rename = "xrservice_type")]
    pub xrservice_type: String,
    #[serde(rename = "xrservice_path")]
    pub xrservice_path: String,
    #[serde(rename = "xrservice_repo")]
    pub xrservice_repo: Option<String>,
    #[serde(rename = "xrservice_branch")]
    pub xrservice_branch: Option<String>,
    #[serde(rename = "opencomposite_path")]
    pub opencomposite_path: String,
    #[serde(rename = "opencomposite_repo")]
    pub opencomposite_repo: Value,
    #[serde(rename = "opencomposite_branch")]
    pub opencomposite_branch: Value,
    #[serde(rename = "ovr_comp")]
    pub ovr_comp: OvrComp,
    pub environment: Environment,
    pub prefix: String,
    #[serde(rename = "can_be_built")]
    pub can_be_built: bool,
    pub editable: bool,
    #[serde(rename = "pull_on_build")]
    pub pull_on_build: bool,
    #[serde(rename = "lighthouse_driver")]
    pub lighthouse_driver: String,
    #[serde(rename = "xrservice_launch_options")]
    pub xrservice_launch_options: String,
    #[serde(rename = "skip_dependency_check")]
    pub skip_dependency_check: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OvrComp {
    #[serde(rename = "mod_type")]
    pub mod_type: String,
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub path: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    #[serde(rename = "LD_LIBRARY_PATH")]
    pub ld_library_path: String,
    #[serde(rename = "U_PACING_APP_USE_MIN_FRAME_PERIOD")]
    pub u_pacing_app_use_min_frame_period: String,
    #[serde(rename = "XRT_CURATED_GUI")]
    pub xrt_curated_gui: String,
    #[serde(rename = "XRT_DEBUG_GUI")]
    pub xrt_debug_gui: String,
}