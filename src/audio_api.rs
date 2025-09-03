use pipewire::context::Context;
use pipewire::main_loop::MainLoop;
use pipewire::metadata::{Metadata, MetadataListener};
use pipewire::node::{Node, NodeListener};
use pipewire::registry::{Listener, Registry};
use pipewire::spa::pod::deserialize::PodDeserializer;
use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::spa::pod::{Pod, Property, Value, ValueArray};
use pipewire::spa::support::system::IoFlags;
use pipewire::spa::sys::{SPA_PROP_channelVolumes, SPA_PROP_mute};
use pipewire::spa::utils::SpaTypes;
use pipewire::spa::{param::ParamType, pod};
use pipewire::types::ObjectType;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::io;
use std::os::fd::OwnedFd;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;

#[derive(Debug, Clone, Serialize)]
pub enum DeviceChangeEvent {
    DefaultInputChanged(AudioDevice),
    DefaultOutputChanged(AudioDevice),
    VolumeMuteChanged(AudioDevice),
}

#[allow(dead_code)]
pub struct PipeWireManager {
    audio_state: Arc<Mutex<AudioState>>,
    join_handle: JoinHandle<()>,
    change_tx: Sender<DeviceChangeEvent>,
    cmd_write_fd: OwnedFd,
    cmd_tx: std::sync::mpsc::Sender<AudioControlCommand>,
}

pub struct AudioState {
    input_devices: HashMap<u32, AudioDevice>,
    output_devices: HashMap<u32, AudioDevice>,

    default_input_device: Option<AudioDevice>,
    default_output_device: Option<AudioDevice>,

    defaults_metadata: Option<SendBox<Metadata>>,
    metadata_listener: Option<SendBox<MetadataListener>>,
    node_listeners: HashMap<u32, SendBox<NodeListener>>,
    node_proxies: HashMap<u32, SendBox<Node>>,
}

impl AudioState {
    pub fn set_default_input_device(&self, device: AudioDevice) {
        if let Some(metadata) = &self.defaults_metadata {
            let value = json!({"name": device.name,}).to_string();
            metadata.0.set_property(0, "default.audio.source", Some("Spa:String:JSON"), Some(&value));
        }
    }

    pub fn set_default_output_device(&self, device: AudioDevice) {
        if let Some(metadata) = &self.defaults_metadata {
            let value = json!({"name": device.name,}).to_string();
            metadata.0.set_property(0, "default.audio.sink", Some("Spa:String:JSON"), Some(&value));
        }
    }

    pub fn set_device_volume(&self, device: AudioDevice, volume: u8, muted: bool) {
        let volume = volume.min(100) as f32 / 100f32;

        if let Some(proxy) = self.node_proxies.get(&device.id) {
            let volume_float = volume.powf(3f32);
            let device = self.input_devices.get(&device.id)
                .or_else(|| self.output_devices.get(&device.id))
                .expect("Could not find the specified device");

            if let Some(pod_data) = &device.pod_bytes {
                let pod_data = PipeWireManager::deserialize_pod_value(pod_data);
                if let Some(Value::Object(mut obj)) = pod_data {
                    let mut channel_count: Option<usize> = None;
                    for property in &mut obj.properties {
                        #[allow(non_upper_case_globals)]
                        match property.key {
                            SPA_PROP_channelVolumes => {
                                if let Value::ValueArray(ValueArray::Float(values)) = &property.value {
                                    channel_count.replace(values.len());
                                }
                            }
                            SPA_PROP_mute => {
                                property.value = Value::Bool(muted);
                            }
                            _ => {}
                        }
                    }

                    if let Some(channel_count) = channel_count {
                        let channel_volumes = vec![volume_float; channel_count];
                        let pod_data = Value::Object(pod::object! {
                                                        SpaTypes::ObjectParamProps,
                                                        ParamType::Props,
                                                        Property::new(SPA_PROP_channelVolumes, Value::ValueArray(ValueArray::Float(channel_volumes))),
                                                        Property::new(SPA_PROP_mute, Value::Bool(muted)),
                                                    });

                        if let Ok((cursor, _)) = PodSerializer::serialize(io::Cursor::new(Vec::new()), &pod_data) {
                            let pod_bytes = cursor.into_inner();
                            let pod = Pod::from_bytes(pod_bytes.as_ref());
                            proxy.0.set_param(ParamType::Props, 0, pod.unwrap());
                        }
                    }
                }
            }
        }
    }
}

