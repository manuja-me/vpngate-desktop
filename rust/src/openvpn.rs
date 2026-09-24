use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use crate::models::VpnServer;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VpnState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
    Error,
}

pub struct OpenVpnManager {
    pub state: Arc<Mutex<VpnState>>,
    pub status_message: Arc<Mutex<String>>,
    pub log_sender: Sender<String>,
    pub log_receiver: Arc<Mutex<Receiver<String>>>,
    active_child: Arc<Mutex<Option<Child>>>,
    active_temp_dir: Arc<Mutex<Option<PathBuf>>>,
    active_mgmt_port: Arc<Mutex<Option<u16>>>,
    pub connection_start_time: Arc<Mutex<Option<Instant>>>,
}

impl OpenVpnManager {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            state: Arc::new(Mutex::new(VpnState::Disconnected)),
            status_message: Arc::new(Mutex::new(String::new())),
            log_sender: tx,
            log_receiver: Arc::new(Mutex::new(rx)),
            active_child: Arc::new(Mutex::new(None)),
            active_temp_dir: Arc::new(Mutex::new(None)),
            active_mgmt_port: Arc::new(Mutex::new(None)),
            connection_start_time: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_state(&self) -> VpnState {
        *self.state.lock().unwrap()
    }

    #[allow(dead_code)]
    pub fn get_status_message(&self) -> String {
        self.status_message.lock().unwrap().clone()
    }

    pub fn get_connection_duration(&self) -> Option<Duration> {
        self.connection_start_time.lock().unwrap().map(|t| t.elapsed())
    }

    #[allow(dead_code)]
    pub fn is_installed(&self) -> bool {
        find_openvpn_binary().is_some()
    }

    pub fn connect(&self, server: &VpnServer) -> Result<(), String> {
        let openvpn_exe = find_openvpn_binary().ok_or_else(|| {
            "OpenVPN executable not found. Please install OpenVPN.".to_string()
        })?;

        // 1. Purge any leftover zombie routes
        purge_stale_routes();

        // 2. Decode configuration
        let raw_config = server
            .get_decoded_config()
            .ok_or_else(|| "Failed to decode OpenVPN profile configuration.".to_string())?;

        // 3. Create active temp directory
        let temp_dir = std::env::temp_dir().join(format!("vpngate_{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create working directory: {}", e))?;

        let config_path = temp_dir.join("profile.ovpn");
        let auth_path = temp_dir.join("auth.txt");

        // Write credentials
        fs::write(&auth_path, "vpn\nvpn\n")
            .map_err(|e| format!("Failed to write credentials: {}", e))?;

        // Sanitize and inject leak protection directives
        let mut clean_config = String::new();
        for line in raw_config.lines() {
            let trimmed = line.trim();
            if trimmed.eq_ignore_ascii_case("auth-user-pass")
                || trimmed.to_lowercase().starts_with("auth-user-pass ")
                || trimmed.eq_ignore_ascii_case("persist-key")
                || trimmed.to_lowercase().starts_with("block-outside-dns")
            {
                continue;
            }
            clean_config.push_str(line);
            clean_config.push('\n');
        }

        clean_config.push_str("\n# === Injected Leak Protection & Resolvers ===\n");
        clean_config.push_str("dhcp-option DNS 8.8.8.8\n");
        clean_config.push_str("dhcp-option DNS 1.1.1.1\n");
        clean_config.push_str("block-ipv6\n");

        fs::write(&config_path, clean_config)
            .map_err(|e| format!("Failed to write profile: {}", e))?;

        let mgmt_port = get_available_port();
        *self.active_mgmt_port.lock().unwrap() = Some(mgmt_port);
        *self.active_temp_dir.lock().unwrap() = Some(temp_dir.clone());

        {
            let mut state = self.state.lock().unwrap();
            *state = VpnState::Connecting;
        }

        let _ = self.log_sender.send(format!(
            "Connecting to relay {} ({}:{}) via mgmt port {}...",
            server.country_long, server.ip, server.port, mgmt_port
        ));

        let mut child = Command::new(&openvpn_exe)
            .args([
                "--config", "profile.ovpn",
                "--auth-user-pass", "auth.txt",
                "--management", "127.0.0.1", &mgmt_port.to_string(),
                "--verb", "3"
            ])
            .current_dir(&temp_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to spawn OpenVPN: {}", e))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        *self.active_child.lock().unwrap() = Some(child);

        // Spawn output monitor threads
        let log_tx_out = self.log_sender.clone();
        let state_clone = Arc::clone(&self.state);
        let status_clone = Arc::clone(&self.status_message);
        let start_time_clone = Arc::clone(&self.connection_start_time);

        if let Some(out) = stdout {
            thread::spawn(move || {
                let reader = BufReader::new(out);
                for line_res in reader.lines() {
                    if let Ok(line) = line_res {
                        let _ = log_tx_out.send(line.clone());
                        if line.contains("Initialization Sequence Completed") {
                            let mut st = state_clone.lock().unwrap();
                            *st = VpnState::Connected;
                            *status_clone.lock().unwrap() = "CONNECTED".to_string();
                            *start_time_clone.lock().unwrap() = Some(Instant::now());
                            let _ = log_tx_out.send(">>> Tunnel Established! Traffic is now routed through VPN Gate.".to_string());
                            flush_dns();
                        } else if line.contains("AUTH_FAILED") {
                            let mut st = state_clone.lock().unwrap();
                            *st = VpnState::Error;
                            *status_clone.lock().unwrap() = "Authentication Failed".to_string();
                        } else if line.contains("Cannot resolve host") || line.contains("Connection refused") {
                            let mut st = state_clone.lock().unwrap();
                            *st = VpnState::Error;
                            *status_clone.lock().unwrap() = "Relay server unreachable".to_string();
                        } else if line.contains("route addition failed") || line.contains("requires elevation") {
                            let _ = log_tx_out.send("⚠️ [Warning] Route modification requires Administrator privileges.".to_string());
                        }
                    }
                }
            });
        }

        let log_tx_err = self.log_sender.clone();
        if let Some(err) = stderr {
            thread::spawn(move || {
                let reader = BufReader::new(err);
                for line_res in reader.lines() {
                    if let Ok(line) = line_res {
                        let _ = log_tx_err.send(line);
                    }
                }
            });
        }

        // Monitor process exit
        let child_arc = Arc::clone(&self.active_child);
        let state_exit = Arc::clone(&self.state);
        let status_exit = Arc::clone(&self.status_message);
        let temp_dir_exit = Arc::clone(&self.active_temp_dir);
        let start_time_exit = Arc::clone(&self.connection_start_time);
        let log_exit = self.log_sender.clone();

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(400));
                let mut guard = child_arc.lock().unwrap();
                if let Some(ref mut proc) = *guard {
                    match proc.try_wait() {
                        Ok(Some(status)) => {
                            let mut st = state_exit.lock().unwrap();
                            if *st != VpnState::Disconnecting && *st != VpnState::Disconnected {
                                if !status.success() {
                                    *st = VpnState::Error;
                                    *status_exit.lock().unwrap() = format!("OpenVPN exited with code {}", status.code().unwrap_or(-1));
                                } else {
                                    *st = VpnState::Disconnected;
                                    *status_exit.lock().unwrap() = "DISCONNECTED".to_string();
                                }
                            }
                            *start_time_exit.lock().unwrap() = None;
                            *guard = None;
                            let _ = log_exit.send(format!("OpenVPN process exited ({})", status));
                            break;
                        }
                        Ok(None) => {}
                        Err(_) => break,
                    }
                } else {
                    break;
                }
            }

