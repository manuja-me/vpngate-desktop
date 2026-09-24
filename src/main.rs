#![windows_subsystem = "windows"]

use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

mod models;
mod openvpn;
mod vpngate;

#[derive(Debug)]
enum UserEvent {
    VpnLog(String),
    VpnStatus(String, u64),
    ServersLoaded(String),
    AdminStatus(bool),
    MemoryTelemetry(f64),
    #[allow(dead_code)]
    EvaluateScript(String),
}

#[derive(serde::Deserialize)]
struct IpcMessage {
    cmd: String,
    #[serde(default)]
    payload: serde_json::Value,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let openvpn_mgr = Arc::new(openvpn::OpenVpnManager::new());
    let exit_openvpn = Arc::clone(&openvpn_mgr);

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("VPN Gate Studio • Monochrome Edition")
        .with_inner_size(LogicalSize::new(1080.0, 720.0))
        .with_min_inner_size(LogicalSize::new(960.0, 620.0))
        .build(&event_loop)?;

    // Prepare self-contained HTML
    let raw_html = include_str!("../ui/index.html");
    let raw_css = include_str!("../ui/style.css");
    let raw_js = include_str!("../ui/app.js");

    let complete_html = raw_html
        .replace("<link rel=\"stylesheet\" href=\"style.css\">", "")
        .replace("/* INJECT_STYLE */", raw_css)
        .replace("<script src=\"app.js\"></script>", "")
        .replace("/* INJECT_SCRIPT */", raw_js);

    // Setup IPC handler
    let ipc_proxy = proxy.clone();
    let ipc_openvpn = Arc::clone(&openvpn_mgr);

