use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use anyhow::ensure;
use crate::logging::log_channel::LogChannel;

pub struct LogSession {
    channels: HashMap<String, Arc<Mutex<LogChannel>>>,
}

impl LogSession {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    pub fn create_channel(&mut self, name: &str) -> anyhow::Result<Arc<Mutex<LogChannel>>> {
        ensure!(!name.is_empty(), "Name cannot be empty");
        
        if let Some(channel) = self.channels.remove(name) {
            let mut channel = channel.lock().expect("Failed to lock log channel");
            channel.shutdown()?;
        }

        let channel = Arc::new(Mutex::new(LogChannel::new(name)?));
        self.channels.insert(name.into(), channel.clone());

        Ok(channel)
    }
}