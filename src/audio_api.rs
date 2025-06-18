use pipewire::context::Context;
use pipewire::main_loop::MainLoop;
use pipewire::metadata::{Metadata, MetadataListener};
use pipewire::types::ObjectType;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;

#[derive(Debug, Clone, Serialize)]
pub enum DeviceChangeEvent {
    DefaultInputChanged(AudioDevice),
    DefaultOutputChanged(AudioDevice),
}

#[allow(dead_code)]
pub struct PipeWireManager {
    audio_state: Arc<Mutex<AudioState>>,
    join_handle: JoinHandle<()>,
    change_tx: Sender<DeviceChangeEvent>,
}

pub struct AudioState {
    input_devices: HashMap<u32, AudioDevice>,
    output_devices: HashMap<u32, AudioDevice>,

    default_input_device: Option<AudioDevice>,
    default_output_device: Option<AudioDevice>,

    defaults_metadata: Option<SendBox<Metadata>>,
    metadata_listener: Option<SendBox<MetadataListener>>,
}

#[allow(dead_code)]
impl PipeWireManager {
    pub fn new() -> Self {
        pipewire::init();

        let (change_tx, _) = broadcast::channel(100);

        let audio_state = Arc::new(Mutex::new(AudioState {
            input_devices: HashMap::new(),
            output_devices: HashMap::new(),
            default_input_device: None,
            default_output_device: None,
            defaults_metadata: None,
            metadata_listener: None,
        }));

        let change_tx_clone = change_tx.clone();
        let audio_state_clone = audio_state.clone();
        let join_handle = std::thread::spawn(move || {
            if let Err(error) = Self::run_pipewire_thread(audio_state_clone, change_tx_clone) {
                eprintln!("Failed to run the PipeWire thread: {:?}", error);
            }
        });

        std::thread::sleep(std::time::Duration::from_millis(200));

        Self {
            audio_state,
            join_handle,
            change_tx,
        }
    }

    fn run_pipewire_thread(audio_state: Arc<Mutex<AudioState>>, change_tx: Sender<DeviceChangeEvent>) -> anyhow::Result<()> {
        let main_loop = MainLoop::new(None)?;
        let context = Context::new(&main_loop)?;
        let core = context.connect(None)?;
        let registry = core.get_registry()?;

        let audio_state_clone = audio_state.clone();

        let _registry_listener = registry
            .add_listener_local()
            .global(move |global| {
                if global.type_ == ObjectType::Metadata && global.props.is_some_and(|p| p.get("metadata.name").is_some_and(|p| p == "default")) {
                    let registry = core.get_registry().unwrap();
                    if let Ok(metadata) = registry.bind::<Metadata, _>(global) {
                        let audio_state = audio_state.clone();
                        let audio_state_clone = audio_state.clone();
                        let change_tx = change_tx.clone();
                        let listener = metadata.add_listener_local()
                            .property(move |_subject, key, _type, value| {
                                if value.is_none() {
                                    return 0;
                                }

                                let mut audio_state = audio_state.lock().unwrap();

                                let device_name: serde_json::Value = serde_json::from_str(value.unwrap()).unwrap();
                                let device_name = device_name.get("name").unwrap().as_str().unwrap();

                                if let Some("default.audio.sink") = key {
                                    audio_state.output_devices.iter_mut().for_each(|(_, device)| { device.is_default = false });
                                    if let Some((_, device)) = audio_state.output_devices.iter_mut().find(|(_, d)| d.name == device_name) {
                                        device.is_default = true;
                                        let _ = change_tx.send(DeviceChangeEvent::DefaultOutputChanged(device.clone()));
                                        audio_state.default_output_device = Some(device.clone());
                                    }
                                }

                                if let Some("default.audio.source") = key {
                                    audio_state.input_devices.iter_mut().for_each(|(_, device)| { device.is_default = false });
                                    if let Some((_, device)) = audio_state.input_devices.iter_mut().find(|(_, d)| d.name == device_name) {
                                        device.is_default = true;
                                        let _ = change_tx.send(DeviceChangeEvent::DefaultInputChanged(device.clone()));
                                        audio_state.default_input_device = Some(device.clone());
                                    }
                                }

                                0
                            })
                            .register();

                        let mut audio_state = audio_state_clone.lock().unwrap();
                        audio_state.defaults_metadata.replace(SendBox(metadata));
                        audio_state.metadata_listener.replace(SendBox(listener));
                    }
                } else if global.type_ == ObjectType::Node && global.props.is_some_and(|p| p.get("media.class").is_some_and(|c| c == "Audio/Source" || c == "Audio/Sink")) {
                    let mut audio_state = audio_state.lock().unwrap();
                    let class = global.props.unwrap().get("media.class").unwrap();
                    let description = global.props.unwrap().get("node.description").unwrap().to_string();
                    let name = global.props.unwrap().get("node.name").unwrap().to_string();
                    match class {
                        "Audio/Source" => { audio_state.input_devices.insert(global.id, AudioDevice { id: global.id, name, description, is_default: false }); }
                        "Audio/Sink" => { audio_state.output_devices.insert(global.id, AudioDevice { id: global.id, name, description, is_default: false }); }
                        _ => {}
                    }
                }
            })
            .global_remove(move |global| {
                let mut audio_state = audio_state_clone.lock().unwrap();
                if let Some(_) = audio_state.input_devices.get(&global) {
                    audio_state.input_devices.remove(&global);
                }
                if let Some(_) = audio_state.output_devices.get(&global) {
                    audio_state.output_devices.remove(&global);
                }
            })
            .register();

        main_loop.run();

        Ok(())
    }

    pub fn get_input_devices(&self) -> HashSet<AudioDevice> {
        let audio_state = self.audio_state.lock().unwrap();
        audio_state.input_devices.values().cloned().collect()
    }

    pub fn get_output_devices(&self) -> HashSet<AudioDevice> {
        let audio_state = self.audio_state.lock().unwrap();
        audio_state.output_devices.values().cloned().collect()
    }

    pub fn get_default_input_device(&self) -> Option<AudioDevice> {
        let audio_state = self.audio_state.lock().unwrap();
        audio_state.default_input_device.clone()
    }

    pub fn get_default_output_device(&self) -> Option<AudioDevice> {
        let audio_state = self.audio_state.lock().unwrap();
        audio_state.default_output_device.clone()
    }

    pub fn set_default_input_device(&self, device: &AudioDevice) {
        let audio_state = self.audio_state.lock().unwrap();
        if let Some(metadata) = &audio_state.defaults_metadata {
            let value = json!({"name": device.name,}).to_string();
            metadata.0.set_property(0, "default.audio.source", Some("Spa:String:JSON"), Some(&value));
        }
    }

    pub fn set_default_output_device(&self, device: &AudioDevice) {
        let audio_state = self.audio_state.lock().unwrap();
        if let Some(metadata) = &audio_state.defaults_metadata {
            let value = json!({"name": device.name,}).to_string();
            metadata.0.set_property(0, "default.audio.sink", Some("Spa:String:JSON"), Some(&value));
        }
    }
    
    pub fn subscribe_to_changes(&self) -> broadcast::Receiver<DeviceChangeEvent> {
        self.change_tx.subscribe()
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
#[derive(Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub is_default: bool,
}

struct SendBox<T> (T);

unsafe impl<T> Send for SendBox<T> {}