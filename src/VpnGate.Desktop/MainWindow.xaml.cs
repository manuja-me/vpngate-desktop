using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Security.Principal;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;
using Microsoft.Win32;
using VpnGate.Desktop.Models;
using VpnGate.Desktop.Services;

namespace VpnGate.Desktop
{
    public partial class MainWindow : Window
    {
        private readonly VpnGateService _vpnService = new();
        private readonly OpenVpnService _openVpnService = new();
        private readonly DispatcherTimer _durationTimer = new();
        private readonly DispatcherTimer _searchDebounceTimer = new();

        private List<VpnServer> _allServers = new();
        private VpnServer? _selectedServer;
        private string _selectedCountry = "All";
        private string _currentSort = "speed";
        private DateTime? _connectionStartTime;

        public MainWindow()
        {
            InitializeComponent();

            _openVpnService.StateChanged += OnVpnStateChanged;
            _openVpnService.LogReceived += OnVpnLogReceived;

            _durationTimer.Interval = TimeSpan.FromSeconds(1);
            _durationTimer.Tick += (_, _) => UpdateDurationTimer();

            _searchDebounceTimer.Interval = TimeSpan.FromMilliseconds(120);
            _searchDebounceTimer.Tick += (_, _) =>
            {
                _searchDebounceTimer.Stop();
                ApplyFilters();
            };

            Loaded += async (_, _) =>
            {
                UpdateEngineStatus();

                // Clean up any lingering zombie routes from prior ungraceful terminations
                _ = Task.Run(() => OpenVpnService.PurgeStaleRoutes());

                // Instantly load cached servers in <20ms for instant UI rendering
                _allServers = _vpnService.LoadCachedServers();
                if (_allServers.Count > 0)
                {
                    UpdateStatsAndFilters();
                }

                await RefreshServersAsync();
            };

            Closing += (_, _) =>
            {
                if (_openVpnService.State is VpnState.Connected or VpnState.Connecting)
                {
                    _openVpnService.Disconnect();
                }
            };
        }

        private void UpdateEngineStatus()
        {
            var isAdmin = IsRunningAsAdmin();
            if (_openVpnService.IsEngineInstalled)
            {
                TxtEngineStatus.Text = isAdmin ? "READY • ELEVATED" : "READY (NON-ADMIN)";
                TxtEngineIcon.Text = "●";
                TxtEngineIcon.Foreground = Brushes.White;
                BtnInstallEngine.Visibility = Visibility.Collapsed;
            }
            else
            {
                TxtEngineStatus.Text = "MISSING";
                TxtEngineIcon.Text = "○";
                TxtEngineIcon.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#737373"));
                BtnInstallEngine.Visibility = Visibility.Visible;
            }
        }

        private async Task RefreshServersAsync()
        {
            BtnRefresh.IsEnabled = false;
            BtnRefresh.Content = "⏳ Loading...";

            try
            {
                _allServers = await _vpnService.FetchServersAsync(forceRefresh: true);
                UpdateStatsAndFilters();
                OnVpnLogReceived($"Loaded {_allServers.Count} live servers across {GetUniqueCountryCount()} countries.");
            }
            catch (Exception ex)
            {
                if (_allServers.Count == 0)
                {
                    MessageBox.Show($"Failed to retrieve VPN Gate servers: {ex.Message}", "Network Error", MessageBoxButton.OK, MessageBoxImage.Warning);
                }
                else
                {
                    OnVpnLogReceived($"Warning: Network refresh failed ({ex.Message}), showing cached relay servers.");
                }
            }
            finally
            {
                BtnRefresh.IsEnabled = true;
                BtnRefresh.Content = "🔄 Refresh";
            }
        }

        private void UpdateStatsAndFilters()
        {
            TxtStatServers.Text = $"{_allServers.Count} Online";
            TxtStatCountries.Text = $"{GetUniqueCountryCount()} Regions";
            var maxSpeed = _allServers.Count > 0 ? _allServers.Max(s => s.SpeedMbps) : 0;
            TxtStatTopSpeed.Text = $"{maxSpeed:F1} Mbps";
            TxtCountryCount.Text = $"{GetUniqueCountryCount()} countries";

            PopulateCountries();
            ApplyFilters();
        }

        private int GetUniqueCountryCount() => _allServers.Select(s => s.CountryLong).Distinct().Count();

