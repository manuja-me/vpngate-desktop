using System;
using System.Collections.Generic;
using System.Text;

namespace VpnGate.Desktop.Models
{
    public class VpnServer
    {
        public string HostName { get; set; } = string.Empty;
        public string IP { get; set; } = string.Empty;
        public int Score { get; set; }
        public int Ping { get; set; }
        public double SpeedMbps { get; set; }
        public string CountryLong { get; set; } = string.Empty;
        public string CountryShort { get; set; } = string.Empty;
        public int NumVpnSessions { get; set; }
        public long Uptime { get; set; }
        public string Operator { get; set; } = string.Empty;
        public string ConfigBase64 { get; set; } = string.Empty;
        public string Proto { get; set; } = "UDP";
        public int Port { get; set; } = 1194;

        public string Flag => GetFlag(CountryShort);

        public string DisplayTitle => $"{Flag} {CountryLong}";
        public string DisplaySubtitle => $"{IP}:{Port} • {Proto} • {NumVpnSessions} sessions";
        public string DisplaySpeed => $"{SpeedMbps:F1} Mbps";
        public string DisplayPing => $"{Ping} ms";

        public string PingColor => Ping switch
        {
            <= 60 => "#FFFFFF",
            <= 150 => "#A3A3A3",
            _ => "#737373"
        };

        public string GetDecodedConfig()
        {
            try
            {
                if (string.IsNullOrWhiteSpace(ConfigBase64)) return string.Empty;
                var bytes = Convert.FromBase64String(ConfigBase64);
                return Encoding.UTF8.GetString(bytes);
            }
            catch
            {
                return string.Empty;
            }
        }

        private static string GetFlag(string code) => code.ToUpperInvariant() switch
        {
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
            _ => "🌐"
        };
    }
}
