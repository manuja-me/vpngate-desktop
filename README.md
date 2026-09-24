# 🛡️ VPN Gate Studio • Windows Edition

[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011-0078D6?logo=windows&logoColor=white)](https://github.com/manuja-me/vpngate-desktop)
[![Language](https://img.shields.io/badge/Language-Rust%202021-dea584?logo=rust&logoColor=white)](https://github.com/manuja-me/vpngate-desktop)
[![UI Engine](https://img.shields.io/badge/UI-WebView2%20%2B%20Swiss%20Monochrome-000000?logo=html5&logoColor=white)](https://github.com/manuja-me/vpngate-desktop)
[![Binary Size](https://img.shields.io/badge/Binary-2.6%20MB%20Standalone-success)](dist/VpnGate.exe)
[![Memory](https://img.shields.io/badge/RAM-~35--45%20MB-blue)](dist/VpnGate.exe)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

An ultra-lightweight, high-performance Windows desktop application to browse, filter, benchmark, and connect to thousands of free, public relay servers worldwide provided by the **VPN Gate Academic Experiment Project (University of Tsukuba, Japan)**.

Built in **Rust** with an evergreen **WebView2** frontend, featuring a pure **Swiss Minimalist Monochrome** aesthetic, 100% stable retained DOM layout (zero hover jitter), and robust native OpenVPN route orchestration.

---

## 🎯 Purpose of This App

The primary goal of **VPN Gate Studio** is to bring the power of the worldwide academic VPN Gate network to Windows users through a sleek, fast, one-click desktop client.

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

- **Ultra-Lightweight & Fast:** 
  - **2.62 MB standalone `.exe`** with zero external DLLs, Node.js, or .NET runtimes.
  - **~35–45 MB total RAM footprint** (75% lighter than WPF, 80% lighter than Electron).
  - Sub-50ms instant startup from local cache (`%LOCALAPPDATA%\VpnGateDesktop\cache.csv`).
- **Swiss Minimalist Monochrome UI:**
  - High-contrast, typography-driven black & white design inspired by Swiss international style.
  - Crisp monospace telemetry (`Cascadia Code`, `Consolas`).
  - **100% Layout Stability:** Retained DOM flexbox/grid layout eliminates coordinate oscillation and hover vibration.
- **Hero Connection Centerpiece:** Instant 1-click **Connect / Disconnect** button, dynamic state transitions, live duration stopwatch, and target relay summaries.
- **Instant Search & Multi-Criteria Sorting:**
  - Filter by Country, IP, Hostname, or Operator notes.
  - Sort on demand by **Highest Speed (Mbps)**, **Lowest Latency (ms)**, **Most Active Sessions**, or **Score**.
  - One-click geographic region filter pills showing live server distribution by country.
- **Native OpenVPN Route & DNS Leak Protection:**
  - Dynamic local TCP management socket (`--management 127.0.0.1 <port>`) with graceful SIGTERM teardown.
  - Automated zombie route cleanup (`route.exe delete 0.0.0.0 mask 128.0.0.0` and PowerShell route table reset).
  - Automated DNS cache flushing (`ipconfig /flushdns`) on connect and disconnect.
- **Live Diagnostics Console:** Real-time terminal tab showing OpenVPN handshake progress, routing changes, and diagnostic logs with one-click clipboard copying.
- **One-Click `.ovpn` Exporter:** Export any node's raw OpenVPN profile to disk for use on routers, mobile phones, or other devices.
- **100% Offline Capable:** All HTML, CSS, and JS assets are embedded directly into the binary with zero CDN dependencies.

---

## 🚀 Getting Started

### 📦 Run the Precompiled Binary

The standalone Windows executable is located in the [`dist/`](dist/) folder:

1. Download or clone this repository.
2. Run **`dist\VpnGate.exe`**.

### 🛠️ Build from Source

Requirements:
- **Rust 1.80+** (with Cargo)
- **Windows 10 / 11** (WebView2 Runtime is built into Windows 10/11)

```powershell
# Clone the repository
git clone https://github.com/manuja-me/vpngate-desktop.git
cd vpngate-desktop

# Run directly in development mode
cargo run

# Build optimized release binary
cargo build --release
# Output: target\release\vpngate.exe (2.6 MB)
```

---

## ⚙️ OpenVPN Engine Setup

To tunnel Windows system traffic, an OpenVPN executable and virtual network adapter (TAP or Wintun) are required.

* **1-Click Batch Script:** Double-click **`install_openvpn.bat`** in the repository root. It elevates with Windows UAC, downloads the official signed WiX MSI installer, and silently installs OpenVPN without prompts.
* **Windows Package Manager (Winget):**
  ```powershell
  winget install --id OpenVPNTechnologies.OpenVPN -e --accept-package-agreements --accept-source-agreements
  ```
* **Manual Community Installer:** Download directly from [openvpn.net/community-downloads](https://openvpn.net/community-downloads/).

> [!IMPORTANT]
> **Administrator Privileges:** Redirecting Windows system network routing tables requires Administrator permissions. The app provides a one-click **USER MODE • CLICK TO ELEVATE** button on the header to restart with administrative privileges.

---

## 📁 Repository Structure

```
vpngate-desktop/
├── dist/                          # Production executable
│   └── VpnGate.exe                # Standalone native executable (2.6 MB)
├── src/                           # Rust backend core engine
│   ├── main.rs                    # WRY/Tao window, event loop & IPC bridge
│   ├── openvpn.rs                 # OpenVPN management socket, routes & DNS
│   ├── vpngate.rs                 # VPNGate API client, CSV cache & parser
│   └── models.rs                  # Server data structures & serialization
├── ui/                            # Swiss Monochrome frontend
│   ├── index.html                 # Semantic structure & layout
│   ├── style.css                  # Pure CSS stylesheet (zero external CDN)
│   └── app.js                     # IPC communication & state management
├── Cargo.toml                     # Rust package manifest & dependencies
├── Cargo.lock                     # Locked dependency tree
├── install_openvpn.bat            # 1-Click silent automated OpenVPN installer
├── .gitignore                     # Git ignore rules
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
