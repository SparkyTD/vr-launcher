use chrono::{DateTime, Utc};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::time::SystemTime;
use std::{env, fs, thread};
use std::process::Child;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncBufReadExt;

pub struct LogChannel {
    name: String,
    log_file: File,
    stdout_logger_handle: Option<thread::JoinHandle<()>>,
    stderr_logger_handle: Option<thread::JoinHandle<()>>,
    stdout_logger_handle_tokio: Option<tokio::task::JoinHandle<()>>,
    stderr_logger_handle_tokio: Option<tokio::task::JoinHandle<()>>,
}

impl LogChannel {
    pub fn new(name: &str) -> anyhow::Result<LogChannel> {
        let time = SystemTime::now();
        let datetime: DateTime<Utc> = time.into();
        let filename = format!("{}_{}.log", datetime.format("%Y-%m-%d_%H:%M:%S"), name);

        let logs_dir = env::current_dir()?
            .join("logs");

        fs::create_dir_all(&logs_dir)?;
        let log_file_path = logs_dir.join(filename);

        Ok(Self {
            name: name.into(),
            log_file: File::create(&log_file_path)?,
            stdout_logger_handle: None,
            stderr_logger_handle: None,
            stdout_logger_handle_tokio: None,
            stderr_logger_handle_tokio: None,
        })
    }

    pub fn write(&mut self, message: &str, log_type: LogType) {
        let now = SystemTime::now();
        let datetime: DateTime<Utc> = now.into();
        let timestamp = datetime.format("%Y-%m-%d %H:%M:%S");
        let log_type = match log_type {
            LogType::StdOut => "Output",
            LogType::StdErr => "Error",
        };
        println!("[{}] [{}] [{}] {}", timestamp, self.name, log_type, message);
        self.log_file.write_all(format!("{}\n", message).as_bytes()).unwrap();
    }

    pub fn connect_std(logger: Arc<Mutex<LogChannel>>, child: &mut Child) {
        let mut logger_lock = logger.lock().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let stderr = BufReader::new(child.stderr.take().unwrap());

        let stdout_logger = logger.clone();
        let stdout_handle = thread::spawn(move || {
            for line in stdout.lines() {
                if let Ok(line) = line {
                    let mut logger = stdout_logger.lock().unwrap();
                    logger.write(&line, LogType::StdOut);
                }
            }
        });
        logger_lock.stdout_logger_handle.replace(stdout_handle);

        let stderr_logger = logger.clone();
        let stderr_handle = thread::spawn(move || {
            for line in stderr.lines() {
                if let Ok(line) = line {
                    let mut logger = stderr_logger.lock().unwrap();
                    logger.write(&line, LogType::StdErr);
                }
            }
        });
        logger_lock.stderr_logger_handle.replace(stderr_handle);
    }

    pub fn connect_tokio(logger: Arc<Mutex<LogChannel>>, child: &mut tokio::process::Child) {
        let mut logger_lock = logger.lock().unwrap();

        if let Some(stdout) = child.stdout.take() {
            let stdout = tokio::io::BufReader::new(stdout);
            let stdout_logger = logger.clone();
            let stdout_handle = tokio::spawn(async move {
                let mut lines = stdout.lines();
                while let Some(line) = lines.next_line().await.unwrap() {
                    let mut logger = stdout_logger.lock().unwrap();
                    logger.write(&line, LogType::StdOut);
                }
            });
            logger_lock.stdout_logger_handle_tokio.replace(stdout_handle);
        }

        if let Some(stderr) = child.stderr.take() {
            let stderr = tokio::io::BufReader::new(stderr);
            let stderr_logger = logger.clone();
            let stderr_handle = tokio::spawn(async move {
                let mut lines = stderr.lines();
                while let Some(line) = lines.next_line().await.unwrap() {
                    let mut logger = stderr_logger.lock().unwrap();
                    logger.write(&line, LogType::StdErr);
                }
            });
            logger_lock.stderr_logger_handle_tokio.replace(stderr_handle);
        }
    }

    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        _ = self.stdout_logger_handle.take();
        _ = self.stderr_logger_handle.take();
        _ = self.stdout_logger_handle_tokio.take();
        _ = self.stderr_logger_handle_tokio.take();
        
        Ok(())
    }
}

pub enum LogType {
    StdOut,
    StdErr,
}