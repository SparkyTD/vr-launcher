use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=frontend/");

    let npm_output = Command::new("npm")
        .args(&["run", "build"])
        .current_dir("./frontend")
        .output()
        .expect("Failed to execute npm command");

    if !npm_output.status.success() {
        panic!(
            "Frontend build failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&npm_output.stdout),
            String::from_utf8_lossy(&npm_output.stderr)
        );
    }

    let dist_path = Path::new("frontend/dist");
    let mut files = Vec::new();
    collect_files(&dist_path, &dist_path, &mut files);

    if files.is_empty() {
        panic!("No files found in frontend/dist/ after build");
    }

    // Generate Rust code
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("bundled_assets.rs");
    let mut f = fs::File::create(&dest_path).unwrap();

    writeln!(f, r#"
#[derive(Debug, Clone)]
pub struct BundledContent {{
    pub data: Vec<u8>,
    pub mime_type: String,
}}
"#).unwrap();

    for (i, (_relative_path, full_path)) in files.iter().enumerate() {
        writeln!(
            f,
            r#"static ASSET_DATA_{}: &[u8] = include_bytes!({:?});"#,
            i,
            full_path.canonicalize().unwrap().display()
        ).unwrap();
    }

    writeln!(f, "").unwrap();

    writeln!(f, "pub fn get_asset(path: &str) -> Option<BundledContent> {{").unwrap();
    writeln!(f, "    match path {{").unwrap();

    for (i, (relative_path, _)) in files.iter().enumerate() {
        let mime_type = get_mime_type(&relative_path);
        writeln!(
            f,
            r#"        "{}" => Some(BundledContent {{
            data: ASSET_DATA_{}.to_vec(),
            mime_type: "{}".to_string(),
        }}),"#,
            relative_path,
            i,
            mime_type
        ).unwrap();
    }

    writeln!(f, "        _ => None,").unwrap();
    writeln!(f, "    }}").unwrap();
    writeln!(f, "}}").unwrap();
}

fn collect_files(base: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let relative = path.strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((relative, path));
            } else if path.is_dir() {
                collect_files(base, &path, files);
            }
        }
    }
}

fn get_mime_type(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|s| s.to_str()) {
        Some("html") => "text/html",
        Some("js") => "application/javascript",
        Some("css") => "text/css",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("eot") => "application/vnd.ms-fontobject",
        Some("txt") => "text/plain",
        Some("xml") => "application/xml",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}