#[allow(dead_code)]
impl PipeWireManager {
    pub fn new() -> anyhow::Result<Self> {
        pipewire::init();

        let (change_tx, _) = broadcast::channel(100);
        let audio_state = Arc::new(Mutex::new(AudioState {
            input_devices: HashMap::new(),
            output_devices: HashMap::new(),
            default_input_device: None,
            default_output_device: None,
            defaults_metadata: None,
            metadata_listener: None,
            node_listeners: HashMap::new(),
            node_proxies: HashMap::new(),
        }));

        // Nix pip for main loop communication
        let (read_fd, write_fd) = nix::unistd::pipe()?;
        use nix::fcntl::{fcntl, FcntlArg, OFlag};
        let flags = fcntl(&read_fd, FcntlArg::F_GETFL)?;
        fcntl(&read_fd, FcntlArg::F_SETFL(OFlag::from_bits_truncate(flags) | OFlag::O_NONBLOCK))?;

        // Pip for sending commands to main loop
        let (tx, rx) = std::sync::mpsc::channel::<AudioControlCommand>();

        let change_tx_clone = change_tx.clone();
        let audio_state_clone = audio_state.clone();
        let join_handle = std::thread::spawn(move || {
            match Self::create_pipewire_thread(audio_state_clone.clone(), change_tx_clone) {
                Ok(context) => {
                    let audio_state = audio_state_clone.clone();
                    let _io_source = context.main_loop.loop_().add_io(read_fd.try_clone().unwrap(), IoFlags::IN, move |_fd| {
                        // Drain pipe
                        let mut buf = [0u8; 1];
                        while nix::unistd::read(&read_fd, &mut buf).is_ok() {
                            continue;
                        }

                        while let Ok(command) = rx.try_recv() {
                            match command {
                                AudioControlCommand::SetDefaultInputDevice { device } => {
                                    let audio_state = audio_state.lock().unwrap();
                                    audio_state.set_default_input_device(device);
                                }
                                AudioControlCommand::SetDefaultOutputDevice { device } => {
                                    let audio_state = audio_state.lock().unwrap();
                                    audio_state.set_default_output_device(device);
                                }
                                AudioControlCommand::SetVolume { device, volume, muted } => {
                                    let audio_state = audio_state.lock().unwrap();
                                    audio_state.set_device_volume(device, volume, muted);
                                }
                            }
                        }
                    });

                    println!("Starting audio loop");
                    context.main_loop.run();
                }
                Err(error) => eprintln!("Failed to run the PipeWire thread: {:?}", error),
            }
        });

        std::thread::sleep(std::time::Duration::from_millis(200));

        Ok(Self {
            audio_state,
            join_handle,
            change_tx,
            cmd_write_fd: write_fd,
            cmd_tx: tx,
        })
    }

    pub fn deserialize_pod_value(pod: &[u8]) -> Option<Value> {
        let deserializer = PodDeserializer::deserialize_from::<Value>(pod);
        if let Ok((_, Value::Object(obj))) = deserializer {
            Some(Value::Object(obj))
        } else {
            None
        }
    }

    fn parse_volume_from_pod(pod: &Pod) -> Option<(f32, bool, Vec<u8>)> {
        if let Some(Value::Object(obj)) = Self::deserialize_pod_value(pod.as_bytes()) {
            let mut volume: Option<f32> = None;
            let mut muted: Option<bool> = None;

            for property in &obj.properties {
                #[allow(non_upper_case_globals)]
                match property.key {
                    SPA_PROP_channelVolumes => {
                        if let Value::ValueArray(ValueArray::Float(values)) = &property.value {
                            let mut max_vol = 0.0f32;
                            for val in values {
                                max_vol = max_vol.max(*val);
                            }
                            volume = Some(max_vol);
                        }
                    }
                    SPA_PROP_mute => {
                        if let Value::Bool(v) = property.value {
                            muted = Some(v)
                        }
                    }
                    _ => {}
                }
            }

            if let (Some(volume), Some(muted)) = (volume, muted) {
                return Some((volume, muted, pod.as_bytes().to_vec()));
            }
        }
        None
    }

