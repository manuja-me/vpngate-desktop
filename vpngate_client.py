import os
import io
import csv
import json
import base64
import logging
import requests
from dataclasses import dataclass, asdict
from typing import List, Optional, Dict, Any

logger = logging.getLogger("vpngate.client")

VPN_GATE_API_URL = "https://www.vpngate.net/api/iphone/"
CACHE_FILE = os.path.join(os.path.dirname(__file__), ".vpngate_cache.json")

# Country ISO code to Flag emoji mapping
COUNTRY_FLAGS: Dict[str, str] = {
    "JP": "🇯🇵", "US": "🇺🇸", "KR": "🇰🇷", "GB": "🇬🇧", "DE": "🇩🇪",
    "CA": "🇨🇦", "FR": "🇫🇷", "SG": "🇸🇬", "AU": "🇦🇺", "NL": "🇳🇱",
    "RU": "🇷🇺", "TH": "🇹🇭", "VN": "🇻🇳", "TW": "🇹🇼", "HK": "🇭🇰",
    "IN": "🇮🇳", "BR": "🇧🇷", "IT": "🇮🇹", "ES": "🇪🇸", "PL": "🇵🇱",
    "UA": "🇺🇦", "ID": "🇮🇩", "MY": "🇲🇾", "PH": "🇵🇭", "SE": "🇸🇪",
    "CH": "🇨🇭", "NO": "🇳🇴", "FI": "🇫🇮", "TR": "🇹🇷", "ZA": "🇿🇦"
}

def get_flag(country_code: str) -> str:
    """Returns flag emoji for ISO country code or default globe."""
    return COUNTRY_FLAGS.get(country_code.upper(), "🌐")

@dataclass
class VpnServer:
    hostname: str
    ip: str
    score: int
    ping: int                # Latency in ms
    speed: float             # Speed in Mbps
    country_long: str        # e.g. "Japan"
    country_short: str       # e.g. "JP"
    num_vpn_sessions: int
    uptime: int              # Uptime in seconds
    total_users: int
    total_traffic: int
    log_type: str
    operator: str
    message: str
    config_base64: str
    proto: str = "UDP"       # Detected protocol (UDP or TCP)
    port: int = 1194         # Detected port

    @property
    def flag(self) -> str:
        return get_flag(self.country_short)

    def get_ovpn_config(self) -> str:
        """Decodes the Base64 OpenVPN configuration into plaintext."""
        try:
            return base64.b64decode(self.config_base64).decode("utf-8", errors="ignore")
        except Exception as e:
            logger.error(f"Failed to decode config for {self.hostname}: {e}")
            return ""


