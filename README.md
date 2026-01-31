# sysinfo-logger
#### Runs basic system info checks and sends logs over the network
#### See: https://crates.io/crates/better-logger for more details
## config.toml
#### Put in same directory as the executable
```toml
terminal_logs = true
terminal_log_lvl = "error"
wasm_logging = false
file_logs = false
file_log_lvl = ""
log_file_path = ""
network_logs = true
network_log_lvl = "info"
network_endpoint_url_low = "https://info-log-dump.com/"
network_endpoint_url_high = "https://warn-error.com/"
debug_extra = false
async_logging = false
machine_name = "testing-01"
loop_seconds = 60
log_at_this_interval = 30
memory_warn_threshold = 85.0
memory_error_threshold = 90.0
cpu_warn_threshold = 70.0
cpu_error_threshold = 90
[network_format]
type = "JsonText"
field = "text"
```
### This executable is committed:
- I do this so I don't have to build on the production VM.
- You should clean and rebuild your own executable.
```bash
cargo +stable build --release --target x86_64-unknown-linux-gnu
```