            // Cleanup temp dir
            let mut td_guard = temp_dir_exit.lock().unwrap();
            if let Some(ref path) = *td_guard {
                let _ = fs::remove_dir_all(path);
            }
            *td_guard = None;
        });

        Ok(())
    }

    pub fn disconnect(&self) {
        if self.get_state() == VpnState::Disconnected {
            return;
        }

        {
            let mut st = self.state.lock().unwrap();
            *st = VpnState::Disconnecting;
            *self.status_message.lock().unwrap() = "DISCONNECTING...".to_string();
        }

        let _ = self.log_sender.send("Stopping VPN tunnel and restoring network routes...".to_string());

        // 1. Send graceful shutdown command over management socket
        if let Some(port) = *self.active_mgmt_port.lock().unwrap() {
            if let Ok(mut stream) = TcpStream::connect(format!("127.0.0.1:{}", port)) {
                let _ = stream.write_all(b"signal SIGTERM\n");
                let _ = stream.flush();
                let _ = self.log_sender.send("Sent graceful termination signal (SIGTERM) via management socket.".to_string());
            }
        }

        // 2. Wait up to 2.5s for process to finish
        let start = Instant::now();
        loop {
            thread::sleep(Duration::from_millis(100));
            let mut guard = self.active_child.lock().unwrap();
            if let Some(ref mut proc) = *guard {
                if let Ok(Some(_)) = proc.try_wait() {
                    *guard = None;
                    break;
                }
            } else {
                break;
            }

            if start.elapsed() > Duration::from_millis(2500) {
                // Fallback to taskkill /F
                if let Some(ref mut proc) = *guard {
                    let _ = Command::new("taskkill")
                        .args(["/F", "/PID", &proc.id().to_string(), "/T"])
                        .creation_flags(CREATE_NO_WINDOW)
                        .status();
                }
                *guard = None;
                break;
            }
        }

        // 3. Purge any stale /1 routes from routing table
        purge_stale_routes();

        // 4. Clean temp directory
        let mut td = self.active_temp_dir.lock().unwrap();
        if let Some(ref path) = *td {
            let _ = fs::remove_dir_all(path);
        }
        *td = None;

        *self.connection_start_time.lock().unwrap() = None;
        *self.active_mgmt_port.lock().unwrap() = None;

        {
            let mut st = self.state.lock().unwrap();
            *st = VpnState::Disconnected;
            *self.status_message.lock().unwrap() = "DISCONNECTED".to_string();
        }

        let _ = self.log_sender.send("VPN Disconnected. System default gateway restored.".to_string());
    }

    pub fn export_config(&self, server: &VpnServer, destination: &Path) -> Result<(), String> {
        let raw = server
            .get_decoded_config()
            .ok_or_else(|| "Failed to decode OpenVPN profile.".to_string())?;

        let mut out = format!(
            "# ========================================================\n\
             # VPN Gate Server: {} ({})\n\
             # Speed: {:.1} Mbps | Ping: {} ms\n\
             # Credentials: Username='vpn', Password='vpn'\n\
             # ========================================================\n\n",
            server.country_long, server.ip, server.speed_mbps, server.ping
        );

        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.to_lowercase().starts_with("block-outside-dns") {
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }

        out.push_str("\n# === Injected Leak Protection & Resolvers ===\n");
        out.push_str("dhcp-option DNS 8.8.8.8\n");
        out.push_str("dhcp-option DNS 1.1.1.1\n");
        out.push_str("block-ipv6\n");

        fs::write(destination, out).map_err(|e| format!("Failed to write export file: {}", e))
    }
}

