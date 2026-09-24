use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use egui_extras::{Column, TableBuilder};
use crate::models::VpnServer;
use crate::openvpn::{is_running_as_admin, restart_as_admin, OpenVpnManager, VpnState};
use crate::vpngate::{fetch_servers, filter_and_sort, load_cached_servers};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ActiveTab {
    Relays,
    Diagnostics,
}

pub struct VpnGateApp {
    openvpn: Arc<OpenVpnManager>,
    all_servers: Vec<VpnServer>,
    filtered_servers: Vec<VpnServer>,
    selected_server: Option<VpnServer>,
    
    search_query: String,
    selected_country: String,
    sort_by: String,
    active_tab: ActiveTab,
    is_refreshing: bool,
    is_admin: bool,
    
    logs: Vec<String>,
    refresh_receiver: Option<Receiver<Result<Vec<VpnServer>, String>>>,
    elevation_prompt_open: bool,
    frame_count: u64,
    prev_state: VpnState,
}

impl VpnGateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure Swiss Minimalist Monochrome Theme
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(Color32::from_rgb(255, 255, 255));
        visuals.panel_fill = Color32::from_rgb(0, 0, 0);
        visuals.window_fill = Color32::from_rgb(0, 0, 0);
        visuals.extreme_bg_color = Color32::from_rgb(8, 8, 8);
        visuals.faint_bg_color = Color32::from_rgb(12, 12, 12);
        
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(0, 0, 0);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(31, 31, 31));
        
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(8, 8, 8);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(31, 31, 31));
        
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(20, 20, 20);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(60, 60, 60));
        
        visuals.widgets.active.bg_fill = Color32::from_rgb(255, 255, 255);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0, 0, 0));
        
        cc.egui_ctx.set_visuals(visuals);

        let openvpn = Arc::new(OpenVpnManager::new());
        let is_admin = is_running_as_admin();

        // 1. Purge any stale zombie routes on startup
        thread::spawn(|| {
            crate::openvpn::purge_stale_routes();
        });

        // 2. Load cached servers immediately (< 2ms)
        let cached = load_cached_servers();
        let filtered = filter_and_sort(&cached, "", "All", "speed");

        let mut app = Self {
            openvpn,
            all_servers: cached,
            filtered_servers: filtered,
            selected_server: None,
            search_query: String::new(),
            selected_country: "All".to_string(),
            sort_by: "speed".to_string(),
            active_tab: ActiveTab::Relays,
            is_refreshing: false,
            is_admin,
            logs: vec!["[System] Initialized VPN Gate Studio (Native Rust Edition)".to_string()],
            refresh_receiver: None,
            elevation_prompt_open: false,
            frame_count: 0,
            prev_state: VpnState::Disconnected,
        };

        // Select fastest server by default if available
        if let Some(first) = app.filtered_servers.first() {
            app.selected_server = Some(first.clone());
        }

        // 3. Trigger initial background fetch
        app.trigger_refresh(false);

        app
    }

    fn trigger_refresh(&mut self, force: bool) {
        if self.is_refreshing {
            return;
        }
        self.is_refreshing = true;
        let (tx, rx) = channel();
        self.refresh_receiver = Some(rx);

        thread::spawn(move || {
            let res = fetch_servers(force);
            let _ = tx.send(res);
        });
    }

    fn update_filtered(&mut self) {
        self.filtered_servers = filter_and_sort(
            &self.all_servers,
            &self.search_query,
            &self.selected_country,
            &self.sort_by,
        );
    }

    fn poll_background_events(&mut self) {
        // Poll incoming logs from OpenVPN
        if let Ok(rx) = self.openvpn.log_receiver.lock() {
            while let Ok(msg) = rx.try_recv() {
                if self.logs.len() > 1500 {
                    self.logs.drain(0..500);
                }
                self.logs.push(msg);
            }
        }

        // Automatic working set trimming on state transitions (e.g. Connected, Disconnected)
        let current_state = self.openvpn.get_state();
        if current_state != self.prev_state {
            self.prev_state = current_state;
            crate::openvpn::trim_working_set();
        }

        // Poll refresh task
        if let Some(ref rx) = self.refresh_receiver {
            if let Ok(res) = rx.try_recv() {
                self.is_refreshing = false;
                match res {
                    Ok(servers) => {
                        self.logs.push(format!("[Network] Successfully loaded {} servers.", servers.len()));
                        self.all_servers = servers;
                        self.update_filtered();
                        crate::openvpn::trim_working_set();
                    }
                    Err(e) => {
                        self.logs.push(format!("[Error] Refresh failed: {}", e));
                    }
                }
                self.refresh_receiver = None;
            }
        }
    }
}