class VpnGateClient:
    """Client for fetching, caching, and filtering VPN Gate servers."""

    def __init__(self, cache_file: str = CACHE_FILE):
        self.cache_file = cache_file
        self.servers: List[VpnServer] = []

    def fetch_servers(self, timeout: int = 15, force_refresh: bool = False) -> List[VpnServer]:
        """
        Fetches live server list from VPN Gate API with fallback to local cache.
        """
        raw_text = None
        if not force_refresh and os.path.exists(self.cache_file):
            try:
                # If cache is younger than 10 minutes, use it
                mtime = os.path.getmtime(self.cache_file)
                if (os.path.getmtime(self.cache_file) + 600) > os.path.getctime(self.cache_file):
                    pass
            except Exception:
                pass

        # Attempt network fetch
        try:
            headers = {"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) VPN-Gate-Desktop/1.0"}
            resp = requests.get(VPN_GATE_API_URL, headers=headers, timeout=timeout)
            if resp.status_code == 200 and resp.text:
                raw_text = resp.text
                # Save cache
                try:
                    with open(self.cache_file, "w", encoding="utf-8") as f:
                        f.write(raw_text)
                except Exception as ce:
                    logger.warning(f"Could not write cache file: {ce}")
        except Exception as e:
            logger.warning(f"Network fetch failed ({e}). Falling back to local cache.")
            if os.path.exists(self.cache_file):
                try:
                    with open(self.cache_file, "r", encoding="utf-8") as f:
                        raw_text = f.read()
                except Exception as ce:
                    logger.error(f"Failed to read cache file: {ce}")

        if not raw_text:
            raise RuntimeError("Unable to retrieve VPN Gate server list from network or cache.")

        self.servers = self._parse_csv(raw_text)
        return self.servers

    def _parse_csv(self, raw_csv: str) -> List[VpnServer]:
        """Parses the VPN Gate CSV payload."""
        lines = [line.strip() for line in raw_csv.splitlines() if line.strip() and not line.startswith("*")]
        if not lines:
            return []

        # Fix the leading '#' on the header line
        if lines[0].startswith("#"):
            lines[0] = lines[0][1:]

        reader = csv.DictReader(io.StringIO("\n".join(lines)))
        parsed: List[VpnServer] = []

        for row in reader:
            try:
                cfg_b64 = row.get("OpenVPN_ConfigData_Base64", "").strip()
                if not cfg_b64:
                    continue

                raw_speed = int(row.get("Speed", 0) or 0)
                speed_mbps = round(raw_speed / 1_000_000, 2)
                ping_ms = int(row.get("Ping", 0) or 0)
                score = int(row.get("Score", 0) or 0)
                uptime = int(row.get("Uptime", 0) or 0)
                sessions = int(row.get("NumVpnSessions", 0) or 0)
                users = int(row.get("TotalUsers", 0) or 0)
                traffic = int(row.get("TotalTraffic", 0) or 0)

                # Quick protocol / port scan from config
                proto = "UDP"
                port = 1194
                try:
                    decoded = base64.b64decode(cfg_b64).decode("utf-8", errors="ignore")
                    for line in decoded.splitlines():
                        line_s = line.strip()
                        if line_s.startswith("proto "):
                            proto = line_s.split()[1].upper()
                        elif line_s.startswith("remote "):
                            parts = line_s.split()
                            if len(parts) >= 3 and parts[2].isdigit():
                                port = int(parts[2])
                except Exception:
                    pass

                server = VpnServer(
                    hostname=row.get("HostName", "Unknown"),
                    ip=row.get("IP", "0.0.0.0"),
                    score=score,
                    ping=ping_ms,
                    speed=speed_mbps,
                    country_long=row.get("CountryLong", "Unknown"),
                    country_short=row.get("CountryShort", "XX"),
                    num_vpn_sessions=sessions,
                    uptime=uptime,
                    total_users=users,
                    total_traffic=traffic,
                    log_type=row.get("LogType", ""),
                    operator=row.get("Operator", ""),
                    message=row.get("Message", ""),
                    config_base64=cfg_b64,
                    proto=proto,
                    port=port
                )
                parsed.append(server)
            except Exception as row_err:
                logger.debug(f"Skipping malformed server row: {row_err}")

        return parsed

    def filter_and_sort(
        self,
        search_query: str = "",
        country: str = "All",
        sort_by: str = "speed"  # "speed", "ping", "score", "sessions"
    ) -> List[VpnServer]:
        """Filters and sorts servers according to UI controls."""
        filtered = self.servers

        # Country filter
        if country and country != "All":
            filtered = [s for s in filtered if s.country_long.lower() == country.lower() or s.country_short.upper() == country.upper()]

        # Search query (matches country, IP, hostname, or operator)
        if search_query:
            q = search_query.strip().lower()
            filtered = [
                s for s in filtered
                if q in s.country_long.lower()
                or q in s.country_short.lower()
                or q in s.ip
                or q in s.hostname.lower()
                or q in s.operator.lower()
            ]

        # Sorting
        if sort_by == "speed":
            filtered.sort(key=lambda s: s.speed, reverse=True)
        elif sort_by == "ping":
            filtered.sort(key=lambda s: s.ping if s.ping > 0 else 9999)
        elif sort_by == "score":
            filtered.sort(key=lambda s: s.score, reverse=True)
        elif sort_by == "sessions":
            filtered.sort(key=lambda s: s.num_vpn_sessions, reverse=True)

        return filtered

    def get_countries(self) -> Dict[str, int]:
        """Returns a dictionary of country names and their active server count."""
        counts: Dict[str, int] = {}
        for s in self.servers:
            counts[s.country_long] = counts.get(s.country_long, 0) + 1
        return dict(sorted(counts.items(), key=lambda item: item[1], reverse=True))
