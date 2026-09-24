using System;
using System.Diagnostics;
using System.IO;
using System.Net;
using System.Net.Http;
using System.Net.Sockets;
using System.Text;
using System.Threading.Tasks;
using VpnGate.Desktop.Models;

namespace VpnGate.Desktop.Services
{
    public enum VpnState
    {
        Disconnected,
        Connecting,
        Connected,
        Disconnecting,
        Error
    }

    public class OpenVpnService
    {
        private const string MsiDownloadUrl = "https://swupdate.openvpn.org/community/releases/OpenVPN-2.7.7-I001-amd64.msi";
        private static readonly string[] SearchPaths = new[]
        {
            @"C:\Program Files\OpenVPN\bin\openvpn.exe",
            @"C:\Program Files (x86)\OpenVPN\bin\openvpn.exe",
            Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), @"Programs\OpenVPN\bin\openvpn.exe")
        };

        private Process? _process;
        private string? _activeTempDir;
        private int _managementPort;

        public VpnState State { get; private set; } = VpnState.Disconnected;
        public VpnServer? CurrentServer { get; private set; }

        public event Action<VpnState, string?>? StateChanged;
        public event Action<string>? LogReceived;

        public string? FindOpenVpnBinary()
        {
            foreach (var path in SearchPaths)
            {
                if (File.Exists(path)) return path;
            }

            // Check PATH environment variable
            var envPath = Environment.GetEnvironmentVariable("PATH") ?? string.Empty;
            foreach (var folder in envPath.Split(';', StringSplitOptions.RemoveEmptyEntries))
            {
                var candidate = Path.Combine(folder.Trim(), "openvpn.exe");
                if (File.Exists(candidate)) return candidate;
            }

            return null;
        }

        public bool IsEngineInstalled => FindOpenVpnBinary() != null;

        public async Task ConnectAsync(VpnServer server)
        {
            if (State is VpnState.Connected or VpnState.Connecting) return;

            var openvpnExe = FindOpenVpnBinary();
            if (openvpnExe == null)
            {
                SetState(VpnState.Error, "OpenVPN engine not found. Please install OpenVPN first.");
                return;
            }

            CurrentServer = server;
            SetState(VpnState.Connecting);
            Log($"Preparing tunnel for {server.Flag} {server.CountryLong} ({server.IP}:{server.Port})...");

            try
            {
                // Create temp directory for this connection session
                _activeTempDir = Path.Combine(Path.GetTempPath(), $"vpngate_{Guid.NewGuid():N}");
                Directory.CreateDirectory(_activeTempDir);

                var configPath = Path.Combine(_activeTempDir, "profile.ovpn");
                var authPath = Path.Combine(_activeTempDir, "auth.txt");

                // Write auth credentials (vpn / vpn)
                await File.WriteAllTextAsync(authPath, "vpn\nvpn\n");

                // Decode and sanitize config lines
                var rawConfig = server.GetDecodedConfig();
                if (string.IsNullOrWhiteSpace(rawConfig))
                {
                    SetState(VpnState.Error, "Failed to decode OpenVPN profile configuration.");
                    return;
                }

                var sb = new StringBuilder();
                using (var reader = new StringReader(rawConfig))
                {
                    string? line;
                    while ((line = reader.ReadLine()) != null)
                    {
                        var trimmed = line.Trim();
                        // Strip naked or conflicting auth-user-pass directives
                        if (trimmed.Equals("auth-user-pass", StringComparison.OrdinalIgnoreCase) ||
                            trimmed.StartsWith("auth-user-pass ", StringComparison.OrdinalIgnoreCase))
                        {
                            continue;
                        }
                        // Strip deprecated persist-key directive
                        if (trimmed.Equals("persist-key", StringComparison.OrdinalIgnoreCase))
                        {
                            continue;
                        }
                        // Strip any pre-existing directives that could fail or cause conflicts
                        if (trimmed.StartsWith("block-outside-dns", StringComparison.OrdinalIgnoreCase))
                        {
                            continue;
                        }
                        sb.AppendLine(line);
                    }
                }

                // Injected directives to guarantee DNS fallback and IPv6 leak prevention
                sb.AppendLine();
                sb.AppendLine("# === Leak Protection & Resolvers ===");
                sb.AppendLine("dhcp-option DNS 8.8.8.8");
                sb.AppendLine("dhcp-option DNS 1.1.1.1");
                sb.AppendLine("block-ipv6");

                await File.WriteAllTextAsync(configPath, sb.ToString());

                // Purge any stale zombie routes before establishing new tunnel
                PurgeStaleRoutes();

                _managementPort = GetAvailablePort();

                // Launch OpenVPN in working directory with relative paths (completely avoids backslash issues)
                var startInfo = new ProcessStartInfo
                {
                    FileName = openvpnExe,
                    Arguments = $"--config profile.ovpn --auth-user-pass auth.txt --management 127.0.0.1 {_managementPort} --verb 3",
                    WorkingDirectory = _activeTempDir,
                    UseShellExecute = false,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    CreateNoWindow = true
                };

                _process = new Process { StartInfo = startInfo, EnableRaisingEvents = true };

                _process.OutputDataReceived += (_, e) =>
                {
                    if (e.Data != null) HandleProcessOutput(e.Data);
                };
                _process.ErrorDataReceived += (_, e) =>
                {
                    if (e.Data != null) HandleProcessOutput(e.Data);
                };

                _process.Exited += (_, _) =>
                {
                    if (State != VpnState.Disconnecting && State != VpnState.Disconnected)
                    {
                        var code = _process?.ExitCode ?? 0;
                        if (code != 0)
                        {
                            SetState(VpnState.Error, $"OpenVPN exited with error code {code}.");
                        }
                        else
                        {
                            SetState(VpnState.Disconnected);
                        }
                    }
                    Cleanup();
                };

                Log($"Spawning OpenVPN engine (mgmt port {_managementPort}): {openvpnExe} ...");
                _process.Start();
                _process.BeginOutputReadLine();
                _process.BeginErrorReadLine();
            }
            catch (Exception ex)
            {
                SetState(VpnState.Error, ex.Message);
                Cleanup();
            }
        }

        private void HandleProcessOutput(string line)
        {
            Log(line);

            if (line.Contains("Initialization Sequence Completed", StringComparison.OrdinalIgnoreCase))
            {
                SetState(VpnState.Connected);
                Log(">>> Tunnel Established! Internet traffic is now routed through VPN Gate.");
                Task.Run(() => FlushDns());
            }
            else if (line.Contains("AUTH_FAILED", StringComparison.OrdinalIgnoreCase))
            {
                SetState(VpnState.Error, "Authentication Failed.");
            }
            else if (line.Contains("Cannot resolve host", StringComparison.OrdinalIgnoreCase) ||
                     line.Contains("Connection refused", StringComparison.OrdinalIgnoreCase))
            {
                SetState(VpnState.Error, "Server unreachable. Try another relay.");
            }
            else if (line.Contains("route addition failed", StringComparison.OrdinalIgnoreCase) ||
                     line.Contains("requires elevation", StringComparison.OrdinalIgnoreCase) ||
                     line.Contains("ERROR: Windows route add", StringComparison.OrdinalIgnoreCase))
            {
                Log("⚠️ [Routing Warning] System route modification failed: Administrator rights required to redirect default gateway.");
            }
        }

        public void Disconnect()
        {
            if (State == VpnState.Disconnected) return;

            SetState(VpnState.Disconnecting);
            Log("Stopping VPN tunnel and restoring system routing tables...");

            try
            {
                if (_process != null && !_process.HasExited)
                {
                    // 1. Send graceful shutdown command via Management Interface to let OpenVPN remove its routes
                    if (_managementPort > 0)
                    {
                        try
                        {
                            using var client = new TcpClient();
                            var connectTask = client.ConnectAsync("127.0.0.1", _managementPort);
                            if (Task.WhenAny(connectTask, Task.Delay(1000)).Result == connectTask && client.Connected)
                            {
                                using var stream = client.GetStream();
                                using var writer = new StreamWriter(stream) { AutoFlush = true };
                                writer.WriteLine("signal SIGTERM");
                                Log("Sent graceful termination signal to OpenVPN engine.");
                            }
                        }
                        catch { }
                    }

                    // 2. Wait up to 3 seconds for OpenVPN to finish its cleanup and route removal
                    if (!_process.WaitForExit(3000))
                    {
                        Log("OpenVPN did not exit in 3s; terminating process tree...");
                        var killProc = Process.Start(new ProcessStartInfo
                        {
                            FileName = "taskkill",
                            Arguments = $"/F /PID {_process.Id} /T",
                            CreateNoWindow = true,
                            UseShellExecute = false
                        });
                        killProc?.WaitForExit(2000);
                    }
                }
            }
            catch (Exception ex)
            {
                Log($"Notice while stopping process: {ex.Message}");
            }

            // 3. Purge any stale /1 routes as a failsafe sweep
            PurgeStaleRoutes();

            Cleanup();
            SetState(VpnState.Disconnected);
            Log("VPN Disconnected. System default gateway restored.");
        }

        private static int GetAvailablePort()
        {
            try
            {
                using var listener = new TcpListener(IPAddress.Loopback, 0);
                listener.Start();
                int port = ((IPEndPoint)listener.LocalEndpoint).Port;
                listener.Stop();
                return port;
            }
            catch
            {
                return 25340;
            }
        }

        public static void PurgeStaleRoutes()
        {
            try
            {
                // Clear any stacked 0.0.0.0/1 and 128.0.0.0/1 routes from dead/killed sessions
                for (int i = 0; i < 8; i++)
                {
                    using var p1 = Process.Start(new ProcessStartInfo
                    {
                        FileName = "route.exe",
                        Arguments = "delete 0.0.0.0 mask 128.0.0.0",
                        CreateNoWindow = true,
                        UseShellExecute = false
                    });
                    p1?.WaitForExit(400);
                    if (p1?.ExitCode != 0) break;
                }

                for (int i = 0; i < 8; i++)
                {
                    using var p2 = Process.Start(new ProcessStartInfo
                    {
                        FileName = "route.exe",
                        Arguments = "delete 128.0.0.0 mask 128.0.0.0",
                        CreateNoWindow = true,
                        UseShellExecute = false
                    });
                    p2?.WaitForExit(400);
                    if (p2?.ExitCode != 0) break;
                }

                // PowerShell NetRoute cleanup as a comprehensive fallback
                using var psProc = Process.Start(new ProcessStartInfo
                {
                    FileName = "powershell.exe",
                    Arguments = "-NoProfile -NonInteractive -Command \"Remove-NetRoute -DestinationPrefix '0.0.0.0/1' -Confirm:$false -ErrorAction SilentlyContinue; Remove-NetRoute -DestinationPrefix '128.0.0.0/1' -Confirm:$false -ErrorAction SilentlyContinue\"",
                    CreateNoWindow = true,
                    UseShellExecute = false
                });
                psProc?.WaitForExit(1500);

                FlushDns();
            }
            catch { }
        }

        private static void FlushDns()
        {
            try
            {
                using var pDns = Process.Start(new ProcessStartInfo
                {
                    FileName = "ipconfig.exe",
                    Arguments = "/flushdns",
                    CreateNoWindow = true,
                    UseShellExecute = false
                });
                pDns?.WaitForExit(1000);
            }
            catch { }
        }

        public async Task<bool> InstallEngineAsync(Action<string> statusCallback)
        {
            try
            {
                var tempMsi = Path.Combine(Path.GetTempPath(), "OpenVPN-Setup.msi");
                statusCallback("Downloading official OpenVPN installer...");

                using (var http = new HttpClient { Timeout = TimeSpan.FromMinutes(2) })
                {
                    var bytes = await http.GetByteArrayAsync(MsiDownloadUrl);
                    await File.WriteAllBytesAsync(tempMsi, bytes);
                }

                statusCallback("Installing OpenVPN and Wintun driver (Accept UAC prompt)...");

                var psi = new ProcessStartInfo
                {
                    FileName = "msiexec.exe",
                    Arguments = $"/i \"{tempMsi}\" /quiet /norestart",
                    Verb = "runas",
                    UseShellExecute = true
                };

                var proc = Process.Start(psi);
                if (proc != null)
                {
                    await proc.WaitForExitAsync();
                    try { File.Delete(tempMsi); } catch { }
                    return proc.ExitCode == 0 || proc.ExitCode == 3010;
                }
                return false;
            }
            catch (Exception ex)
            {
                statusCallback($"Installation error: {ex.Message}");
                return false;
            }
        }

        public bool ExportConfig(VpnServer server, string destinationPath)
        {
            try
            {
                var cfg = server.GetDecodedConfig();
                if (string.IsNullOrWhiteSpace(cfg)) return false;

                var header = $"# ========================================================\n" +
                             $"# VPN Gate Server: {server.CountryLong} ({server.IP})\n" +
                             $"# Speed: {server.SpeedMbps:F1} Mbps | Ping: {server.Ping} ms\n" +
                             $"# Credentials: Username='vpn', Password='vpn'\n" +
                             $"# ========================================================\n\n";

                var sb = new StringBuilder();
                sb.Append(header);

                using (var reader = new StringReader(cfg))
                {
                    string? line;
                    while ((line = reader.ReadLine()) != null)
                    {
                        var trimmed = line.Trim();
                        if (trimmed.StartsWith("block-outside-dns", StringComparison.OrdinalIgnoreCase))
                        {
                            continue;
                        }
                        sb.AppendLine(line);
                    }
                }

                sb.AppendLine();
                sb.AppendLine("# === Leak Protection & Resolvers ===");
                sb.AppendLine("dhcp-option DNS 8.8.8.8");
                sb.AppendLine("dhcp-option DNS 1.1.1.1");
                sb.AppendLine("block-ipv6");

                File.WriteAllText(destinationPath, sb.ToString(), Encoding.UTF8);
                return true;
            }
            catch
            {
                return false;
            }
        }

        private void Cleanup()
        {
            try
            {
                if (!string.IsNullOrEmpty(_activeTempDir) && Directory.Exists(_activeTempDir))
                {
                    Directory.Delete(_activeTempDir, true);
                }
            }
            catch { }

            _activeTempDir = null;
            _process = null;
        }

        private void SetState(VpnState newState, string? message = null)
        {
            State = newState;
            if (message != null) Log($"[{newState}] {message}");
            StateChanged?.Invoke(newState, message);
        }

        private void Log(string message) => LogReceived?.Invoke(message);
    }
}