impl eframe::App for VpnGateApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_background_events();
        
        // Post-launch working set compaction after initial OpenGL pipeline initialization
        self.frame_count = self.frame_count.saturating_add(1);
        if self.frame_count == 15 || self.frame_count == 45 {
            crate::openvpn::trim_working_set();
        }

        // Request repaints when connected or connecting for timer and logs
        if self.openvpn.get_state() != VpnState::Disconnected || self.is_refreshing {
            ctx.request_repaint_after(Duration::from_millis(250));
        }

        // Top Navigation Bar
        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(0, 0, 0)).stroke(Stroke::new(1.0_f32, Color32::from_rgb(31, 31, 31))))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    
                    // Brand Indicator
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(ui.cursor().min + Vec2::new(0.0, 4.0), Vec2::new(10.0, 10.0)),
                        0.0,
                        Color32::WHITE,
                    );
                    ui.add_space(16.0);
                    ui.label(RichText::new("VPN GATE").strong().size(12.0).color(Color32::WHITE));
                    ui.add_space(8.0);

                    egui::Frame::NONE
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(40, 40, 40)))
                        .inner_margin(egui::Margin::symmetric(4, 1))
                        .show(ui, |ui| {
                            ui.label(RichText::new("STUDIO / RUST").size(9.0).monospace().color(Color32::from_rgb(115, 115, 115)));
                        });
                    
                    ui.add_space(30.0);

                    // Navigation Tabs
                    let relays_active = self.active_tab == ActiveTab::Relays;
                    let diag_active = self.active_tab == ActiveTab::Diagnostics;

                    let relays_btn = egui::Button::new(
                        RichText::new("RELAYS")
                            .size(11.0)
                            .strong()
                            .monospace()
                            .color(if relays_active { Color32::BLACK } else { Color32::from_rgb(115, 115, 115) }),
                    )
                    .fill(if relays_active { Color32::WHITE } else { Color32::TRANSPARENT })
                    .stroke(Stroke::NONE);

                    if ui.add(relays_btn).clicked() {
                        self.active_tab = ActiveTab::Relays;
                    }

                    let diag_btn = egui::Button::new(
                        RichText::new("DIAGNOSTICS")
                            .size(11.0)
                            .strong()
                            .monospace()
                            .color(if diag_active { Color32::BLACK } else { Color32::from_rgb(115, 115, 115) }),
                    )
                    .fill(if diag_active { Color32::WHITE } else { Color32::TRANSPARENT })
                    .stroke(Stroke::NONE);

                    if ui.add(diag_btn).clicked() {
                        self.active_tab = ActiveTab::Diagnostics;
                    }

                    // Right Aligned Status & Elevation Indicator
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(14.0);

                        // Elevation Status
                        if self.is_admin {
                            ui.label(RichText::new("● ELEVATED").size(10.0).monospace().color(Color32::from_rgb(163, 163, 163)));
                        } else {
                            if ui.button(RichText::new("⚠ RESTART AS ADMIN").size(10.0).monospace().color(Color32::BLACK))
                                .on_hover_text("Administrator elevation is required to modify Windows default routing tables")
                                .clicked()
                            {
                                restart_as_admin();
                            }
                        }

                        ui.add_space(12.0);

                        // Engine Status
                        let state = self.openvpn.get_state();
                        let (dot, text, color) = match state {
                            VpnState::Connected => ("●", "CONNECTED", Color32::WHITE),
                            VpnState::Connecting => ("◌", "CONNECTING...", Color32::from_rgb(163, 163, 163)),
                            VpnState::Disconnecting => ("◌", "DISCONNECTING...", Color32::from_rgb(163, 163, 163)),
                            VpnState::Error => ("✖", "ERROR", Color32::WHITE),
                            VpnState::Disconnected => ("○", "READY", Color32::from_rgb(115, 115, 115)),
                        };

                        ui.label(RichText::new(format!("{} {}", dot, text)).size(10.0).monospace().color(color));
                    });
                });
                ui.add_space(8.0);
            });

        // Bottom Action Dock
        egui::TopBottomPanel::bottom("bottom_dock")
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(8, 8, 8)).stroke(Stroke::new(1.0_f32, Color32::from_rgb(31, 31, 31))))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);

                    // Selected Server Info
                    if let Some(ref server) = self.selected_server {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("{} {}", server.flag(), server.country_long)).size(13.0).strong().color(Color32::WHITE));
                                ui.label(RichText::new(format!("• {}:{}", server.ip, server.port)).size(11.0).monospace().color(Color32::from_rgb(163, 163, 163)));
                            });
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("{:.1} Mbps", server.speed_mbps)).size(10.0).monospace().color(Color32::WHITE));
                                ui.label(RichText::new(format!("• {} ms", server.ping)).size(10.0).monospace().color(Color32::from_rgb(163, 163, 163)));
                                ui.label(RichText::new(format!("• {} sessions", server.num_sessions)).size(10.0).monospace().color(Color32::from_rgb(115, 115, 115)));
                                
                                if let Some(dur) = self.openvpn.get_connection_duration() {
                                    let secs = dur.as_secs();
                                    let h = secs / 3600;
                                    let m = (secs % 3600) / 60;
                                    let s = secs % 60;
                                    ui.label(RichText::new(format!("• UP: {:02}:{:02}:{:02}", h, m, s)).size(10.0).monospace().strong().color(Color32::WHITE));
                                }
                            });
                        });
                    } else {
                        ui.label(RichText::new("SELECT A RELAY FROM THE TABLE TO CONNECT").size(11.0).monospace().color(Color32::from_rgb(115, 115, 115)));
                    }

                    // Action Buttons (Right)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);

                        let state = self.openvpn.get_state();
                        let has_selection = self.selected_server.is_some();

                        // Main Connect / Disconnect Action Button
                        match state {
                            VpnState::Connected => {
                                let btn = egui::Button::new(
                                    RichText::new("■ DISCONNECT FROM RELAY").size(12.0).strong().monospace().color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(14, 14, 14))
                                .stroke(Stroke::new(1.0_f32, Color32::WHITE))
                                .min_size(Vec2::new(200.0, 32.0));

                                if ui.add(btn).clicked() {
                                    self.openvpn.disconnect();
                                }
                            }
                            VpnState::Connecting => {
                                let btn = egui::Button::new(
                                    RichText::new("◌ CONNECTING...").size(12.0).strong().monospace().color(Color32::BLACK),
                                )
                                .fill(Color32::from_rgb(160, 160, 160))
                                .min_size(Vec2::new(180.0, 32.0));
                                ui.add_enabled(false, btn);
                            }
                            _ => {
                                let btn = egui::Button::new(
                                    RichText::new("⚡ CONNECT TO RELAY").size(12.0).strong().monospace().color(Color32::BLACK),
                                )
                                .fill(Color32::WHITE)
                                .min_size(Vec2::new(180.0, 32.0));

                                if ui.add_enabled(has_selection, btn).clicked() {
                                    if !self.is_admin {
                                        self.elevation_prompt_open = true;
                                    } else if let Some(ref server) = self.selected_server {
                                        if let Err(e) = self.openvpn.connect(server) {
                                            self.logs.push(format!("[Error] Connect failed: {}", e));
                                        }
                                    }
                                }
                            }
                        }

                        // Export Config Button
                        let export_btn = egui::Button::new(
                            RichText::new("EXPORT .OVPN").size(10.0).monospace().color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(18, 18, 18))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 45)))
                        .min_size(Vec2::new(100.0, 32.0));

                        if ui.add_enabled(has_selection, export_btn).clicked() {
                            if let Some(ref server) = self.selected_server {
                                let file_name = format!("vpngate_{}_{}.ovpn", server.country_short, server.ip);
                                if let Some(path) = rfd::FileDialog::new().set_file_name(&file_name).add_filter("OpenVPN Config", &["ovpn"]).save_file() {
                                    if let Err(e) = self.openvpn.export_config(server, &path) {
                                        self.logs.push(format!("[Error] Export failed: {}", e));
                                    } else {
                                        self.logs.push(format!("[Export] Saved configuration to {}", path.display()));
                                    }
                                }
                            }
                        }
                    });
                });
                ui.add_space(10.0);
            });

        // Main Center Content
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(0, 0, 0)))
            .show(ctx, |ui| {
                match self.active_tab {
                    ActiveTab::Relays => self.render_relays_view(ui),
                    ActiveTab::Diagnostics => self.render_diagnostics_view(ui),
                }
            });

        // Elevation Required Modal Window
        if self.elevation_prompt_open {
            egui::Window::new("ADMINISTRATOR PERMISSION REQUIRED")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.add_space(8.0);
                    ui.label("Windows requires Administrator privileges to modify system network gateways and route 100% of traffic through the VPN.");
                    ui.label("Without elevation, your real physical IP address will remain visible.");
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("RESTART AS ADMINISTRATOR").strong().color(Color32::BLACK)).clicked() {
                            self.elevation_prompt_open = false;
                            restart_as_admin();
                        }
                        if ui.button("CANCEL").clicked() {
                            self.elevation_prompt_open = false;
                        }
                    });
                });
        }
    }
}

