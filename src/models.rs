use base64::Engine;

#[allow(dead_code)]
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VpnServer {
    pub host_name: String,
    pub ip: String,
    pub score: u32,
    pub ping: u32,
    pub speed_mbps: f64,
    pub country_long: String,
    pub country_short: String,
    pub num_sessions: u32,
    pub uptime: u64,
    pub operator: String,
    pub config_base64: String,
    pub proto: String,
    pub port: u16,
}

impl VpnServer {
    #[allow(dead_code)]
    pub fn flag(&self) -> &'static str {
        get_flag(&self.country_short)
    }

    pub fn get_decoded_config(&self) -> Option<String> {
        if self.config_base64.trim().is_empty() {
            return None;
        }
        let engine = base64::engine::general_purpose::STANDARD;
        let bytes = engine.decode(self.config_base64.trim()).ok()?;
        String::from_utf8(bytes).ok()
    }
}

#[allow(dead_code)]
pub fn get_flag(code: &str) -> &'static str {
    match code.to_uppercase().as_str() {
        "JP" => "🇯🇵",
        "US" => "🇺🇸",
        "KR" => "🇰🇷",
        "GB" => "🇬🇧",
        "DE" => "🇩🇪",
        "CA" => "🇨🇦",
        "FR" => "🇫🇷",
        "SG" => "🇸🇬",
        "AU" => "🇦🇺",
        "NL" => "🇳🇱",
        "RU" => "🇷🇺",
        "TH" => "🇹🇭",
        "VN" => "🇻🇳",
        "TW" => "🇹🇼",
        "HK" => "🇭🇰",
        "IN" => "🇮🇳",
        "BR" => "🇧🇷",
        "IT" => "🇮🇹",
        "ES" => "🇪🇸",
        "PL" => "🇵🇱",
        "UA" => "🇺🇦",
        "ID" => "🇮🇩",
        "MY" => "🇲🇾",
        "PH" => "🇵🇭",
        "SE" => "🇸🇪",
        "CH" => "🇨🇭",
        "NO" => "🇳🇴",
        "FI" => "🇫🇮",
        "TR" => "🇹🇷",
        "ZA" => "🇿🇦",
        _ => "🌐",
    }
}
