use crate::steam::launch_modifiers::LaunchModifier;
use crate::steam::steam_interface::{ProtonVersion, SteamApp};
use anyhow::ensure;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use tokio::process;
use vdf_reader::entry::Table;

pub struct SteamLaunchModifier;

impl SteamLaunchModifier {
    pub fn new() -> Self {
        Self {}
    }
}

const STEAMAPPS: &str = "steamapps";
const COMPATDATA: &str = "steamapps/compatdata";
const COMMON: &str = "steamapps/common";
const SHADERCACHE: &str = "steamapps/shadercache";
const LOGINUSERS: &str = "config/loginusers.vdf";

impl LaunchModifier for SteamLaunchModifier {
    fn apply(&self, command: &mut process::Command, app: &SteamApp, compat_version: Option<&ProtonVersion>) -> anyhow::Result<()> {
        // Steam IDs
        let assigned_id = match app.steam_id {
            0 => generate_20_digit_code(&app.executable),
            _ => app.steam_id.to_string()
        };
        command.env("SteamAppId", app.steam_id.to_string());
        command.env("SteamGameId", &assigned_id);
        command.env("SteamOverlayGameId", &assigned_id);

        // Steam Basic
        command.env("SteamUser", get_user_name()?);
        command.env("SteamEnv", "1");

        // Steam Compat
        let steam_home = steamlocate::SteamDir::locate()?;
        command.env("STEAM_COMPAT_APP_ID", app.steam_id.to_string());
        command.env("STEAM_COMPAT_CLIENT_INSTALL_PATH", steam_home.path());
        command.env("STEAM_COMPAT_DATA_PATH", steam_home.path().join(COMPATDATA).join(assigned_id.to_string()));
        command.env("STEAM_COMPAT_FLAGS", "search-cwd"); // only present on Steam games
        command.env("STEAM_COMPAT_LIBRARY_PATHS", steam_home.path().join(STEAMAPPS));
        command.env("STEAM_COMPAT_INSTALL_PATH", &app.app_folder);
        command.env("PWD", &app.app_folder);
        command.current_dir(&app.app_folder);

        // Fossilize, Shaders and Drivers
        command.env("AMD_VK_PIPELINE_CACHE_FILENAME", "steamapp_shader_cache");
        command.env("STEAM_BASE_FOLDER", steam_home.path());
        command.env("STEAM_CLIENT_CONFIG_FILE", steam_home.path().join("steam.cfg"));
        command.env("AMD_VK_PIPELINE_CACHE_PATH", steam_home.path().join(SHADERCACHE).join(assigned_id.to_string()).join("AMDv1"));
        command.env("AMD_VK_USE_PIPELINE_CACHE", "1");
        command.env("DXVK_STATE_CACHE_PATH", steam_home.path().join(SHADERCACHE).join(assigned_id.to_string()).join("DXVK_state_cache"));
        command.env("FOSSILIZE_APPLICATION_INFO_FILTER_PATH", steam_home.path().join("fossilize_engine_filters.json"));
        command.env("SDL_GAMECONTROLLER_ALLOW_STEAM_VIRTUAL_GAMEPAD", "1");
        command.env("SDL_JOYSTICK_HIDAPI_STEAMXBOX", "0");
        command.env("SDL_VIDEO_X11_DGAMOUSE", "0");
        command.env("DISABLE_LAYER_AMD_SWITCHABLE_GRAPHICS_1", "1");
        command.env("ENABLE_VK_LAYER_VALVE_steam_fossilize_1", "1");
        command.env("ENABLE_VK_LAYER_VALVE_steam_overlay_1", "1");
        command.env("MESA_DISK_CACHE_SINGLE_FILE", "1");
        command.env("MESA_GLSL_CACHE_MAX_SIZE", "5G");
        command.env("MESA_SHADER_CACHE_MAX_SIZE", "5G");
        command.env("__GL_SHADER_DISK_CACHE_SKIP_CLEANUP", "1");
        command.env("__GL_SHADER_DISK_CACHE_APP_NAME", "steamapp_shader_cache");
        command.env("__GL_SHADER_DISK_CACHE_READ_ONLY_APP_NAME", "steam_shader_cache;steamapp_merged_shader_cache");
        command.env("__GL_SHADER_DISK_CACHE_PATH", steam_home.path().join(SHADERCACHE).join(assigned_id.to_string()).join("fozmediav1"));
        command.env("STEAM_COMPAT_MEDIA_PATH", steam_home.path().join(SHADERCACHE).join(assigned_id.to_string()).join("fozmediav1"));
        command.env("STEAM_FOSSILIZE_DUMP_PATH", steam_home.path().join(SHADERCACHE).join(assigned_id.to_string()).join("fozpipelinesv6/steamapprun_pipeline_cache"));
        command.env("STEAM_COMPAT_SHADER_PATH", steam_home.path().join(SHADERCACHE).join(assigned_id.to_string()));
        command.env("MESA_GLSL_CACHE_DIR", steam_home.path().join(SHADERCACHE).join(assigned_id.to_string()));
        command.env("MESA_SHADER_CACHE_DIR", steam_home.path().join(SHADERCACHE).join(assigned_id.to_string()));
        command.env("STEAM_COMPAT_TRANSCODED_MEDIA_PATH", steam_home.path().join(SHADERCACHE).join(assigned_id.to_string()));
        command.env("STEAM_COMPAT_MOUNTS", vec![
            steam_home.path().join(COMMON).join("SteamLinuxRuntime_sniper").to_str().unwrap(),
            //     steam_home.path().join(COMMON).join("Steamworks Shared").to_str().unwrap(), // TODO: Only for Steam games ???
        ].join(":"));
        command.env("STEAM_COMPAT_PROTON", "1");

        if let Some(compat_version) = compat_version {
            command.env("STEAM_COMPAT_TOOL_PATHS", vec![
                compat_version.executable_path.parent().unwrap().to_str().unwrap(),
                steam_home.path().join(COMMON).join("SteamLinuxRuntime_sniper").to_str().unwrap(),
            ].join(":"));
        }

        command.env("STEAM_FOSSILIZE_DUMP_PATH_READ_ONLY", "$bucketdir/steam_pipeline_cache.foz;$bucketdir/steamapp_pipeline_cache.foz");
        //command.env("STEAM_RUNTIME_LIBRARY_PATH", todo!("List of Steam's bin library folders"));
        command.env("WINEDLLOVERRIDES", "winhttp=n,b"); // only BSManager does this

        fs::create_dir_all(steam_home.path().join(COMPATDATA).join(assigned_id.to_string()))?;
        fs::create_dir_all(steam_home.path().join(SHADERCACHE).join(assigned_id.to_string()).join("fozmediav1"))?;
        fs::create_dir_all(steam_home.path().join(SHADERCACHE).join(assigned_id.to_string()).join("fozpipelinesv6"))?;
        fs::create_dir_all(steam_home.path().join(SHADERCACHE).join(assigned_id.to_string()).join("DXVK_state_cache"))?;
        fs::create_dir_all(steam_home.path().join(SHADERCACHE).join(assigned_id.to_string()).join("AMDv1"))?;

        Ok(())
    }
}

fn generate_20_digit_code(seed: &str) -> String {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    let hash = hasher.finish();
    let hash_str = format!("{:020}", hash);
    hash_str.chars().take(20).collect()
}

pub fn get_user_name() -> anyhow::Result<String> {
    let users_file = steamlocate::SteamDir::locate()?.path().join(LOGINUSERS);
    ensure!(users_file.exists(), format!("The user database file doesn't exist: {}", users_file.display()));

    let database_text = std::fs::read_to_string(users_file)?;
    let data: Table = vdf_reader::from_str(&database_text)?;
    let user_list = data["users"].as_table().unwrap().values().collect::<Vec<_>>();
    let most_recent = user_list.iter()
        .find(|u| u.as_table().unwrap()["MostRecent"].as_str().is_some_and(|s| s == "1"))
        .unwrap_or_else(|| user_list.first().unwrap())
        .as_table().ok_or(anyhow::anyhow!("No users found"))?;

    let username = most_recent["AccountName"].as_str()
        .ok_or(anyhow::anyhow!("No AccountName found"))?;

    Ok(username.into())
}