        private void PopulateCountries()
        {
            var previousSelection = _selectedCountry;

            var countries = _allServers
                .GroupBy(s => s.CountryLong)
                .OrderByDescending(g => g.Count())
                .Select(g => $"{g.First().Flag}  {g.Key} ({g.Count()})")
                .ToList();

            countries.Insert(0, $"ALL ({_allServers.Count})");

            LstCountries.ItemsSource = countries;

            if (previousSelection != "All")
            {
                var matchIndex = countries.FindIndex(c => c.Contains(previousSelection, StringComparison.OrdinalIgnoreCase));
                LstCountries.SelectedIndex = matchIndex >= 0 ? matchIndex : 0;
            }
            else
            {
                LstCountries.SelectedIndex = 0;
            }
        }

        private void ApplyFilters()
        {
            if (TxtSearch == null || LstServers == null) return;

            var search = TxtSearch.Text ?? string.Empty;
            var filtered = VpnGateService.FilterAndSort(_allServers, search, _selectedCountry, _currentSort);
            LstServers.ItemsSource = filtered;

            if (_selectedServer != null)
            {
                var match = filtered.FirstOrDefault(s => s.IP == _selectedServer.IP);
                if (match != null) LstServers.SelectedItem = match;
            }
        }

        private void LstServers_SelectionChanged(object sender, SelectionChangedEventArgs e)
        {
            if (LstServers.SelectedItem is VpnServer server)
            {
                _selectedServer = server;
                TxtSelectedTitle.Text = $"{server.Flag} {server.CountryLong}";
                TxtSelectedDetails.Text = $"{server.IP}:{server.Port} • {server.SpeedMbps:F1} Mbps • {server.Ping} ms • {server.Proto}";

                BtnExport.IsEnabled = true;
                if (_openVpnService.State == VpnState.Disconnected)
                {
                    BtnConnect.IsEnabled = true;
                }
            }
        }

        private void LstCountries_SelectionChanged(object sender, SelectionChangedEventArgs e)
        {
            if (LstCountries?.SelectedItem is string item)
            {
                if (item.StartsWith("ALL", StringComparison.OrdinalIgnoreCase) || item.StartsWith("🌍"))
                {
                    _selectedCountry = "All";
                }
                else
                {
                    var parts = item.Split("  ");
                    var name = parts.Length > 1 ? parts[1] : parts[0];
                    var idx = name.LastIndexOf('(');
                    _selectedCountry = (idx > 0 ? name[..idx] : name).Trim();
                }
                ApplyFilters();
            }
        }

        private void TxtSearch_TextChanged(object sender, TextChangedEventArgs e)
        {
            _searchDebounceTimer.Stop();
            _searchDebounceTimer.Start();
        }

        private void CmbSort_SelectionChanged(object sender, SelectionChangedEventArgs e)
        {
            if (CmbSort?.SelectedItem is ComboBoxItem item)
            {
                _currentSort = item.Content?.ToString() switch
                {
                    "Lowest Ping" => "ping",
                    "Most Sessions" => "sessions",
                    "Score" => "score",
                    _ => "speed"
                };
                ApplyFilters();
            }
        }

        private async void BtnConnect_Click(object sender, RoutedEventArgs e)
        {
            if (_openVpnService.State == VpnState.Connected)
            {
                _openVpnService.Disconnect();
            }
            else if (_openVpnService.State == VpnState.Disconnected)
            {
                if (_selectedServer == null) return;

                if (!IsRunningAsAdmin())
                {
                    var res = MessageBox.Show(
                        "Windows requires Administrator permissions to update routing tables and redirect your internet gateway to the VPN relay.\n\nWithout elevation, OpenVPN cannot modify the default route and your real IP will remain visible.\n\nWould you like to restart the application as Administrator now?",
                        "Administrator Rights Required",
                        MessageBoxButton.YesNo,
                        MessageBoxImage.Warning);

                    if (res == MessageBoxResult.Yes)
                    {
                        RestartAsAdmin();
                    }
                    return;
                }

                if (!_openVpnService.IsEngineInstalled)
                {
                    var res = MessageBox.Show(
                        "OpenVPN is required to route traffic. Would you like to install it automatically now?",
                        "OpenVPN Required",
                        MessageBoxButton.YesNo,
                        MessageBoxImage.Question);

                    if (res == MessageBoxResult.Yes)
                    {
                        await RunAutoInstallAsync();
                    }
                    return;
                }

                await _openVpnService.ConnectAsync(_selectedServer);
            }
        }

