# 🛡️ VPN Gate Studio • Windows Edition

[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011-0078D6?logo=windows&logoColor=white)](https://github.com/manuja-me/vpngate-desktop)
[![Language](https://img.shields.io/badge/Language-Rust%202021-dea584?logo=rust&logoColor=white)](https://github.com/manuja-me/vpngate-desktop)
[![UI Engine](https://img.shields.io/badge/UI-WebView2%20%2B%20Swiss%20Monochrome-000000?logo=html5&logoColor=white)](https://github.com/manuja-me/vpngate-desktop)
[![Binary Size](https://img.shields.io/badge/Binary-2.6%20MB%20Standalone-success)](dist/VpnGate.exe)
[![Memory](https://img.shields.io/badge/RAM-~35--45%20MB-blue)](dist/VpnGate.exe)
[![GitHub Release](https://img.shields.io/github/v/release/manuja-me/vpngate-desktop?logo=github&color=success)](https://github.com/manuja-me/vpngate-desktop/releases/latest)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

**VPN Gate Studio** is an ultra-lightweight, high-performance Windows desktop client designed to browse, filter, benchmark, and connect to thousands of free, public relay servers worldwide provided by the **VPN Gate Academic Experiment Project (University of Tsukuba, Japan)**.

Engineered in **Rust** with an evergreen **WebView2** frontend, it pairs a pure **Swiss Minimalist Monochrome** aesthetic with robust OpenVPN route orchestration, 100% stable retained DOM layout (zero hover jitter), and an ultra-low working set (~35–45 MB RAM).

---

## ⚡ How This Differs From Others

Many VPN solutions and clients exist, but **VPN Gate Studio** occupies a distinct niche:

| Feature / Dimension | 🏢 Commercial VPNs (Nord, Express, Surfshark) | 📜 Official SoftEther VPN Gate Plugin | 📁 Manual `.ovpn` Files + OpenVPN GUI | 🛡️ **VPN Gate Studio (This App)** |
| :--- | :--- | :--- | :--- | :--- |
| **Cost & Accounts** | Paid subscription ($5–$13/mo), requires email & credit card | Free, no account | Free, no account | **100% Free & Open-Source (FOSS), Zero Signups** |
| **Server Infrastructure** | Centralized datacenter server farms | Decentralized volunteer residential/campus nodes | Decentralized volunteer residential/campus nodes | **Decentralized volunteer residential/campus nodes** |
| **Censorship Evasion** | Datacenter IP ranges are easily identified and blocked by national firewalls (GFW) | Highly resistant to blocking due to residential IP churn | Highly resistant, but manual file hunting is slow | **Maximum: Auto-updated residential pool with 1-click connection** |
| **Client Memory Footprint**| ~150–250 MB RAM + 24/7 background system services | ~40–60 MB RAM (outdated Win32 GUI) | ~15–25 MB (tray icon, no server browser) | **~35–45 MB RAM total, zero 24/7 background daemons** |
| **Executable Size** | 80–150 MB installers | ~55 MB installer bundle | ~25 MB OpenVPN installer | **2.62 MB standalone executable** |
| **User Interface** | Flashy marketing ads, upsells, complex account settings | 2000s-era Windows 98/XP dialogs, cluttered tables | None (right-click Windows system tray menu) | **Modern Swiss Minimalist Monochrome, live telemetry, zero jitter** |
| **Route / Leak Protection**| Proprietary TAP drivers, often leaves DNS lingering | Basic SoftEther virtual adapter | Manual script hooks required | **Automated zombie route purge (`0.0.0.0/1`), DNS cache flush (`1.1.1.1`)** |

---

## 🎯 How This App Is Intended to Be Used

VPN Gate Studio is purposefully designed for **practical, friction-free utility**:

```
[ Launch VpnGate.exe ] ──► [ Filter by Country or Speed ] ──► [ Click "CONNECT TO RELAY" ] ──► [ Traffic Secured ]
```

### ✅ Ideal Use Cases
1. **Bypassing Government or Institutional Firewalls (Anti-Censorship):**
   Commercial VPNs with static IP blocks are rapidly blacklisted by state-level Deep Packet Inspection (DPI). Because VPN Gate nodes are operated by volunteers on residential broadband and university networks, their IP addresses rotate constantly, making them exceptionally difficult to censor.
2. **Instant Privacy on Public Wi-Fi:**
   Connect to coffee shop, airport, or hotel Wi-Fi networks and secure all system traffic with a single click—without creating an account, handing over an email address, or logging in.
3. **Geo-Unblocking for Research & Browsing:**
   Access academic databases, local search indexes, or media restricted to regions like Japan, South Korea, the United States, or Europe.
4. **Quick `.ovpn` Profile Exporting:**
   Need an OpenVPN profile for your iPhone, Android, or OpenWRT router? Click **"EXPORT .OVPN PROFILE"** on any server row to save the raw decrypted config to disk.

### ❌ What This App Is NOT Intended For
* **High-Bandwidth Torrenting or P2P Sharing:** Servers are donated by volunteers and academic institutions. Consuming gigabytes of peer-to-peer torrent traffic exhausts volunteer bandwidth and violates University of Tsukuba fair-use guidelines.
* **Malicious Activities or Attacks:** The University of Tsukuba maintains connection logs on relay servers to comply with Japanese academic network laws and prevent illegal activity.

---

## ⚖️ Pros & Cons

### 🟢 Pros
* **100% Free Forever:** No paywalls, trial periods, subscription prompts, or account registration.
* **Ultra-Lightweight Footprint:** Single **2.62 MB** executable. Runs smoothly even on low-spec hardware and older laptops.
* **No Background Clutter:** Leaves no persistent 24/7 services, telemetry agents, or startup daemons on your Windows machine when closed.
* **Zero Hover Jitter / Retained DOM:** Powered by standard CSS flexbox/grid in WebView2—eliminates the annoying scrollbar oscillation and coordinate vibration common in immediate-mode frameworks (`egui`).
* **Active Leak Protection:** Automatically cleans up zombie `/1` routing table entries upon exit or disconnect, and flushes the Windows DNS cache to prevent real IP leaks.
* **Built-in Diagnostics Console:** Live streaming terminal with real-time OpenVPN handshake progress, routing updates, and prominent `[CONNECTED]` confirmation.

### 🔴 Cons
* **Variable Bandwidth & Latency:** Relay nodes are hosted by volunteers worldwide on home or campus connections. Speeds range from 5 Mbps to 150+ Mbps depending on the host's ISP and geographical distance.
* **Server Churn:** Volunteer servers come online and go offline unpredictably. If a server disconnects, simply pick another from the refreshed matrix.
* **Academic Logging Policy:** Relay nodes record connection timestamps and source IPs to mitigate network abuse per University of Tsukuba policy. While your traffic is encrypted over the wire, exit nodes are public.
* **Requires OpenVPN & Administrator Privileges:** Windows network routing changes require UAC elevation (see explanation below).

---

## 🔐 Why Are Administrator Privileges Needed?

When you launch the app, you may notice the **`USER MODE • CLICK TO ELEVATE`** badge or a Windows UAC prompt.

### What Admin Privileges Provide:
1. **System-Wide Traffic Redirection:** Windows strictly prevents unprivileged programs from altering the system routing table. Admin elevation allows OpenVPN to inject `0.0.0.0/1` and `128.0.0.0/1` routes so all PC internet traffic routes through the encrypted tunnel. Without elevation, your traffic bypasses the VPN (real IP leak).
2. **TAP/Wintun Driver Acquisition:** Accessing the virtual network adapter driver (`\\.\Global\{GUID}.tap`) requires administrative device permissions.
3. **DNS Leak Prevention:** Allows the client to flush the Windows DNS cache (`ipconfig /flushdns`) and set secure fallback resolvers (`1.1.1.1` and `8.8.8.8`).
4. **Clean Disconnect & Route Recovery:** Ensures temporary routing entries are purged when you disconnect, preventing the common *"no internet after disconnecting"* bug.

> [!NOTE]
> Unlike commercial VPNs that install a permanent background service running 24/7 as `NT AUTHORITY\SYSTEM`, VPN Gate Studio is completely portable. It only requests elevation when active and leaves zero lingering services when closed.

---

## ✨ Features

- **Swiss Minimalist Monochrome UI:** Typography-driven black-and-white theme featuring `Cascadia Code` and `Consolas` monospace telemetry.
- **Hero Connection Centerpiece:** Instant 1-click **Connect / Disconnect** button, dynamic state transitions, live duration stopwatch, and selected node summary.
- **Search & Multi-Criteria Sorting:**
  - Search across Country, IP, Hostname, or Operator notes.
  - Sort on demand by **Highest Speed (Mbps)**, **Lowest Latency (ms)**, **Most Active Sessions**, or **Score**.
  - Geographic region filter pills (`ALL`, `JAPAN`, `KOREA`, `USA`, etc.) with server counts.
- **Live Diagnostics Console:** Real-time terminal with syntax-highlighted logs, auto-scroll, and one-click clipboard copying.
- **Sub-50ms Startup:** Automatically caches the relay directory to `%LOCALAPPDATA%\VpnGateDesktop\cache.csv` for instantaneous offline launch.
- **100% Offline Capable:** HTML, CSS, and JS assets are embedded directly into the Rust binary with zero CDN dependencies.

---

## 🚀 Getting Started

### 📦 Download Standalone Executable (Fastest)

Download the standalone package directly from **[GitHub Releases](https://github.com/manuja-me/vpngate-desktop/releases/latest)**:
* **`VpnGate.exe`**: 100% standalone single executable (MSVC build with statically linked WebView2).
* **`VpnGate-v2.0.0-Windows-x64.zip`**: Complete portable bundle containing the executable, silent OpenVPN installer script, and documentation.

Or run the local binary in [`dist\VpnGate.exe`](dist/VpnGate.exe).

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

The build script [`build.rs`](build.rs) automatically deploys `WebView2Loader.dll` alongside the executable.

---

## ⚙️ OpenVPN Engine Setup

A virtual network adapter (TAP-Windows6 or Wintun) is required to tunnel Windows system traffic:

* **1-Click Automated Batch Script:** Double-click **`install_openvpn.bat`** in the repository root. It elevates with Windows UAC, downloads the official signed WiX MSI installer, and silently installs OpenVPN.
* **Windows Package Manager (Winget):**
  ```powershell
  winget install --id OpenVPNTechnologies.OpenVPN -e --accept-package-agreements --accept-source-agreements
  ```
* **Manual Community Installer:** Download directly from [openvpn.net/community-downloads](https://openvpn.net/community-downloads/).

---

## 📁 Repository Structure

```
vpngate-desktop/
├── .github/workflows/             # Automated CI/CD
│   └── release.yml                # Standalone Windows package release pipeline
├── dist/                          # Production distribution
│   ├── VpnGate.exe                # Standalone native executable (2.62 MB)
│   └── WebView2Loader.dll         # Official Microsoft WebView2 loader (165 KB)
├── src/                           # Native Rust backend core
│   ├── main.rs                    # WRY/Tao window, event loop & IPC bridge
│   ├── openvpn.rs                 # OpenVPN management socket, routes & DNS
│   ├── vpngate.rs                 # VPNGate API client, CSV cache & parser
│   └── models.rs                  # Server data structures & serialization
├── ui/                            # Swiss Monochrome frontend
│   ├── index.html                 # Semantic structure & layout
│   ├── style.css                  # Pure CSS stylesheet (zero external CDN)
│   └── app.js                     # IPC communication & state management
├── assets/                        # Static binary dependencies (WebView2Loader.dll)
├── build.rs                       # Automated DLL deployment build script
├── Cargo.toml                     # Rust package manifest & dependencies
├── Cargo.lock                     # Locked dependency tree
├── install_openvpn.bat            # 1-Click silent automated OpenVPN installer
├── LICENSE                        # MIT License
├── .gitignore                     # Git ignore rules
└── README.md                      # Project documentation
```

---

## 🔒 Security & Trust Model

### 1. Client-Side Security (Rust + WebView2)
* **Memory Safety:** Core logic is implemented in safe Rust, eliminating buffer overflows, dangling pointers, and memory corruption bugs.
* **Isolated Offline UI:** All frontend assets (HTML, CSS, JS) are embedded into the compiled binary. Zero remote CDNs or external web origins are loaded, eliminating XSS and supply-chain injection vectors.
* **Zero Telemetry:** No user analytics, no tracking SDKs, no ads, and no cloud accounts. The only external API call is fetching the public relay list directly from `vpngate.net`.
* **Leak Protection:** Enforces trusted DNS resolvers (`1.1.1.1`, `8.8.8.8`), blocks IPv6 traffic (`block-ipv6`), flushes the Windows DNS cache on connect, and purges `/1` routing entries on exit.

### 2. Network Trust Model (Volunteer Relays)
* **End-to-End Encryption:** Traffic between your PC and the relay server is encrypted via OpenVPN/TLS.
* **Volunteer Exit Nodes:** Relays are hosted by independent volunteers and universities worldwide:
  * **HTTPS Traffic:** Fully secure. Relay operators cannot decrypt end-to-end TLS traffic (passwords, bank data, or chats).
  * **Plain HTTP & Metadata:** The relay operator can see destination IPs, hostnames (SNI), and unencrypted HTTP traffic. Always ensure sensitive browsing uses HTTPS.
* **Academic Logging Policy:** VPN Gate is designed for **censorship circumvention**, not criminal anonymity. The University of Tsukuba mandates anti-abuse logging (timestamps, source IP, packet volume) on all nodes.
* **Self-Signed Certificates:** Relay nodes generate self-signed certificates via SoftEther. OpenVPN displays an expected certificate verification warning since there is no central commercial CA.

### 3. Local OS & Process Isolation
* **Administrator Elevation:** Required strictly for modifying Windows routing tables (`0.0.0.0/1`) and binding the virtual network adapter. The app leaves no persistent 24/7 background services.
* **Management Socket:** The OpenVPN management interface is bound strictly to `127.0.0.1` on a randomized ephemeral port, unreachable from external networks.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
VPN Gate is an academic research service provided by the [University of Tsukuba](https://www.vpngate.net/).
