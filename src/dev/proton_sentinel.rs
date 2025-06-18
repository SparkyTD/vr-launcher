use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use sysinfo::{Pid, System};

#[allow(dead_code)]
pub struct ProtonSentinel;

#[allow(dead_code)]
impl ProtonSentinel {
    pub fn start() -> anyhow::Result<()> {
        println!("Starting Proton Sentinel...");

        let mut sys = System::new_all();
        let mut proc_hash_map = HashMap::<Pid, String>::new();

        let host_env = env::vars_os()
            .map(|(k, v)| (k.into_string().unwrap(), v.into_string().unwrap()))
            .filter(|(k, _)| k != "LD_LIBRARY_PATH" && k != "PATH")
            .collect::<HashMap<_, _>>();

        loop {
            sys.refresh_all();

            let proc_list = sys.processes_by_name(OsStr::new("python3"))
                .filter(|p| p.cmd().len() >= 2 && p.cmd()[1].to_str().unwrap().ends_with("/proton"));

            for process in proc_list {
                let cmd = process.cmd().iter().map(|a| a.to_str().unwrap()).collect::<Vec<_>>();
                let cmd_joined = cmd.join(" ");
                if let Some(cmd) = proc_hash_map.get(&process.pid()) {
                    if *cmd == cmd_joined {
                        continue;
                    }
                }
                proc_hash_map.insert(process.pid(), cmd_joined);

                println!("Process {}: {:?}", process.pid(), process.name());

                println!("\nArguments:");
                println!("\t{:?}", process.cmd());

                println!("\nEnvironment:");
                for env in process.environ() {
                    let (key, _) = env.to_str().unwrap().split_once("=").unwrap();
                    if host_env.contains_key(key) {
                        continue;
                    }
                    println!("\t{:?}", env);
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(3000));
        }
    }
}