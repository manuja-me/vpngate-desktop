use std::fs;
use std::path::{Path, PathBuf};
use eframe::egui::Color32;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub bg_black: String,
    pub bg_surface: String,
    pub bg_card: String,
    pub bg_hover: String,
    pub border_hairline: String,
    pub border_subtle: String,
    pub border_focus: String,
    pub text_primary: String,
    pub text_secondary: String,
    pub text_muted: String,
    pub primary_btn_bg: String,
    pub primary_btn_fg: String,
    pub disconnect_btn_bg: String,
    pub disconnect_btn_border: String,
    pub disconnect_btn_fg: String,
    pub card_radius: u8,
    pub button_radius: u8,
    pub input_radius: u8,
    pub font_size_tiny: f32,
    pub font_size_small: f32,
    pub font_size_body: f32,
    pub font_size_title: f32,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            bg_black: "#000000".to_string(),
            bg_surface: "#080808".to_string(),
            bg_card: "#0D0D0D".to_string(),
            bg_hover: "#141414".to_string(),
            border_hairline: "#1F1F1F".to_string(),
            border_subtle: "#333333".to_string(),
            border_focus: "#FFFFFF".to_string(),
            text_primary: "#FFFFFF".to_string(),
            text_secondary: "#A3A3A3".to_string(),
            text_muted: "#737373".to_string(),
            primary_btn_bg: "#FFFFFF".to_string(),
            primary_btn_fg: "#000000".to_string(),
            disconnect_btn_bg: "#000000".to_string(),
            disconnect_btn_border: "#FFFFFF".to_string(),
            disconnect_btn_fg: "#FFFFFF".to_string(),
            card_radius: 2,
            button_radius: 2,
            input_radius: 2,
            font_size_tiny: 9.0,
            font_size_small: 10.5,
            font_size_body: 11.5,
            font_size_title: 14.5,
        }
    }
}

#[allow(dead_code)]
pub struct Theme {
    pub config: ThemeConfig,
    pub c_bg_black: Color32,
    pub c_bg_surface: Color32,
    pub c_bg_card: Color32,
    pub c_bg_hover: Color32,
    pub c_border_hairline: Color32,
    pub c_border_subtle: Color32,
    pub c_border_focus: Color32,
    pub c_text_primary: Color32,
    pub c_text_secondary: Color32,
    pub c_text_muted: Color32,
    pub c_primary_btn_bg: Color32,
    pub c_primary_btn_fg: Color32,
    pub c_disconnect_btn_bg: Color32,
    pub c_disconnect_btn_border: Color32,
    pub c_disconnect_btn_fg: Color32,
}

impl Theme {
    pub fn new(config: ThemeConfig) -> Self {
        Self {
            c_bg_black: parse_hex_color(&config.bg_black, Color32::from_rgb(0, 0, 0)),
            c_bg_surface: parse_hex_color(&config.bg_surface, Color32::from_rgb(8, 8, 8)),
            c_bg_card: parse_hex_color(&config.bg_card, Color32::from_rgb(13, 13, 13)),
            c_bg_hover: parse_hex_color(&config.bg_hover, Color32::from_rgb(20, 20, 20)),
            c_border_hairline: parse_hex_color(&config.border_hairline, Color32::from_rgb(31, 31, 31)),
            c_border_subtle: parse_hex_color(&config.border_subtle, Color32::from_rgb(51, 51, 51)),
            c_border_focus: parse_hex_color(&config.border_focus, Color32::from_rgb(255, 255, 255)),
            c_text_primary: parse_hex_color(&config.text_primary, Color32::from_rgb(255, 255, 255)),
            c_text_secondary: parse_hex_color(&config.text_secondary, Color32::from_rgb(163, 163, 163)),
            c_text_muted: parse_hex_color(&config.text_muted, Color32::from_rgb(115, 115, 115)),
            c_primary_btn_bg: parse_hex_color(&config.primary_btn_bg, Color32::from_rgb(255, 255, 255)),
            c_primary_btn_fg: parse_hex_color(&config.primary_btn_fg, Color32::from_rgb(0, 0, 0)),
            c_disconnect_btn_bg: parse_hex_color(&config.disconnect_btn_bg, Color32::from_rgb(0, 0, 0)),
            c_disconnect_btn_border: parse_hex_color(&config.disconnect_btn_border, Color32::from_rgb(255, 255, 255)),
            c_disconnect_btn_fg: parse_hex_color(&config.disconnect_btn_fg, Color32::from_rgb(255, 255, 255)),
            config,
        }
    }

    pub fn load_or_default() -> Self {
        let path = find_theme_file();
        if let Some(ref p) = path {
            if let Ok(content) = fs::read_to_string(p) {
                if let Ok(cfg) = serde_json::from_str::<ThemeConfig>(&content) {
                    return Self::new(cfg);
                }
            }
        }

        let default_cfg = ThemeConfig::default();
        // If not found, write default theme.json beside the binary so user can easily customize
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(dir) = exe_path.parent() {
                let target = dir.join("theme.json");
                if !target.exists() {
                    if let Ok(json_str) = serde_json::to_string_pretty(&default_cfg) {
                        let _ = fs::write(target, json_str);
                    }
                }
            }
        }

        Self::new(default_cfg)
    }
}

fn find_theme_file() -> Option<PathBuf> {
    // 1. Current working directory
    let cwd_path = Path::new("theme.json");
    if cwd_path.exists() {
        return Some(cwd_path.to_path_buf());
    }

    // 2. Next to executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let exe_theme = dir.join("theme.json");
            if exe_theme.exists() {
                return Some(exe_theme);
            }
        }
    }

    None
}

fn parse_hex_color(hex: &str, fallback: Color32) -> Color32 {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&clean[0..2], 16),
            u8::from_str_radix(&clean[2..4], 16),
            u8::from_str_radix(&clean[4..6], 16),
        ) {
            return Color32::from_rgb(r, g, b);
        }
    }
    fallback
}
