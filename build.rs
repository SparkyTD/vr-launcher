use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let frontend_dir = Path::new(&manifest_dir).join("frontend");
    let frontend_src_dir = frontend_dir.join("src");
    let frontend_dist_dir = frontend_dir.join("dist");

    // Tell cargo to rerun if build.rs changes
    println!("cargo:rerun-if-changed=build.rs");

    // Tell cargo to rerun if anything in frontend/src changes
    println!("cargo:rerun-if-changed=frontend/src");

    // Tell cargo to rerun if package.json changes
    println!("cargo:rerun-if-changed=frontend/package.json");
    println!("cargo:rerun-if-changed=frontend/package-lock.json");

    // Check if we need to rebuild
    let should_rebuild = should_rebuild_frontend(&frontend_src_dir, &frontend_dist_dir);

    if should_rebuild {
        println!("cargo:warning=Frontend source files changed, rebuilding...");
        build_frontend(&frontend_dir);
    }

    // Generate the embedded assets module
    generate_assets_module(&frontend_dist_dir, &out_dir);
}

fn should_rebuild_frontend(src_dir: &Path, dist_dir: &Path) -> bool {
    // If dist directory doesn't exist, we need to build
    if !dist_dir.exists() {
        return true;
    }

    // Get the newest file time in src directory
    let src_newest = get_newest_file_time(src_dir).unwrap_or(SystemTime::UNIX_EPOCH);

    // Get the oldest file time in dist directory
    let dist_oldest = get_oldest_file_time(dist_dir).unwrap_or(SystemTime::now());

    // Rebuild if any source file is newer than any dist file
    src_newest > dist_oldest
}

fn get_newest_file_time(dir: &Path) -> Option<SystemTime> {
    let mut newest = SystemTime::UNIX_EPOCH;

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                if let Some(dir_newest) = get_newest_file_time(&path) {
                    newest = newest.max(dir_newest);
                }
            } else if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    newest = newest.max(modified);
                }
            }
        }
    }

    if newest == SystemTime::UNIX_EPOCH {
        None
    } else {
        Some(newest)
    }
}

fn get_oldest_file_time(dir: &Path) -> Option<SystemTime> {
    let mut oldest = SystemTime::now();
    let mut found_file = false;

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                if let Some(dir_oldest) = get_oldest_file_time(&path) {
                    oldest = oldest.min(dir_oldest);
                    found_file = true;
                }
            } else if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    oldest = oldest.min(modified);
                    found_file = true;
                }
            }
        }
    }

    if found_file {
        Some(oldest)
    } else {
        None
    }
}

fn build_frontend(frontend_dir: &Path) {
    println!("cargo:warning=Running npm run build in {:?}", frontend_dir);

    // First, ensure dependencies are installed
    let npm_install = Command::new("npm")
        .arg("install")
        .current_dir(frontend_dir)
        .status()
        .expect("Failed to run npm install");

    if !npm_install.success() {
        panic!("npm install failed");
    }

    // Then run the build
    let npm_build = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(frontend_dir)
        .status()
        .expect("Failed to run npm run build");

    if !npm_build.success() {
        panic!("npm run build failed");
    }
}

fn generate_assets_module(dist_dir: &Path, out_dir: &str) {
    let mut assets = HashMap::new();

    if dist_dir.exists() {
        collect_assets(dist_dir, dist_dir, &mut assets);
    }

    let dest_path = Path::new(out_dir).join("frontend_assets.rs");
    let mut file = File::create(&dest_path).expect("Failed to create frontend_assets.rs");

    // Write the module header
    writeln!(file, "// Auto-generated frontend assets").unwrap();
    writeln!(file, "use std::collections::HashMap;").unwrap();
    writeln!(file, "use once_cell::sync::Lazy;").unwrap();
    writeln!(file, "").unwrap();

    // Write the BundledContent struct
    writeln!(file, "#[derive(Debug, Clone)]").unwrap();
    writeln!(file, "pub struct BundledContent {{").unwrap();
    writeln!(file, "    pub data: Vec<u8>,").unwrap();
    writeln!(file, "    pub mime_type: String,").unwrap();
    writeln!(file, "}}").unwrap();
    writeln!(file, "").unwrap();

    // Write the static HashMap
    writeln!(file, "pub static FRONTEND_ASSETS: Lazy<HashMap<String, BundledContent>> = Lazy::new(|| {{").unwrap();
    writeln!(file, "    let mut assets = HashMap::new();").unwrap();

    // Write each asset
    for (path, (file_path, mime_type)) in &assets {
        let safe_path = path.replace('\\', "/");
        writeln!(file, "    assets.insert(").unwrap();
        writeln!(file, "        \"{}\".to_string(),", safe_path).unwrap();
        writeln!(file, "        BundledContent {{").unwrap();
        writeln!(file, "            data: include_bytes!(\"{}\").to_vec(),", file_path.display()).unwrap();
        writeln!(file, "            mime_type: \"{}\".to_string(),", mime_type).unwrap();
        writeln!(file, "        }},").unwrap();
        writeln!(file, "    );").unwrap();
    }

    writeln!(file, "    assets").unwrap();
    writeln!(file, "}});").unwrap();

    // Add a helper function to get assets
    writeln!(file, "").unwrap();
    writeln!(file, "pub fn get_asset(path: &str) -> Option<&BundledContent> {{").unwrap();
    writeln!(file, "    FRONTEND_ASSETS.get(path)").unwrap();
    writeln!(file, "}}").unwrap();

    // Add a helper function to list all asset paths
    writeln!(file, "").unwrap();
    writeln!(file, "pub fn list_assets() -> Vec<&'static str> {{").unwrap();
    writeln!(file, "    FRONTEND_ASSETS.keys().map(|k| k.as_str()).collect()").unwrap();
    writeln!(file, "}}").unwrap();
}

fn collect_assets(current_dir: &Path, base_dir: &Path, assets: &mut HashMap<String, (PathBuf, String)>) {
    if let Ok(entries) = fs::read_dir(current_dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                collect_assets(&path, base_dir, assets);
            } else if let Ok(relative_path) = path.strip_prefix(base_dir) {
                let path_str = relative_path.to_string_lossy().to_string();
                let mime_type = guess_mime_type(&path);
                assets.insert(path_str, (path.clone(), mime_type));
            }
        }
    }
}

fn guess_mime_type(path: &Path) -> String {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("eot") => "application/vnd.ms-fontobject",
        Some("otf") => "font/otf",
        Some("map") => "application/json",
        Some("txt") => "text/plain",
        Some("xml") => "application/xml",
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }.to_string()
}