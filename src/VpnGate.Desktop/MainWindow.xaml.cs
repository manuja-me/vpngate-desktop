using System;
using System.Collections.Generic;
using System.Linq;
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

        private List<VpnServer> _allServers = new();
        private VpnServer? _selectedServer;
        private string _selectedCountry = "All";
        private string _currentSort = "speed";
        private DateTime? _connectionStartTime;
        private bool _isLogsExpanded = true;

        public MainWindow()
        {
            InitializeComponent();

            _openVpnService.StateChanged += OnVpnStateChanged;
            _openVpnService.LogReceived += OnVpnLogReceived;

            _durationTimer.Interval = TimeSpan.FromSeconds(1);
            _durationTimer.Tick += (_, _) => UpdateDurationTimer();

            Loaded += async (_, _) =>
            {
                UpdateEngineStatus();
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
            if (_openVpnService.IsEngineInstalled)
            {
                TxtEngineStatus.Text = "OpenVPN Ready";
                TxtEngineIcon.Text = "●";
                TxtEngineIcon.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#10B981"));
                BtnInstallEngine.Visibility = Visibility.Collapsed;
            }
            else
            {
                TxtEngineStatus.Text = "OpenVPN Missing";
                TxtEngineIcon.Text = "●";
                TxtEngineIcon.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#F59E0B"));
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

                // Update KPI Dashboard Stats
                TxtStatServers.Text = $"{_allServers.Count} Online";
                TxtStatCountries.Text = $"{GetUniqueCountryCount()} Regions";
                var maxSpeed = _allServers.Count > 0 ? _allServers.Max(s => s.SpeedMbps) : 0;
                TxtStatTopSpeed.Text = $"{maxSpeed:F1} Mbps";
                TxtCountryCount.Text = $"{GetUniqueCountryCount()} countries";

                PopulateCountries();
                ApplyFilters();
                OnVpnLogReceived($"Loaded {_allServers.Count} live servers across {GetUniqueCountryCount()} countries.");
            }
            catch (Exception ex)
            {
                MessageBox.Show($"Failed to retrieve VPN Gate servers: {ex.Message}", "Network Error", MessageBoxButton.OK, MessageBoxImage.Warning);
            }
            finally
            {
                BtnRefresh.IsEnabled = true;
                BtnRefresh.Content = "🔄 Refresh";
            }
        }

        private int GetUniqueCountryCount() => _allServers.Select(s => s.CountryLong).Distinct().Count();

        private void PopulateCountries()
        {
            var countries = _allServers
                .GroupBy(s => s.CountryLong)
                .OrderByDescending(g => g.Count())
                .Select(g => $"{g.First().Flag}  {g.Key} ({g.Count()})")
                .ToList();

            countries.Insert(0, $"🌍  All ({_allServers.Count})");

            LstCountries.ItemsSource = countries;
            LstCountries.SelectedIndex = 0;
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
                TxtSelectedDetails.Text = $"IP: {server.IP}:{server.Port} • Speed: {server.SpeedMbps:F1} Mbps • Ping: {server.Ping} ms";

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
                if (item.StartsWith("🌍"))
                {
                    _selectedCountry = "All";
                }
                else
                {
                    var parts = item.Split("  ");
                    if (parts.Length > 1)
                    {
                        var name = parts[1];
                        var idx = name.LastIndexOf('(');
                        _selectedCountry = (idx > 0 ? name[..idx] : name).Trim();
                    }
                }
                ApplyFilters();
            }
        }

        private void TxtSearch_TextChanged(object sender, TextChangedEventArgs e) => ApplyFilters();

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
                        TxtStatusDot.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#10B981"));
                        TxtStatusBadge.Text = "Connected";
                        TxtStatusBadge.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#10B981"));
                        BtnConnect.Content = "🛑 Disconnect";
                        BtnConnect.Background = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#EF4444"));
                        BtnConnect.IsEnabled = true;
                        break;

                    case VpnState.Connecting:
                        TxtStatusDot.Text = "●";
                        TxtStatusDot.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#F59E0B"));
                        TxtStatusBadge.Text = "Connecting...";
                        TxtStatusBadge.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#F59E0B"));
                        BtnConnect.Content = "Connecting...";
                        BtnConnect.IsEnabled = false;
                        break;

                    case VpnState.Disconnecting:
                        TxtStatusDot.Text = "●";
                        TxtStatusDot.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#94A3B8"));
                        TxtStatusBadge.Text = "Disconnecting...";
                        TxtStatusBadge.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#94A3B8"));
                        BtnConnect.IsEnabled = false;
                        break;

                    case VpnState.Error:
                        _durationTimer.Stop();
                        TxtDuration.Text = "00:00:00";
                        TxtStatusDot.Text = "●";
                        TxtStatusDot.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#EF4444"));
                        TxtStatusBadge.Text = "Error";
                        TxtStatusBadge.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#EF4444"));
                        BtnConnect.Content = "⚡ Retry Connect";
                        BtnConnect.Background = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#2563EB"));
                        BtnConnect.IsEnabled = _selectedServer != null;
                        break;

                    case VpnState.Disconnected:
                        _durationTimer.Stop();
                        TxtDuration.Text = "00:00:00";
                        TxtStatusDot.Text = "●";
                        TxtStatusDot.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#64748B"));
                        TxtStatusBadge.Text = "Disconnected";
                        TxtStatusBadge.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#94A3B8"));
                        BtnConnect.Content = "⚡ Connect";
                        BtnConnect.Background = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#2563EB"));
                        BtnConnect.IsEnabled = _selectedServer != null;
                        break;
                }
            });
        }

        private void OnVpnLogReceived(string log)
        {
            Dispatcher.Invoke(() =>
            {
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

        private void BtnToggleLogs_Click(object sender, RoutedEventArgs e)
        {
            if (_isLogsExpanded)
            {
                TxtLogs.Visibility = Visibility.Collapsed;
                BtnToggleLogs.Content = "▼ Expand";
                _isLogsExpanded = false;
            }
            else
            {
                TxtLogs.Visibility = Visibility.Visible;
                BtnToggleLogs.Content = "▲ Collapse";
                _isLogsExpanded = true;
            }
        }
    }
}