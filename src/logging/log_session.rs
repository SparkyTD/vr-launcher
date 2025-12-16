use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use anyhow::ensure;
use chrono::{DateTime, Utc};
use flate2::Compression;
use flate2::write::GzEncoder;
use lazy_static::lazy_static;
use regex::Regex;
use tar::Builder;
use crate::logging::log_channel::LogChannel;

lazy_static!{
    static ref COLOR_LIST: Vec<colored::Color> = vec![
        colored::Color::Yellow,
        colored::Color::Green,
        colored::Color::Cyan,
        colored::Color::Magenta,
        colored::Color::Blue,
    ];
}

pub struct LogSession {
    logs_dir: PathBuf,
    start_time: SystemTime,
    channels: HashMap<String, Arc<Mutex<LogChannel>>>,
    last_color_index: usize,
}

impl LogSession {
    pub fn new(logs_dir: PathBuf) -> Self {
        if !logs_dir.exists() {
            fs::create_dir_all(&logs_dir).unwrap();
        }
        
        Self {
            logs_dir,
            start_time: SystemTime::now(),
            channels: HashMap::new(),
            last_color_index: 0,
        }
    }

    pub fn create_channel(&mut self, name: &str) -> anyhow::Result<Arc<Mutex<LogChannel>>> {
        ensure!(!name.is_empty(), "Name cannot be empty");
        ensure!(!self.channels.contains_key(name), "A log channel with the name {} already exists", name);

        let color = COLOR_LIST[self.last_color_index % COLOR_LIST.len()];
        self.last_color_index += 1;

        let channel = Arc::new(Mutex::new(LogChannel::new(name, self.start_time, &self.logs_dir, color)?));
        self.channels.insert(name.into(), channel.clone());

        Ok(channel)
    }

    pub fn archive_old_files(&mut self) -> anyhow::Result<()> {
        let regex = Regex::new(r"^(\d{4}-\d{2}-\d{2}_\d{2}:\d{2}:\d{2})_(.+?)\.log$")?;
        let files = fs::read_dir(self.logs_dir.as_path())?
            .filter_map(Result::ok)
            .map(|entry| (entry.path(), entry.file_name().to_str().unwrap().to_string()))
            .collect::<Vec<_>>();

        let mut files_dict: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for (path, filename) in files {
            if let Some(captures) = regex.captures(&filename) {
                let date_code = captures.get(1).unwrap().as_str();
                files_dict.entry(date_code.to_string())
                    .or_insert_with(Vec::new)
                    .push(path);
            }
        }

        for (date_code, log_paths) in files_dict {            
            let archive_filename = format!("{}.tar.gz", date_code);
            let archive_path = self.logs_dir.join(archive_filename);

            Self::create_tar_gz(&log_paths, archive_path)?;

            for log_file in log_paths {
                fs::remove_file(&log_file)?;
            }
        }

        Ok(())
    }

    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        let mut log_paths = Vec::new();
        for channel in self.channels.values() {
            let mut channel = channel.lock()
                .expect("Failed to lock the log channel");
            channel.shutdown()?;

            if channel.file_path.exists() {
                log_paths.push(channel.file_path.clone());
            }
        }
        self.channels.clear();

        // Archive old log files
        if !log_paths.is_empty() {
            let datetime: DateTime<Utc> = self.start_time.into();
            let archive_filename = format!("{}.tar.gz", datetime.format("%Y-%m-%d_%H:%M:%S"));
            let archive_path = self.logs_dir.join(archive_filename);

            Self::create_tar_gz(&log_paths, archive_path)?;

            for log_file in log_paths {
                fs::remove_file(&log_file)?;
            }
        }

        Ok(())
    }

    fn create_tar_gz(files: &[PathBuf], output_path: PathBuf) -> anyhow::Result<()> {
        let tar_gz = File::create(output_path)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = Builder::new(enc);

        for file_path in files {
            let mut file = File::open(file_path)?;
            tar.append_file(file_path.file_name().unwrap(), &mut file)?;
        }

        tar.finish()?;
        Ok(())
    }
}