pub fn find_openvpn_binary() -> Option<PathBuf> {
    let candidates = [
        r"C:\Program Files\OpenVPN\bin\openvpn.exe",
        r"C:\Program Files (x86)\OpenVPN\bin\openvpn.exe",
    ];

    for path_str in candidates {
        let p = PathBuf::from(path_str);
        if p.exists() {
            return Some(p);
        }
    }

    if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
        let local_p = PathBuf::from(appdata).join(r"Programs\OpenVPN\bin\openvpn.exe");
        if local_p.exists() {
            return Some(local_p);
        }
    }

    None
}

pub fn is_running_as_admin() -> bool {
    let output = Command::new("net")
        .args(["session"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

pub fn restart_as_admin() {
    if let Ok(exe_path) = std::env::current_exe() {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let exe_wide: Vec<u16> = exe_path.as_os_str().encode_wide().chain(Some(0)).collect();
        let verb_wide: Vec<u16> = OsStr::new("runas").encode_wide().chain(Some(0)).collect();

        unsafe {
            windows_sys::Win32::UI::Shell::ShellExecuteW(
                std::ptr::null_mut(),
                verb_wide.as_ptr(),
                exe_wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1, // SW_SHOWNORMAL
            );
        }
        std::process::exit(0);
    }
}

pub fn purge_stale_routes() {
    for _ in 0..8 {
        let status = Command::new("route.exe")
            .args(["delete", "0.0.0.0", "mask", "128.0.0.0"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
        if let Ok(s) = status {
            if !s.success() {
                break;
            }
        }
    }
    for _ in 0..8 {
        let status = Command::new("route.exe")
            .args(["delete", "128.0.0.0", "mask", "128.0.0.0"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
        if let Ok(s) = status {
            if !s.success() {
                break;
            }
        }
    }

    let _ = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Remove-NetRoute -DestinationPrefix '0.0.0.0/1' -Confirm:$false -ErrorAction SilentlyContinue; Remove-NetRoute -DestinationPrefix '128.0.0.0/1' -Confirm:$false -ErrorAction SilentlyContinue",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status();

    flush_dns();
}

pub fn flush_dns() {
    let _ = Command::new("ipconfig.exe")
        .args(["/flushdns"])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

fn get_available_port() -> u16 {
    if let Ok(listener) = TcpListener::bind("127.0.0.1:0") {
        if let Ok(addr) = listener.local_addr() {
            return addr.port();
        }
    }
    25340
}

pub fn trim_working_set() {
    unsafe {
        let process = windows_sys::Win32::System::Threading::GetCurrentProcess();
        windows_sys::Win32::System::ProcessStatus::K32EmptyWorkingSet(process);
    }
}

pub fn get_working_set_bytes() -> usize {
    unsafe {
        let process = windows_sys::Win32::System::Threading::GetCurrentProcess();
        let mut pmc: windows_sys::Win32::System::ProcessStatus::PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        pmc.cb = std::mem::size_of::<windows_sys::Win32::System::ProcessStatus::PROCESS_MEMORY_COUNTERS>() as u32;
        if windows_sys::Win32::System::ProcessStatus::K32GetProcessMemoryInfo(process, &mut pmc, pmc.cb) != 0 {
            pmc.WorkingSetSize as usize
        } else {
            0
        }
    }
}