        private static bool IsRunningAsAdmin()
        {
            try
            {
                using var identity = WindowsIdentity.GetCurrent();
                var principal = new WindowsPrincipal(identity);
                return principal.IsInRole(WindowsBuiltInRole.Administrator);
            }
            catch
            {
                return false;
            }
        }

        private static void RestartAsAdmin()
        {
            try
            {
                var exePath = Environment.ProcessPath ?? Process.GetCurrentProcess().MainModule?.FileName;
                if (!string.IsNullOrEmpty(exePath))
                {
                    var psi = new ProcessStartInfo
                    {
                        FileName = exePath,
                        UseShellExecute = true,
                        Verb = "runas"
                    };
                    Process.Start(psi);
                    Application.Current.Shutdown();
                }
            }
            catch (Exception ex)
            {
                MessageBox.Show($"Could not elevate application: {ex.Message}", "Elevation Error", MessageBoxButton.OK, MessageBoxImage.Error);
            }
        }

        private async void BtnInstallEngine_Click(object sender, RoutedEventArgs e) => await RunAutoInstallAsync();

        private async Task RunAutoInstallAsync()
        {
            BtnInstallEngine.IsEnabled = false;
            TxtEngineStatus.Text = "⏳ Installing OpenVPN...";

            var ok = await Task.Run(() => _openVpnService.InstallEngineAsync(msg =>
            {
                Dispatcher.Invoke(() => OnVpnLogReceived(msg));
            }));

            UpdateEngineStatus();
            BtnInstallEngine.IsEnabled = true;

            if (ok)
            {
                MessageBox.Show("OpenVPN engine was installed successfully!", "Installation Success", MessageBoxButton.OK, MessageBoxImage.Information);
            }
        }

        private void OnVpnStateChanged(VpnState state, string? message)
        {
            Dispatcher.Invoke(() =>
            {
                switch (state)
                {
                    case VpnState.Connected:
                        _connectionStartTime = DateTime.UtcNow;
                        _durationTimer.Start();
                        TxtStatusDot.Text = "●";
                        TxtStatusDot.Foreground = Brushes.White;
                        TxtStatusBadge.Text = "CONNECTED";
                        TxtStatusBadge.Foreground = Brushes.White;
                        BtnConnect.Content = "■ DISCONNECT FROM RELAY";
                        BtnConnect.ClearValue(BackgroundProperty);
                        BtnConnect.ClearValue(ForegroundProperty);
                        BtnConnect.ClearValue(BorderBrushProperty);
                        try { BtnConnect.Style = (Style)FindResource("DisconnectBtn"); } catch { }
                        BtnConnect.IsEnabled = true;
                        break;

                    case VpnState.Connecting:
                        TxtStatusDot.Text = "◌";
                        TxtStatusDot.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#A3A3A3"));
                        TxtStatusBadge.Text = "CONNECTING...";
                        TxtStatusBadge.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#A3A3A3"));
                        BtnConnect.Content = "CONNECTING...";
                        BtnConnect.IsEnabled = false;
                        break;

                    case VpnState.Disconnecting:
                        TxtStatusDot.Text = "◌";
                        TxtStatusDot.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#737373"));
                        TxtStatusBadge.Text = "DISCONNECTING...";
                        TxtStatusBadge.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#737373"));
                        BtnConnect.IsEnabled = false;
                        break;

                    case VpnState.Error:
                        _durationTimer.Stop();
                        TxtDuration.Text = "00:00:00";
                        TxtStatusDot.Text = "✕";
                        TxtStatusDot.Foreground = Brushes.White;
                        TxtStatusBadge.Text = "ERROR";
                        TxtStatusBadge.Foreground = Brushes.White;
                        BtnConnect.Content = "⚡ RETRY CONNECT";
                        BtnConnect.ClearValue(BackgroundProperty);
                        BtnConnect.ClearValue(ForegroundProperty);
                        BtnConnect.ClearValue(BorderBrushProperty);
                        try { BtnConnect.Style = (Style)FindResource("PrimaryBtn"); } catch { }
                        BtnConnect.IsEnabled = _selectedServer != null;
                        break;

                    case VpnState.Disconnected:
                        _durationTimer.Stop();
                        TxtDuration.Text = "00:00:00";
                        TxtStatusDot.Text = "○";
                        TxtStatusDot.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#737373"));
                        TxtStatusBadge.Text = "DISCONNECTED";
                        TxtStatusBadge.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#A3A3A3"));
                        BtnConnect.Content = "⚡ CONNECT TO RELAY";
                        BtnConnect.ClearValue(BackgroundProperty);
                        BtnConnect.ClearValue(ForegroundProperty);
                        BtnConnect.ClearValue(BorderBrushProperty);
                        try { BtnConnect.Style = (Style)FindResource("PrimaryBtn"); } catch { }
                        BtnConnect.IsEnabled = _selectedServer != null;
                        break;
                }
            });
        }

