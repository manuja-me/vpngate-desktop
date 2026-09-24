use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use crate::models::VpnServer;

const API_URL: &str = "https://www.vpngate.net/api/iphone/";

pub fn get_cache_path() -> PathBuf {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(local_app_data).join("VpnGateDesktop");
    let _ = fs::create_dir_all(&dir);
    dir.join("cache.csv")
}

pub fn load_cached_servers() -> Vec<VpnServer> {
    let path = get_cache_path();
    if path.exists() {
        if let Ok(raw) = fs::read_to_string(&path) {
            return parse_csv(&raw);
        }
    }
    Vec::new()
}

pub fn fetch_servers(force_refresh: bool) -> Result<Vec<VpnServer>, String> {
    let cache_path = get_cache_path();
    if !force_refresh && cache_path.exists() {
        let cached = load_cached_servers();
        if !cached.is_empty() {
            return Ok(cached);
        }
    }

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .build();

    let resp = agent
        .get(API_URL)
        .call()
        .map_err(|e| format!("Failed to reach VPN Gate API: {}", e))?;

    let text = resp
        .into_string()
        .map_err(|e| format!("Failed to read response stream: {}", e))?;

    if text.trim().is_empty() {
        return Err("VPN Gate returned an empty response.".to_string());
    }

    // Save to local cache
    let _ = fs::write(&cache_path, &text);

    let parsed = parse_csv(&text);
    if parsed.is_empty() {
        Err("Failed to parse servers from API response.".to_string())
    } else {
        Ok(parsed)
    }
}

pub fn parse_csv(raw: &str) -> Vec<VpnServer> {
    let mut servers = Vec::new();
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(raw.as_bytes());

    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };

        if record.len() < 15 {
            continue;
        }

        let host = record.get(0).unwrap_or_default().trim();
        if host.is_empty() || host.starts_with('*') || host.starts_with('#') {
            continue;
        }

        let ip = record.get(1).unwrap_or_default().trim().to_string();
        if ip.is_empty() {
            continue;
        }

        let score: u32 = record.get(2).unwrap_or_default().trim().parse().unwrap_or(0);
        let ping: u32 = record.get(3).unwrap_or_default().trim().parse().unwrap_or(999);
        let speed_bps: u64 = record.get(4).unwrap_or_default().trim().parse().unwrap_or(0);
        let speed_mbps = (speed_bps as f64) / 1_000_000.0;

        let country_long = record.get(5).unwrap_or_default().trim().to_string();
        let country_short = record.get(6).unwrap_or_default().trim().to_string();
        let num_sessions: u32 = record.get(7).unwrap_or_default().trim().parse().unwrap_or(0);
        let uptime: u64 = record.get(8).unwrap_or_default().trim().parse().unwrap_or(0);
        let operator = record.get(12).unwrap_or_default().trim().to_string();
        let config_base64 = record.get(14).unwrap_or_default().trim().to_string();

        let mut proto = "UDP".to_string();
        let mut port = 1194;

        // Quick heuristic to inspect proto & port from config if possible
        let dummy = VpnServer { config_base64: config_base64.clone(), ..Default::default() };
        if let Some(decoded) = dummy.get_decoded_config() {
            for line in decoded.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("proto ") {
                    let p = trimmed.trim_start_matches("proto ").trim().to_uppercase();
                    if p.starts_with("TCP") {
                        proto = "TCP".to_string();
                    }
                } else if trimmed.starts_with("port ") {
                    if let Ok(pt) = trimmed.trim_start_matches("port ").trim().parse::<u16>() {
                        port = pt;
                    }
                }
            }
        }

        servers.push(VpnServer {
            host_name: host.to_string(),
            ip,
            score,
            ping,
            speed_mbps,
            country_long,
            country_short,
            num_sessions,
            uptime,
            operator,
            config_base64,
            proto,
            port,
        });
    }

    servers
}

#[allow(dead_code)]
pub fn filter_and_sort(
    servers: &[VpnServer],
    search: &str,
    country: &str,
    sort_by: &str,
) -> Vec<VpnServer> {
    let search_lower = search.trim().to_lowercase();
    let country_is_all = country.eq_ignore_ascii_case("all") || country.starts_with("ALL");

    let mut filtered: Vec<VpnServer> = servers
        .iter()
        .filter(|s| {
            if !country_is_all && !s.country_long.eq_ignore_ascii_case(country) && !s.country_short.eq_ignore_ascii_case(country) {
                return false;
            }
            if !search_lower.is_empty() {
                let matches_ip = s.ip.contains(&search_lower);
                let matches_country = s.country_long.to_lowercase().contains(&search_lower);
                let matches_host = s.host_name.to_lowercase().contains(&search_lower);
                let matches_operator = s.operator.to_lowercase().contains(&search_lower);
                return matches_ip || matches_country || matches_host || matches_operator;
            }
            true
        })
        .cloned()
        .collect();

    match sort_by {
        "ping" => filtered.sort_by(|a, b| a.ping.cmp(&b.ping)),
        "sessions" => filtered.sort_by(|a, b| b.num_sessions.cmp(&a.num_sessions)),
        "score" => filtered.sort_by(|a, b| b.score.cmp(&a.score)),
        _ => filtered.sort_by(|a, b| b.speed_mbps.partial_cmp(&a.speed_mbps).unwrap_or(std::cmp::Ordering::Equal)),
    }

    filtered
}