    fn create_pipewire_thread(audio_state: Arc<Mutex<AudioState>>, change_tx: Sender<DeviceChangeEvent>) -> anyhow::Result<PipeWireContext> {
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
                                if value.is_none_or(|v| v == "-1") {
                                    return 0;
                                }

                                let mut audio_state = audio_state.lock().unwrap();

                                let device_info: serde_json::Value = serde_json::from_str(value.unwrap()).unwrap();
                                let device_name = device_info.get("name").unwrap().as_str().unwrap();

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
                    let audio_state_clone = audio_state.clone();
                    let mut audio_state = audio_state.lock().unwrap();
                    let class = global.props.unwrap().get("media.class").unwrap();
                    let description = global.props.unwrap().get("node.description").unwrap().to_string();
                    let name = global.props.unwrap().get("node.name").unwrap().to_string();
                    match class {
                        "Audio/Source" => { audio_state.input_devices.insert(global.id, AudioDevice { id: global.id, name, description, is_default: false, volume: 100, is_muted: false, pod_bytes: None }); }
                        "Audio/Sink" => { audio_state.output_devices.insert(global.id, AudioDevice { id: global.id, name, description, is_default: false, volume: 100, is_muted: false, pod_bytes: None }); }
                        _ => {}
                    }

                    let registry = core.get_registry().unwrap();
                    if let Ok(node) = registry.bind::<Node, _>(global) {
                        let device_id = global.id;
                        let change_tx = change_tx.clone();
                        let listener = node
                            .add_listener_local()
                            .param(move |_seq, param_type, _index, _next, param| {
                                if param_type != ParamType::Props || param.is_none() {
                                    return;
                                }

                                let param = param.unwrap();
                                if let Some((volume, is_muted, pod_bytes)) = Self::parse_volume_from_pod(&param) {
                                    let mut audio_state = audio_state_clone.lock().unwrap();
                                    let device_opt = match audio_state.input_devices.get_mut(&device_id) {
                                        Some(device) => Some(device),
                                        None => audio_state.output_devices.get_mut(&device_id),
                                    };
                                    if let Some(device) = device_opt {
                                        device.is_muted = is_muted;
                                        device.volume = (volume.powf(1f32 / 3f32) * 100f32) as u8;
                                        device.pod_bytes = Some(pod_bytes);
                                        let _ = change_tx.send(DeviceChangeEvent::VolumeMuteChanged(device.clone()));
                                    }
                                }
                            })
                            .register();
                        node.subscribe_params(&[ParamType::Props]);
                        audio_state.node_listeners.insert(device_id, SendBox(listener));
                        audio_state.node_proxies.insert(device_id, SendBox(node));
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

        Ok(PipeWireContext {
            main_loop,
            context,
            registry,
            registry_listener: _registry_listener,
        })
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
        self.send_command_to_loop(AudioControlCommand::SetDefaultInputDevice {
            device: device.clone(),
        }).unwrap();
    }

    pub fn set_default_output_device(&self, device: &AudioDevice) {
        self.send_command_to_loop(AudioControlCommand::SetDefaultOutputDevice {
            device: device.clone(),
        }).unwrap();
    }

    pub fn set_device_volume(&self, device: &AudioDevice, volume: u8, muted: bool) {
        self.send_command_to_loop(AudioControlCommand::SetVolume {
            device: device.clone(),
            volume,
            muted,
        }).unwrap();
    }

    pub fn subscribe_to_changes(&self) -> broadcast::Receiver<DeviceChangeEvent> {
        self.change_tx.subscribe()
    }

    fn send_command_to_loop(&self, cmd: AudioControlCommand) -> anyhow::Result<()> {
        self.cmd_tx.send(cmd)?;
        nix::unistd::write(&self.cmd_write_fd, &[1])?;

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
#[derive(Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub is_default: bool,

    #[serde(skip_serializing)]
    pub pod_bytes: Option<Vec<u8>>,
    pub volume: u8,
    pub is_muted: bool,
}

struct SendBox<T> (T);

unsafe impl<T> Send for SendBox<T> {}

#[derive(Debug)]
enum AudioControlCommand {
    SetDefaultInputDevice { device: AudioDevice },
    SetDefaultOutputDevice { device: AudioDevice },
    SetVolume { device: AudioDevice, volume: u8, muted: bool },
}

#[allow(dead_code)]
struct PipeWireContext {
    main_loop: MainLoop,
    context: Context,
    registry: Registry,
    registry_listener: Listener,
}