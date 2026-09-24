#![windows_subsystem = "windows"]

mod models;
mod openvpn;
mod ui;
mod vpngate;

use ui::VpnGateApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([960.0, 600.0])
            .with_title("VPN Gate Studio • Rust Edition")
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "VPN Gate Studio",
        options,
        Box::new(|cc| Ok(Box::new(VpnGateApp::new(cc)))),
    )
}
