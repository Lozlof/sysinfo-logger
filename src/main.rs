use sysinfo::System;

fn bytes_to_mib(b: u64) -> f64 {
    b as f64 / 1024.0 / 1024.0
}

fn main() {
    // Load CPUs + memory
    let mut sys = System::new_all();

    // Initial snapshot
    sys.refresh_all();

    // CPU usage needs a delta
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_cpu_usage();

    // --- MEMORY (VM view) ---
    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let free_mem = sys.free_memory();

    // --- CPU (VM view) ---
    let logical_cpus = sys.cpus().len();
    let total_cpu_percent = sys.global_cpu_usage();

    println!("=== VM Memory ===");
    println!("Total: {:.2} MiB", bytes_to_mib(total_mem));
    println!("Used : {:.2} MiB", bytes_to_mib(used_mem));
    println!("Free : {:.2} MiB", bytes_to_mib(free_mem));

    println!("\n=== VM CPU ===");
    println!("vCPUs: {}", logical_cpus);
    println!("Total CPU usage: {:.2}%", total_cpu_percent);
}