    let webview = WebViewBuilder::new()
        .with_html(&complete_html)
        .with_ipc_handler(move |request| {
            let body = request.body().to_string();
            if let Ok(msg) = serde_json::from_str::<IpcMessage>(&body) {
                match msg.cmd.as_str() {
                    "init" => {
                        let is_admin = openvpn::is_running_as_admin();
                        let _ = ipc_proxy.send_event(UserEvent::AdminStatus(is_admin));

                        // Load cached servers immediately
                        let cached = vpngate::load_cached_servers();
                        if !cached.is_empty() {
                            let _ = ipc_proxy.send_event(UserEvent::VpnLog(format!(
                                "[CACHE] Loaded {} servers from local cache.",
                                cached.len()
                            )));
                            if let Ok(json) = serde_json::to_string(&cached) {
                                let _ = ipc_proxy.send_event(UserEvent::ServersLoaded(json));
                            }
                        } else {
                            // If no cache, spawn auto-fetch
                            let p = ipc_proxy.clone();
                            thread::spawn(move || {
                                let _ = p.send_event(UserEvent::VpnLog(
                                    "[SYSTEM] No local cache found. Contacting VPN Gate API...".into(),
                                ));
                                match vpngate::fetch_servers(false) {
                                    Ok(srvs) => {
                                        let _ = p.send_event(UserEvent::VpnLog(format!(
                                            "[SYSTEM] Retrieved {} live relays.",
                                            srvs.len()
                                        )));
                                        if let Ok(json) = serde_json::to_string(&srvs) {
                                            let _ = p.send_event(UserEvent::ServersLoaded(json));
                                        }
                                    }
                                    Err(e) => {
                                        let _ = p.send_event(UserEvent::VpnLog(format!(
                                            "[ERROR] Initial API fetch failed: {}",
                                            e
                                        )));
                                    }
                                }
                            });
                        }

                        let mb = openvpn::get_working_set_bytes() as f64 / 1_048_576.0;
                        let _ = ipc_proxy.send_event(UserEvent::MemoryTelemetry(mb));
                    }
                    "fetch_servers" => {
                        let p = ipc_proxy.clone();
                        let force = msg
                            .payload
                            .get("force")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);

                        thread::spawn(move || {
                            let _ = p.send_event(UserEvent::VpnLog(
                                "[API] Refreshing server matrix from VPN Gate...".into(),
                            ));
                            match vpngate::fetch_servers(force) {
                                Ok(srvs) => {
                                    let _ = p.send_event(UserEvent::VpnLog(format!(
                                        "[API] Successfully parsed {} relays.",
                                        srvs.len()
                                    )));
                                    if let Ok(json) = serde_json::to_string(&srvs) {
                                        let _ = p.send_event(UserEvent::ServersLoaded(json));
                                    }
                                }
                                Err(e) => {
                                    let _ = p.send_event(UserEvent::VpnLog(format!(
                                        "[ERROR] Refresh failed: {}",
                                        e
                                    )));
                                }
                            }
                        });
                    }
                    "connect" => {
                        if let Ok(server) = serde_json::from_value::<models::VpnServer>(msg.payload) {
                            let p = ipc_proxy.clone();
                            let ovpn = Arc::clone(&ipc_openvpn);
                            let _ = p.send_event(UserEvent::VpnStatus("Connecting".into(), 0));
                            let _ = p.send_event(UserEvent::VpnLog(format!(
                                "[CONNECT] Establishing tunnel to {} ({}:{})...",
                                server.country_long, server.ip, server.port
                            )));

                            thread::spawn(move || {
                                if let Err(e) = ovpn.connect(&server) {
                                    let _ = p.send_event(UserEvent::VpnLog(format!(
                                        "[ERROR] Connection initialization failed: {}",
                                        e
                                    )));
                                    let _ = p.send_event(UserEvent::VpnStatus("Error".into(), 0));
                                }
                            });
                        }
                    }
                    "disconnect" => {
                        let p = ipc_proxy.clone();
                        let ovpn = Arc::clone(&ipc_openvpn);
                        let _ = p.send_event(UserEvent::VpnStatus("Disconnecting".into(), 0));

                        thread::spawn(move || {
                            let _ = p.send_event(UserEvent::VpnLog("[DISCONNECT] Terminating OpenVPN tunnel...".into()));
                            let _ = ovpn.disconnect();
                            openvpn::purge_stale_routes();
                            openvpn::flush_dns();
                            openvpn::trim_working_set();
                            let _ = p.send_event(UserEvent::VpnLog("[DISCONNECT] Default routes restored. DNS cache flushed.".into()));
                            let _ = p.send_event(UserEvent::VpnStatus("Disconnected".into(), 0));
                        });
                    }
                    "export_config" => {
                        if let Ok(server) = serde_json::from_value::<models::VpnServer>(msg.payload) {
                            if let Some(config) = server.get_decoded_config() {
                                let p = ipc_proxy.clone();
                                thread::spawn(move || {
                                    let fname = format!(
                                        "vpngate_{}_{}.ovpn",
                                        server.country_short.to_lowercase(),
                                        server.ip.replace('.', "-")
                                    );
                                    if let Some(path) = rfd::FileDialog::new()
                                        .set_file_name(&fname)
                                        .add_filter("OpenVPN Profile", &["ovpn"])
                                        .save_file()
                                    {
                                        if let Err(e) = std::fs::write(&path, config) {
                                            let _ = p.send_event(UserEvent::VpnLog(format!(
                                                "[ERROR] Failed to save profile: {}",
                                                e
                                            )));
                                        } else {
                                            let _ = p.send_event(UserEvent::VpnLog(format!(
                                                "[EXPORT] Configuration saved to {:?}",
                                                path
                                            )));
                                        }
                                    }
                                });
                            }
                        }
                    }
                    "restart_as_admin" => {
                        openvpn::restart_as_admin();
                    }
                    "copy_clipboard" => {
                        if let Some(text) = msg.payload.get("text").and_then(|t| t.as_str()) {
                            if let Ok(mut cb) = arboard::Clipboard::new() {
                                let _ = cb.set_text(text);
                            }
                        }
                    }
                    _ => {}
                }
            }
        })
        .build(&window)?;

    // Spawn background worker to stream OpenVPN logs to Webview
    let rx = Arc::clone(&openvpn_mgr.log_receiver);
    let log_proxy = proxy.clone();
    thread::spawn(move || {
        loop {
            let msg = {
                let guard = rx.lock().unwrap();
                guard.recv()
            };
            match msg {
                Ok(line) => {
                    let _ = log_proxy.send_event(UserEvent::VpnLog(line));
                }
                Err(_) => break,
            }
        }
    });

    // Spawn state telemetry poller
    let poll_mgr = Arc::clone(&openvpn_mgr);
    let poll_proxy = proxy.clone();
    thread::spawn(move || {
        let mut last_state = openvpn::VpnState::Disconnected;
        loop {
            thread::sleep(Duration::from_millis(500));
            let cur_state = poll_mgr.get_state();
            let dur_secs = poll_mgr
                .get_connection_duration()
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let state_str = match cur_state {
                openvpn::VpnState::Disconnected => "Disconnected",
                openvpn::VpnState::Connecting => "Connecting",
                openvpn::VpnState::Connected => "Connected",
                openvpn::VpnState::Disconnecting => "Disconnecting",
                openvpn::VpnState::Error => "Error",
            };

            if cur_state != last_state || cur_state == openvpn::VpnState::Connected {
                let _ = poll_proxy.send_event(UserEvent::VpnStatus(state_str.to_string(), dur_secs));
                last_state = cur_state;
            }

            let mb = openvpn::get_working_set_bytes() as f64 / 1_048_576.0;
            let _ = poll_proxy.send_event(UserEvent::MemoryTelemetry(mb));
        }
    });

    // Main Tao Event Loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::EvaluateScript(script)) => {
                let _ = webview.evaluate_script(&script);
            }
            Event::UserEvent(UserEvent::VpnLog(log)) => {
                if let Ok(escaped) = serde_json::to_string(&log) {
                    let _ = webview.evaluate_script(&format!(
                        "window.onVpnLog && window.onVpnLog({})",
                        escaped
                    ));
                }
            }
            Event::UserEvent(UserEvent::VpnStatus(status, dur)) => {
                let _ = webview.evaluate_script(&format!(
                    "window.onVpnStatus && window.onVpnStatus('{}', {})",
                    status, dur
                ));
            }
            Event::UserEvent(UserEvent::ServersLoaded(json)) => {
                let _ = webview.evaluate_script(&format!(
                    "window.onServersLoaded && window.onServersLoaded({})",
                    json
                ));
            }
            Event::UserEvent(UserEvent::AdminStatus(is_admin)) => {
                let _ = webview.evaluate_script(&format!(
                    "window.onAdminStatus && window.onAdminStatus({})",
                    is_admin
                ));
            }
            Event::UserEvent(UserEvent::MemoryTelemetry(mb)) => {
                let _ = webview.evaluate_script(&format!(
                    "window.onMemoryTelemetry && window.onMemoryTelemetry({})",
                    mb
                ));
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                let _ = exit_openvpn.disconnect();
                openvpn::purge_stale_routes();
                openvpn::flush_dns();
                *control_flow = ControlFlow::Exit;
            }
            _ => (),
        }
    });
}
