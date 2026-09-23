# 🛡️ VPN Gate Studio • Desktop Edition for Windows

[![Windows Native](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011-0078D6?logo=windows&logoColor=white)](https://github.com/manuja-me/vpngate-desktop)
[![Target Framework](https://img.shields.io/badge/Framework-.NET%208.0%20WPF-512BD4?logo=dotnet&logoColor=white)](https://github.com/manuja-me/vpngate-desktop)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Status](https://img.shields.io/badge/Status-Direct3D%20Hardware%20Accelerated-success)](https://github.com/manuja-me/vpngate-desktop)

A high-performance, modern Windows-native desktop application to browse, benchmark, filter, and connect to thousands of free, public relay servers worldwide provided by the **VPN Gate Academic Experiment Project (University of Tsukuba, Japan)**.

---

## 🎯 Purpose of This App

The primary goal of **VPN Gate Studio** is to bring the power of the worldwide academic VPN Gate network to everyday Windows users through a sleek, fast, one-click desktop client.

### 1. 🌐 Free, Account-Free Privacy & Anonymity
* **Zero Subscriptions or Logins:** Connect to thousands of public volunteer-run relay servers across Japan, South Korea, the United States, Europe, and more at zero cost.
* **Encrypted Tunneling:** Routes all system traffic through secure OpenVPN tunnels, concealing your true IP address and protecting your connection on unsecured public Wi-Fi networks.

### 2. 🧱 Censorship & Firewall Resistance
* Commercial VPN services rely on static data-center IP blocks that are easily targeted and banned by government firewalls (such as the Great Firewall) or campus/workplace filters.
* Because VPN Gate relays are hosted dynamically by volunteers on residential and university connections, **their IP addresses are decentralized and constantly shifting**, making them exceptionally resistant to centralized blocking.

### 3. ⚡ Eliminating the Friction of Manual Setup
Traditionally, using VPN Gate requires either:
* Running the outdated, legacy 2000s-era SoftEther client, or
* Manually hunting down, downloading, and importing individual `.ovpn` configuration files into OpenVPN.

**VPN Gate Studio solves this completely:** It continuously fetches online relays, runs latency and bandwidth metrics, automatically resolves configuration parameters, and connects with a single click.

---

## ✨ Key Features

- **High-Speed Direct3D GPU Composition:** Built on .NET 8 WPF with pure hardware-accelerated Direct3D rendering for butter-smooth 60–144 FPS UI performance.
- **Hero Connection Centerpiece:** Instant 1-click **Connect / Disconnect** button, dynamic state transitions, live duration clock, and target relay summaries.
- **Pixel-Based Virtualized Explorer:** Recycled visual element rendering effortlessly handles hundreds of live servers with zero scrolling lag or dropped frames.
- **120ms Debounced Search & Instant Filtering:**
  - Real-time search across Country, IP, Hostname, and Operator notes with zero memory allocation.
  - Sort on demand by **Highest Speed (Mbps)**, **Lowest Latency (ms)**, **Most Active Sessions**, or **Score**.
  - One-click geographic region filter showing server distribution by country.
- **Embedded Engine Installer:** Detects if OpenVPN is present on your system and provides a 1-click silent automated installer directly inside the app.
- **Live Diagnostics Terminal:** Built-in console tab to view real-time OpenVPN handshake progress, routing changes, and diagnostic logs with one-click clipboard copying.
- **One-Click `.ovpn` Exporter:** Export any node's raw OpenVPN profile to disk for use on routers, mobile phones, or other devices.
- **Sub-20ms Startup:** Automatically caches the relay directory to `%LOCALAPPDATA%\VpnGateDesktop\cache.csv` so the interface loads instantly upon opening, while silently refreshing the latest network state in the background.

---

## 🚀 Getting Started

### 🔷 Native Executable (.NET 8 WPF — Recommended)

The precompiled standalone Windows executable is included in the [`dist/`](dist/) folder:

1. **Launch Immediately:**
   Double-click **`Launch-VpnGate.bat`** (or run **`dist\VpnGate.Desktop.exe`** directly).
2. **Rebuild from Source:**
   Run **`build.bat`** or compile using the .NET 8 CLI:
   ```powershell
   dotnet publish src\VpnGate.Desktop\VpnGate.Desktop.csproj -c Release -o dist
   ```

---

### 🐍 Python / CustomTkinter Edition (Alternative)

If you prefer running the Python CustomTkinter implementation:
* Double-click **`run.bat`** (or **`run.pyw`** for console-free background execution).
* Or execute directly from your terminal:
  ```powershell
  python app.py
  ```

---

## ⚙️ OpenVPN Engine Setup

To tunnel Windows system traffic, an OpenVPN executable and virtual network adapter (TAP or Wintun) are required.

* **In-App Installation (Easiest):** If OpenVPN is missing, the app will display an **Install** button on the title bar and prompt you to install it automatically with one click.
* **1-Click Batch Script:** Double-click **`install_openvpn.bat`** in the repository root. It elevates with Windows UAC, downloads the official signed WiX MSI installer, and silently installs OpenVPN without prompts.
* **Windows Package Manager (Winget):**
  ```powershell
  winget install --id OpenVPNTechnologies.OpenVPN -e --accept-package-agreements --accept-source-agreements
  ```
* **Manual Community Installer:** Download directly from [openvpn.net/community-downloads](https://openvpn.net/community-downloads/).

> [!IMPORTANT]
> **Administrator Privileges:** Redirecting Windows system network routing tables requires Administrator permissions. If prompted by Windows UAC, approve elevation to allow the tunnel adapter to route traffic.

---

## 📁 Repository Structure

```
vpngate-desktop/
├── dist/                          # Compiled, production-ready Windows binaries
│   └── VpnGate.Desktop.exe        # Standalone native executable
├── src/VpnGate.Desktop/           # C# .NET 8 WPF Studio Edition
│   ├── Models/
│   │   └── VpnServer.cs           # Server data model & formatting
│   ├── Services/
│   │   ├── VpnGateService.cs      # API fetcher, CSV parser, cache, sorting
│   │   └── OpenVpnService.cs      # Tunnel lifecycle, route restorer, installer
│   ├── MainWindow.xaml            # Hardware-accelerated Direct3D Dark Theme UI
│   ├── MainWindow.xaml.cs         # Event handlers, search debounce, state machine
│   └── VpnGate.Desktop.csproj     # Project manifest (net8.0-windows)
├── ui/                            # Python CustomTkinter UI components
├── app.py                         # Python CustomTkinter entry point
├── vpngate_client.py              # Python API client & CSV parser
├── vpn_manager.py                 # Python OpenVPN process controller
├── Launch-VpnGate.bat             # 1-Click launcher for .NET 8 Native app
├── build.bat                      # Build script for .NET 8 Desktop app
├── install_openvpn.bat            # 1-Click silent automated OpenVPN installer
├── run.bat                        # Launcher for Python edition
└── README.md                      # Project documentation
```

---

## 🔒 Security & Privacy Notice

* **Academic Volunteer Network:** VPN Gate servers are operated by volunteers as part of an academic experiment by the University of Tsukuba.
* **Authentication:** Default credentials for all VPN Gate servers are preconfigured as username `vpn` and password `vpn`.
* **HTTPS Recommended:** While the tunnel between your PC and the relay server is strongly encrypted, exit traffic enters the public internet from the volunteer's host. Always verify that sensitive web connections use HTTPS/TLS.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
VPN Gate is an academic research service provided by the [University of Tsukuba](https://www.vpngate.net/).
