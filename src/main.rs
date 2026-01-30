/* MAIN.RS */

use better_logger::{LoggerSettings, NetworkFormat, logger};
use std::fs::read_to_string;
use std::error::Error;
use std::process::exit;
use std::thread::sleep;
use std::time::Duration;
use std::sync::{OnceLock, RwLock};
use sysinfo::System;
use serde::Deserialize;
use chrono::Local;

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ConfigNetworkFormat {
    PlainText,
    JsonText { field: String },
}

impl From<ConfigNetworkFormat> for NetworkFormat {
    fn from(v: ConfigNetworkFormat) -> Self {
        match v {
            ConfigNetworkFormat::PlainText => NetworkFormat::PlainText,
            ConfigNetworkFormat::JsonText { field } => NetworkFormat::JsonText { field },
        }
    }
}

#[derive(Deserialize)]
struct Config {
    terminal_logs: bool,
    terminal_log_lvl: String,
    wasm_logging: bool,
    file_logs: bool,
    file_log_lvl: String,
    log_file_path: String,
    network_logs: bool,
    network_log_lvl: String,
    network_endpoint_url: String,
    network_format: ConfigNetworkFormat,
    debug_extra: bool,
    async_logging: bool,
    machine_name: String,
    loop_seconds: u64,
    log_at_this_interval: u64,
    memory_warn_threshold: f64,
    memory_error_threshold: f64,
    cpu_warn_threshold: f64,
    cpu_error_threshold: f64,
}

enum Status {
    Clean
}
static STATUS: OnceLock<RwLock<Status>> = OnceLock::new();
impl Status {
    fn init() {
        STATUS.get_or_init(|| RwLock::new(Status::Clean));
    }
    pub fn set(new_status: Status) {
        let lock = match STATUS.get() {
            Some(lock) => lock,
            None => panic!("Status::init() was not called"),
        };

        let mut guard = match lock.write() {
            Ok(guard) => guard,
            Err(_) => panic!("STATUS lock poisoned"),
        };

        *guard = new_status;
    }
 
}

fn load_config(path: &str) -> Result<Config, Box<dyn Error>> {
    let raw = read_to_string(path)?;
    return Ok(toml::from_str(&raw)?);
}


fn bytes_to_mib(b: u64) -> f64 {
    return b as f64 / 1024.0 / 1024.0;
}

fn quit() {

}

fn main() {
    let now = Local::now();
    let now_formatted = format!("{}", now.format("%Y-%m-%d %H:%M:%S"));

    let config_path = "config.toml";
    let config = match load_config(&config_path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{}: {:?}", now_formatted, err);
            exit(1);
        }
    };

    let settings = LoggerSettings {
        terminal_logs: config.terminal_logs,
        terminal_log_lvl: config.terminal_log_lvl,
        wasm_logging: config.wasm_logging,
        file_logs: config.file_logs,
        file_log_lvl: config.file_log_lvl,
        log_file_path: config.log_file_path,
        network_logs: config.network_logs,
        network_log_lvl: config.network_log_lvl,
        network_endpoint_url: config.network_endpoint_url,
        network_format: config.network_format.into(),
        debug_extra: config.debug_extra,
        async_logging: config.async_logging,
    };

    if let Err(err) = logger::init(settings) {
        eprintln!("{}: {:?}", now_formatted, err);
        exit(1);
    }

    Status::init();
    let mut system = System::new_all();

    let mut count: u64 = 0;
    loop {
        //run(&mut system, false);

        count += 1;

        if count >= config.log_at_this_interval {
            //run(&mut system, true);
            count = 0;
        }

        sleep(Duration::from_secs(config.loop_seconds));       
    }
}

fn run(system: &mut System, previous_status: Status, log: bool) {
    system.refresh_all();

    sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    system.refresh_cpu_usage();

    let total_memory = system.total_memory();
    let used_memory = system.used_memory();
    let free_memory = system.free_memory();

    let logical_cpus = system.cpus().len();
    let total_cpu_percent = system.global_cpu_usage();

    let mem_usage_percent = (used_memory as f64 / total_memory as f64) * 100.0;
    let cpu_usage_percent = total_cpu_percent;



    println!("=== VM Memory ===");
    println!("Total: {:.2} MiB", bytes_to_mib(total_memory));
    println!("Used : {:.2} MiB", bytes_to_mib(used_memory));
    println!("Free : {:.2} MiB", bytes_to_mib(free_memory));

    println!("\n=== VM CPU ===");
    println!("vCPUs: {}", logical_cpus);
    println!("Total CPU usage: {:.2}%", total_cpu_percent);

    println!("\n=== PER ===");
    println!("per mem: {:.2}%", mem_usage_percent);
    println!("per cpu: {:.2}%", cpu_usage_percent);
}



/* FILEEND */