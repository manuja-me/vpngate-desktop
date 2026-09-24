#![windows_subsystem = "windows"]

mod models;
mod openvpn;
mod theme;
mod ui;
mod vpngate;

use ui::VpnGateApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 740.0])
            .with_min_inner_size([1000.0, 640.0])
            .with_title("VPN Gate Studio • Monochrome Edition")
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "VPN Gate Studio",
        options,
        Box::new(|cc| Ok(Box::new(VpnGateApp::new(cc)))),
    )
}
