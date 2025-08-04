use crate::steam::vfd_format::AppInfoDatabase;
use std::collections::HashSet;
use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SteamApp {
    pub steam_id: u32,
    pub is_vr_app: bool,
    pub title: String,
    pub app_folder: PathBuf,
    pub executable: String,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ProtonVersion {
    pub steam_id: Option<u32>,
    pub name: String,
    pub executable_path: PathBuf,
}

pub struct SteamInterface {}

impl SteamInterface {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_installed_apps(&self) -> anyhow::Result<Vec<SteamApp>> {
        let steam_dir = steamlocate::SteamDir::locate()?;

        let mut found_apps = Vec::new();
        let mut seen_steam_ids = HashSet::new();

        let app_info = AppInfoDatabase::load_from(steam_dir.path().join("appcache/appinfo.vdf").to_path_buf())?;
        for library in steam_dir.libraries()?.into_iter().filter_map(|l| l.ok()) {
            //println!("Library at {:?}:", library.path());

            for app in library.apps().into_iter().filter_map(|l| l.ok()) {
                if seen_steam_ids.contains(&app.app_id) {
                    continue;
                }

                seen_steam_ids.insert(app.app_id);

                let app_entry = app_info.app_by_id(app.app_id).unwrap();
                let launch_options = app_entry.data["appinfo"]["config.launch"].as_object();
                let is_vr = app_entry.data["appinfo"]["common.openvrsupport"].parse_i32_and(|i| i == 1);

                if launch_options.is_none() {
                    continue;
                }

                let launch_options = launch_options.unwrap().values().collect::<Vec<_>>();
                let launch_config = launch_options
                    .iter()
                    .find(|l| l["config.oslist"]
                        .is_string_and(|s| s.contains("windows"))
                        || l["config.oslist"].is_none());

                if launch_config.is_none() {
                    continue;
                }

                let launch_config = launch_config.unwrap();
                let working_dir = launch_config["workingdir"].as_string();
                let executable = launch_config["executable"].as_string();
                let arguments = launch_config["arguments"].as_string();
                let arguments = match arguments {
                    None => vec![],
                    Some(args) => args.split(' ').into_iter().map(|s| s.to_string()).collect(),
                };

                if executable.is_none() {
                    continue;
                }

                let executable = executable.unwrap().replace("\\", "/");
                let working_dir = working_dir.map(|wd| wd.replace("\\", "/"));
                let app_install_dir = library.path().join("steamapps/common").join(app.install_dir);
                let app_exe_path = app_install_dir.join(&executable);
                let working_dir = working_dir.map(|wd| app_install_dir.join(wd)).unwrap_or(app_install_dir.clone());

                if !app_exe_path.exists() {
                    continue;
                }

                //println!("   - App {}: {:?}", app.app_id, app.name.clone().unwrap());

                found_apps.push(SteamApp {
                    steam_id: app.app_id,
                    is_vr_app: is_vr,
                    title: app.name.unwrap(),
                    executable: executable.clone(),
                    arguments,
                    app_folder: app_install_dir,
                    working_directory: working_dir,
                })
            }
        }

        Ok(found_apps)
    }

    pub fn get_proton_versions(&self) -> anyhow::Result<Vec<ProtonVersion>> {
        let steam_dir = steamlocate::SteamDir::locate()?;

        let mut seen_steam_ids = HashSet::new();
        let mut versions = Vec::new();

        // Find first-party installations
        for library in steam_dir.libraries()?.into_iter().filter_map(|l| l.ok()) {
            for app in library.apps().into_iter().filter_map(|l| l.ok()) {
                if seen_steam_ids.contains(&app.app_id) {
                    continue;
                }

                seen_steam_ids.insert(app.app_id);

                let app_install_dir = library.path().join("steamapps/common").join(&app.install_dir);
                let proton_binary = app_install_dir.join("proton");

                if !proton_binary.exists() {
                    continue;
                }

                versions.push(ProtonVersion {
                    steam_id: Some(app.app_id),
                    name: app.name.unwrap_or(app.install_dir),
                    executable_path: proton_binary,
                });
            }
        }

        // Find external installations
        let compat_tools_path = steam_dir.path().join("compatibilitytools.d");
        for entry in std::fs::read_dir(&compat_tools_path)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            let proton_binary = path.join("proton");

            if !proton_binary.exists() {
                continue;
            }

            versions.push(ProtonVersion {
                steam_id: None,
                name: file_name.into_string().unwrap(),
                executable_path: proton_binary.clone(),
            });
        }

        Ok(versions)
    }
}