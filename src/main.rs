/* MAIN.RS */

use better_logger::{logger, LoggerSettings, NetworkFormat, NetworkEndpointUrl, MultipleNet};
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
    network_endpoint_url_low: String,
    network_endpoint_url_high: String,
    network_format: ConfigNetworkFormat,
    debug_extra: bool,
    async_logging: bool,
    machine_name: String,
    loop_seconds: u64,
    log_at_this_interval: u64,
    memory_warn_threshold: f64,
    memory_error_threshold: f64,
    cpu_warn_threshold: f32,
    cpu_error_threshold: f32,
}

#[derive(Clone, PartialEq)]
enum Status {
    Clean,
    CpuWarn,
    CpuError,
}
static STATUS: OnceLock<RwLock<Status>> = OnceLock::new();
impl Status {
    fn init() {
        STATUS.get_or_init(|| RwLock::new(Status::Clean));
    }
    fn set(new_status: Status) -> Result<(), String> {
        let lock = match STATUS.get() {
            Some(lock) => lock,
            None => return Err("status not initialized".to_string()),
        };

        let mut guard = match lock.write() {
            Ok(guard) => guard,
            Err(error) => return Err(format!("status poison: {}", error)),
        };

        *guard = new_status;
        return Ok(());
    }
    fn get() -> Result<Status, String> {
        let lock = match STATUS.get() {
            Some(lock) => lock,
            None => return Err("status not initialized".to_string()),
        };

        let guard = match lock.read() {
            Ok(guard) => guard,
            Err(error) => return Err(format!("status poison: {}", error)),
        };

        Ok(guard.clone())
    }
}

fn load_config(path: &str) -> Result<Config, Box<dyn Error>> {
    let raw = read_to_string(path)?;
    return Ok(toml::from_str(&raw)?);
}


fn bytes_to_mib(b: u64) -> f64 {
    return b as f64 / 1024.0 / 1024.0;
}

fn quit(error: String, machine_name: &str) {
    logger::error!("\nmachine name: {}\n!!! FATAL ERROR, process exiting !!!\nerror: {}",machine_name, error);
    exit(1);
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

    let endpoints = MultipleNet {
        trace: "".to_string(),
        debug: "".to_string(),
        debugx: "".to_string(),
        info: config.network_endpoint_url_low,
        warn: config.network_endpoint_url_high.clone(),
        error: config.network_endpoint_url_high,
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
        network_endpoint_url: NetworkEndpointUrl::Multiple(endpoints),
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
        if let Err(error) = run(
            &mut system, 
            &config.machine_name,
            config.memory_error_threshold,
            config.memory_warn_threshold,
            config.cpu_error_threshold,
            config.cpu_warn_threshold,
            false
        ) {
            quit(error, &config.machine_name);
        };

        count += 1;

        if count >= config.log_at_this_interval {
            if let Err(error) = run(
                &mut system, 
                &config.machine_name,
                config.memory_error_threshold,
                config.memory_warn_threshold,
                config.cpu_error_threshold,
                config.cpu_warn_threshold,
                true
            ) {
                quit(error, &config.machine_name);
            }
            count = 0;
        }

        sleep(Duration::from_secs(config.loop_seconds));       
    }
}

fn run(
    system: &mut System, 
    machine_name: &str,
    memory_error_threshold: f64,
    memory_warn_threshold: f64,
    cpu_error_threshold: f32,
    cpu_warn_threshold: f32,
    log: bool,
) -> Result<(), String> {
    system.refresh_memory();
    system.refresh_cpu_usage();
    sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    system.refresh_cpu_usage();

    let total_memory = bytes_to_mib(system.total_memory());
    let used_memory = bytes_to_mib(system.used_memory());
    let free_memory = bytes_to_mib(system.free_memory());
    let available_memory = bytes_to_mib(system.available_memory());

    let logical_cpus = system.cpus().len();
    let total_cpu_percent = system.global_cpu_usage();

    let memory_usage_percent = if total_memory > 0.0 {
        (used_memory / total_memory) * 100.0
    } else {
        0.0
    };

    let main_message = main_message(
        machine_name, 
        total_memory, 
        used_memory, 
        free_memory, 
        available_memory,
        logical_cpus, 
        total_cpu_percent, 
        memory_usage_percent
    );
   
    if memory_usage_percent >= memory_error_threshold {
        logger::error!("{}", memory_error_message(&main_message));
    } 
    else if memory_usage_percent >= memory_warn_threshold {
        logger::warn!("{}", memory_warn_message(&main_message));
    }

    if total_cpu_percent >= cpu_error_threshold {
        if Status::get()? == Status::CpuError {
            logger::error!("{}", cpu_error_message(&main_message));
        } else {
            Status::set(Status::CpuError)?;
        }
        return Ok(());
    }
    else if total_cpu_percent >= cpu_warn_threshold {
        if Status::get()? == Status::CpuWarn {
            logger::warn!("{}", cpu_warn_message(&main_message));
        } else {
            Status::set(Status::CpuWarn)?;
        }
        return Ok(());
    }

    if log {
        logger::info!("{}", main_message);
    }

    Status::set(Status::Clean)?;
    return Ok(());
}

fn main_message(
    machine_name: &str,
    total_memory: f64,
    used_memory: f64,
    free_memory: f64,
    available_memory: f64,
    logical_cpus: usize,
    total_cpu_percent: f32,
    memory_usage_percent: f64,
) -> String{
    let mem_header: &str = "=== VM Memory ===";
    let cpu_header: &str = "=== VM CPU ===";

    return format!(
        "\nMachine Name: {}\n\n{}\nTotal: {:.2} MiB\nUsed: {:.2} MiB\nFree: {:.2} MiB\nAvailable: {:.2}MiB\nPercent Used: {:.2}%\n\n{}\nvCPUs: {}\nPercent Used: {:.6}%", 
        machine_name, mem_header, total_memory, used_memory, free_memory, available_memory, memory_usage_percent, cpu_header, logical_cpus, total_cpu_percent
    );
}

fn memory_error_message(main_message: &str) -> String {
    return format!(
        "\n!!! MEMORY USAGE AT CRITICAL THRESHOLD !!!\n{}", main_message
    );
}
fn cpu_error_message(main_message: &str) -> String {
    return format!(
        "\n!!! CPU USAGE AT CRITICAL THRESHOLD !!!\n{}", main_message
    );
}

fn memory_warn_message(main_message: &str) -> String {
    return format!(
        "\n!!! memory usage at dangerous threshold !!!\n{}", main_message
    );
}
fn cpu_warn_message(main_message: &str) -> String {
    return format!(
        "\n!!! cpu usage at dangerous threshold !!!\n{}", main_message
    );
}

/* FILEEND */