impl VpnGateApp {
    fn render_relays_view(&mut self, ui: &mut egui::Ui) {
        // Stats & Filter Row
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(16.0);

            // Metrics Summary
            let unique_countries: std::collections::HashSet<_> = self.all_servers.iter().map(|s| &s.country_long).collect();
            let max_speed = self.all_servers.iter().map(|s| s.speed_mbps).fold(0.0, f64::max);

            ui.label(RichText::new(format!("{} ONLINE", self.all_servers.len())).size(11.0).monospace().color(Color32::WHITE));
            ui.label(RichText::new("•").size(11.0).color(Color32::from_rgb(50, 50, 50)));
            ui.label(RichText::new(format!("{} REGIONS", unique_countries.len())).size(11.0).monospace().color(Color32::from_rgb(163, 163, 163)));
            ui.label(RichText::new("•").size(11.0).color(Color32::from_rgb(50, 50, 50)));
            ui.label(RichText::new(format!("{:.1} Mbps PEAK", max_speed)).size(11.0).monospace().color(Color32::from_rgb(163, 163, 163)));
            ui.label(RichText::new("•").size(11.0).color(Color32::from_rgb(50, 50, 50)));
            let ram_mb = crate::openvpn::get_working_set_bytes() as f64 / (1024.0 * 1024.0);
            ui.label(RichText::new(format!("{:.1} MB RAM", ram_mb)).size(11.0).monospace().color(Color32::from_rgb(163, 163, 163)));

            // Filter Controls (Right-aligned)
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(16.0);

                // Refresh Button
                let refresh_text = if self.is_refreshing { "⏳ REFRESHING..." } else { "🔄 REFRESH" };
                if ui.button(RichText::new(refresh_text).size(10.0).monospace()).clicked() {
                    self.trigger_refresh(true);
                }

                // Sort Dropdown
                egui::ComboBox::from_id_salt("sort_by")
                    .selected_text(match self.sort_by.as_str() {
                        "ping" => "Lowest Ping",
                        "sessions" => "Most Sessions",
                        "score" => "Highest Score",
                        _ => "Fastest Speed",
                    })
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        changed |= ui.selectable_value(&mut self.sort_by, "speed".to_string(), "Fastest Speed").clicked();
                        changed |= ui.selectable_value(&mut self.sort_by, "ping".to_string(), "Lowest Ping").clicked();
                        changed |= ui.selectable_value(&mut self.sort_by, "sessions".to_string(), "Most Sessions").clicked();
                        changed |= ui.selectable_value(&mut self.sort_by, "score".to_string(), "Highest Score").clicked();
                        if changed {
                            self.update_filtered();
                        }
                    });

                // Country Dropdown
                let mut countries: Vec<String> = self.all_servers.iter().map(|s| s.country_long.clone()).collect();
                countries.sort();
                countries.dedup();
                countries.insert(0, "All".to_string());

                egui::ComboBox::from_id_salt("country_filter")
                    .selected_text(format!("Region: {}", self.selected_country))
                    .show_ui(ui, |ui| {
                        for c in countries {
                            if ui.selectable_value(&mut self.selected_country, c.clone(), &c).clicked() {
                                self.update_filtered();
                            }
                        }
                    });

                // Search Bar
                let search_box = egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Search IP, Country...")
                    .desired_width(180.0);
                if ui.add(search_box).changed() {
                    self.update_filtered();
                }
            });
        });

        ui.add_space(10.0);

        // Swiss Virtualized Data Table
        let available_height = ui.available_height() - 10.0;
        let num_rows = self.filtered_servers.len();

        ui.push_id("relays_table", |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(180.0)) // Relay (Flag + Country)
                .column(Column::exact(140.0)) // IP
                .column(Column::exact(60.0))  // Proto
                .column(Column::exact(100.0)) // Speed
                .column(Column::exact(80.0))  // Ping
                .column(Column::remainder())  // Sessions
                .min_scrolled_height(available_height)
                .header(24.0, |mut header| {
                    header.col(|ui| { ui.label(RichText::new("RELAY REGION").size(10.0).monospace().color(Color32::from_rgb(115, 115, 115))); });
                    header.col(|ui| { ui.label(RichText::new("ENDPOINT IP").size(10.0).monospace().color(Color32::from_rgb(115, 115, 115))); });
                    header.col(|ui| { ui.label(RichText::new("PROTO").size(10.0).monospace().color(Color32::from_rgb(115, 115, 115))); });
                    header.col(|ui| { ui.label(RichText::new("THROUGHPUT").size(10.0).monospace().color(Color32::from_rgb(115, 115, 115))); });
                    header.col(|ui| { ui.label(RichText::new("PING").size(10.0).monospace().color(Color32::from_rgb(115, 115, 115))); });
                    header.col(|ui| { ui.label(RichText::new("SESSIONS").size(10.0).monospace().color(Color32::from_rgb(115, 115, 115))); });
                })
                .body(|body| {
                    body.rows(28.0, num_rows, |mut row| {
                        let idx = row.index();
                        if let Some(server) = self.filtered_servers.get(idx).cloned() {
                            let is_selected = self.selected_server.as_ref().map_or(false, |s| s.ip == server.ip);

                            row.col(|ui| {
                                if ui.selectable_label(is_selected, format!("{} {}", server.flag(), server.country_long)).clicked() {
                                    self.selected_server = Some(server.clone());
                                }
                            });
                            row.col(|ui| {
                                ui.label(RichText::new(format!("{}:{}", server.ip, server.port)).monospace().size(11.0).color(Color32::from_rgb(200, 200, 200)));
                            });
                            row.col(|ui| {
                                ui.label(RichText::new(&server.proto).monospace().size(10.0).color(Color32::from_rgb(163, 163, 163)));
                            });
                            row.col(|ui| {
                                ui.label(RichText::new(format!("{:.1} Mbps", server.speed_mbps)).monospace().size(11.0).strong().color(Color32::WHITE));
                            });
                            row.col(|ui| {
                                let ping_color = match server.ping {
                                    0..=60 => Color32::WHITE,
                                    61..=150 => Color32::from_rgb(163, 163, 163),
                                    _ => Color32::from_rgb(100, 100, 100),
                                };
                                ui.label(RichText::new(format!("{} ms", server.ping)).monospace().size(11.0).color(ping_color));
                            });
                            row.col(|ui| {
                                ui.label(RichText::new(format!("{} users", server.num_sessions)).monospace().size(10.0).color(Color32::from_rgb(115, 115, 115)));
                            });
                        }
                    });
                });
        });
    }

    fn render_diagnostics_view(&mut self, ui: &mut egui::Ui) {
        let ram_mb = crate::openvpn::get_working_set_bytes() as f64 / (1024.0 * 1024.0);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.label(RichText::new("OPENVPN ENGINE LOG OUTPUT").size(11.0).monospace().strong().color(Color32::WHITE));
            ui.label(RichText::new("•").size(11.0).color(Color32::from_rgb(50, 50, 50)));
            ui.label(RichText::new(format!("RAM: {:.1} MB", ram_mb)).size(11.0).monospace().color(Color32::from_rgb(163, 163, 163)));
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(16.0);
                if ui.button(RichText::new("🗑 CLEAR LOGS").size(10.0).monospace()).clicked() {
                    self.logs.clear();
                }
                if ui.button(RichText::new("📋 COPY TO CLIPBOARD").size(10.0).monospace()).clicked() {
                    let all_text = self.logs.join("\n");
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(all_text);
                    }
                }
                if ui.button(RichText::new("⚡ TRIM RAM").size(10.0).monospace()).on_hover_text("Flush unneeded working set memory back to Windows").clicked() {
                    crate::openvpn::trim_working_set();
                }
            });
        });

        ui.add_space(8.0);

        egui::Frame::NONE
            .fill(Color32::from_rgb(5, 5, 5))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(25, 25, 25)))
            .inner_margin(12.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        for line in &self.logs {
                            let color = if line.contains(">>>") {
                                Color32::WHITE
                            } else if line.contains("⚠️") || line.contains("Warning") {
                                Color32::from_rgb(220, 220, 220)
                            } else if line.contains("Error") || line.contains("Failed") {
                                Color32::from_rgb(255, 180, 180)
                            } else {
                                Color32::from_rgb(140, 140, 140)
                            };
                            ui.label(RichText::new(line).monospace().size(10.5).color(color));
                        }
                    });
            });
    }
}