        private void OnVpnLogReceived(string log)
        {
            Dispatcher.BeginInvoke(DispatcherPriority.Background, () =>
            {
                if (TxtLogs.Text.Length > 80_000)
                {
                    TxtLogs.Text = TxtLogs.Text[^40_000..];
                }
                TxtLogs.AppendText(log + Environment.NewLine);
                TxtLogs.ScrollToEnd();
            });
        }

        private void UpdateDurationTimer()
        {
            if (_connectionStartTime.HasValue)
            {
                var elapsed = DateTime.UtcNow - _connectionStartTime.Value;
                TxtDuration.Text = elapsed.ToString(@"hh\:mm\:ss");
            }
        }

        private void BtnExport_Click(object sender, RoutedEventArgs e)
        {
            if (_selectedServer == null) return;

            var dialog = new SaveFileDialog
            {
                FileName = $"vpngate_{_selectedServer.CountryShort}_{_selectedServer.IP}.ovpn",
                Filter = "OpenVPN Config (*.ovpn)|*.ovpn|All Files (*.*)|*.*",
                Title = "Export OpenVPN Profile"
            };

            if (dialog.ShowDialog() == true)
            {
                var ok = _openVpnService.ExportConfig(_selectedServer, dialog.FileName);
                if (ok)
                {
                    MessageBox.Show($"Configuration exported successfully to:\n{dialog.FileName}", "Export Completed", MessageBoxButton.OK, MessageBoxImage.Information);
                }
            }
        }

        private async void BtnRefresh_Click(object sender, RoutedEventArgs e) => await RefreshServersAsync();

        private void BtnClearLogs_Click(object sender, RoutedEventArgs e) => TxtLogs.Clear();

        private void BtnExportLogs_Click(object sender, RoutedEventArgs e)
        {
            try
            {
                Clipboard.SetText(TxtLogs.Text);
                MessageBox.Show("OpenVPN tunnel output copied to clipboard!", "Diagnostics", MessageBoxButton.OK, MessageBoxImage.Information);
            }
            catch (Exception ex)
            {
                MessageBox.Show($"Could not copy logs: {ex.Message}", "Error", MessageBoxButton.OK, MessageBoxImage.Warning);
            }
        }

        // Window Caption Controls
        private void BtnMinimize_Click(object sender, RoutedEventArgs e) => WindowState = WindowState.Minimized;

        private void BtnMaximize_Click(object sender, RoutedEventArgs e)
        {
            WindowState = WindowState == WindowState.Maximized ? WindowState.Normal : WindowState.Maximized;
        }

        private void BtnClose_Click(object sender, RoutedEventArgs e) => Close();

        // View Tabs
        private void TabBtnServers_Click(object sender, RoutedEventArgs e)
        {
            TabBtnServers.IsChecked = true;
            TabBtnDiagnostics.IsChecked = false;
            TabBtnServers.Background = Brushes.White;
            TabBtnServers.Foreground = Brushes.Black;
            TabBtnDiagnostics.Background = Brushes.Transparent;
            TabBtnDiagnostics.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#737373"));
            ViewServers.Visibility = Visibility.Visible;
            ViewDiagnostics.Visibility = Visibility.Collapsed;
        }

        private void TabBtnDiagnostics_Click(object sender, RoutedEventArgs e)
        {
            TabBtnServers.IsChecked = false;
            TabBtnDiagnostics.IsChecked = true;
            TabBtnServers.Background = Brushes.Transparent;
            TabBtnServers.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#737373"));
            TabBtnDiagnostics.Background = Brushes.White;
            TabBtnDiagnostics.Foreground = Brushes.Black;
            ViewServers.Visibility = Visibility.Collapsed;
            ViewDiagnostics.Visibility = Visibility.Visible;
        }
    }
}