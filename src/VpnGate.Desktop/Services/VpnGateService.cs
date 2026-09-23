using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Text;
using System.Threading.Tasks;
using VpnGate.Desktop.Models;

namespace VpnGate.Desktop.Services
{
    public class VpnGateService
    {
        private const string ApiUrl = "https://www.vpngate.net/api/iphone/";
        private static readonly HttpClient HttpClient = new HttpClient { Timeout = TimeSpan.FromSeconds(15) };
        private readonly string _cacheFile;

        public VpnGateService()
        {
            var appData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
            var dir = Path.Combine(appData, "VpnGateDesktop");
            Directory.CreateDirectory(dir);
            _cacheFile = Path.Combine(dir, "cache.csv");
        }

        public List<VpnServer> LoadCachedServers()
        {
            if (File.Exists(_cacheFile))
            {
                try
                {
                    var raw = File.ReadAllText(_cacheFile);
                    return ParseCsv(raw);
                }
                catch { }
            }
            return new List<VpnServer>();
        }

        public async Task<List<VpnServer>> FetchServersAsync(bool forceRefresh = false)
        {
            string rawCsv = string.Empty;

            // Try cached file if not force refreshing
            if (!forceRefresh && File.Exists(_cacheFile))
            {
                try
                {
                    var fileInfo = new FileInfo(_cacheFile);
                    if (DateTime.UtcNow - fileInfo.LastWriteTimeUtc < TimeSpan.FromMinutes(10))
                    {
                        rawCsv = await File.ReadAllTextAsync(_cacheFile);
                    }
                }
                catch { }
            }

            // Fetch from network if cache is empty or stale
            if (string.IsNullOrWhiteSpace(rawCsv))
            {
                try
                {
                    HttpClient.DefaultRequestHeaders.UserAgent.ParseAdd("Mozilla/5.0 (Windows NT 10.0; Win64; x64) VPN-Gate-Desktop/1.0");
                    rawCsv = await HttpClient.GetStringAsync(ApiUrl);
                    if (!string.IsNullOrWhiteSpace(rawCsv))
                    {
                        await File.WriteAllTextAsync(_cacheFile, rawCsv);
                    }
                }
                catch
                {
                    // Network failed, attempt cache fallback
                    if (File.Exists(_cacheFile))
                    {
                        rawCsv = await File.ReadAllTextAsync(_cacheFile);
                    }
                    else
                    {
                        throw;
                    }
                }
            }

            return ParseCsv(rawCsv);
        }

        public static List<VpnServer> ParseCsv(string rawCsv)
        {
            var list = new List<VpnServer>();
            if (string.IsNullOrWhiteSpace(rawCsv)) return list;

            using var reader = new StringReader(rawCsv);
            string? line;
            string[]? headers = null;

            while ((line = reader.ReadLine()) != null)
            {
                line = line.Trim();
                if (string.IsNullOrEmpty(line) || line.StartsWith("*")) continue;

                if (headers == null)
                {
                    // Fix leading '#' on header
                    if (line.StartsWith("#")) line = line.Substring(1);
                    headers = line.Split(',');
                    continue;
                }

                var parts = line.Split(',');
                if (parts.Length < 15) continue;

                try
                {
                    var configB64 = parts[14].Trim();
                    if (string.IsNullOrEmpty(configB64)) continue;

                    long.TryParse(parts[4], out var rawSpeed);
                    int.TryParse(parts[3], out var ping);
                    int.TryParse(parts[2], out var score);
                    int.TryParse(parts[7], out var sessions);
                    long.TryParse(parts[8], out var uptime);

                    var proto = "UDP";
                    var port = 1194;

                    // Inspect config for protocol and port
                    try
                    {
                        var decoded = Encoding.UTF8.GetString(Convert.FromBase64String(configB64));
                        foreach (var cfgLine in decoded.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.RemoveEmptyEntries))
                        {
                            var trimmed = cfgLine.Trim();
                            if (trimmed.StartsWith("proto ", StringComparison.OrdinalIgnoreCase))
                            {
                                proto = trimmed.Substring(6).Trim().ToUpperInvariant();
                            }
                            else if (trimmed.StartsWith("remote ", StringComparison.OrdinalIgnoreCase))
                            {
                                var remoteParts = trimmed.Split(' ', StringSplitOptions.RemoveEmptyEntries);
                                if (remoteParts.Length >= 3 && int.TryParse(remoteParts[2], out var p))
                                {
                                    port = p;
                                }
                            }
                        }
                    }
                    catch { }

                    list.Add(new VpnServer
                    {
                        HostName = parts[0],
                        IP = parts[1],
                        Score = score,
                        Ping = ping,
                        SpeedMbps = Math.Round(rawSpeed / 1_000_000.0, 2),
                        CountryLong = parts[5],
                        CountryShort = parts[6],
                        NumVpnSessions = sessions,
                        Uptime = uptime,
                        Operator = parts[12],
                        ConfigBase64 = configB64,
                        Proto = proto,
                        Port = port
                    });
                }
                catch { }
            }

            return list;
        }

        public static List<VpnServer> FilterAndSort(
            IEnumerable<VpnServer> servers,
            string search = "",
            string country = "All",
            string sortBy = "speed")
        {
            var query = servers.AsEnumerable();

            if (!string.IsNullOrWhiteSpace(country) && !country.Equals("All", StringComparison.OrdinalIgnoreCase))
            {
                query = query.Where(s => s.CountryLong.Equals(country, StringComparison.OrdinalIgnoreCase) ||
                                         s.CountryShort.Equals(country, StringComparison.OrdinalIgnoreCase));
            }

            if (!string.IsNullOrWhiteSpace(search))
            {
                var q = search.Trim();
                query = query.Where(s => s.CountryLong.Contains(q, StringComparison.OrdinalIgnoreCase) ||
                                         s.CountryShort.Contains(q, StringComparison.OrdinalIgnoreCase) ||
                                         s.IP.Contains(q, StringComparison.OrdinalIgnoreCase) ||
                                         s.HostName.Contains(q, StringComparison.OrdinalIgnoreCase) ||
                                         s.Operator.Contains(q, StringComparison.OrdinalIgnoreCase));
            }

            return (sortBy.ToLowerInvariant() switch
            {
                "ping" => query.OrderBy(s => s.Ping > 0 ? s.Ping : 9999),
                "sessions" => query.OrderByDescending(s => s.NumVpnSessions),
                "score" => query.OrderByDescending(s => s.Score),
                _ => query.OrderByDescending(s => s.SpeedMbps)
            }).ToList();
        }
    }
}
