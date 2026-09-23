# 🛡️ VPN Gate Desktop for Windows

A modern, native Windows desktop application that lets you browse, filter, and connect to thousands of free, public relay servers worldwide provided by the **VPN Gate Academic Project (University of Tsukuba)**.

---

## ✨ Features

- **Global Relay Network:** Access 90+ live community-run exit nodes across Japan, the United States, South Korea, Germany, the UK, Thailand, and more.
- **Modern Windows 11 Fluent UI:** Dark-themed CustomTkinter interface with smooth search, responsive server cards, and live badges.
- **Smart Sorting & Filtering:**
  - Sort by **Highest Speed (Mbps)**, **Lowest Ping (ms)**, **Most Active Sessions**, or **Score**.
  - One-click filter by country (with server counts).
  - Real-time search across Country, IP, Hostname, and Operator notes.
- **OpenVPN Tunnel Management:**
  - One-click **Connect / Disconnect** with connection duration timer.
  - Automatic detection of `openvpn.exe` on your system.
  - Real-time terminal log drawer to inspect handshakes and routing changes.
- **Export `.ovpn` Profiles:** One-click export of any server's OpenVPN configuration file to your PC for use with third-party clients (OpenVPN Connect, SoftEther, etc.).
- **Automatic Fallback Caching:** Keeps your last fetched relay list cached locally in case the VPN Gate API is momentarily unreachable.

---

## 🚀 How to Run

### Method 1: Double-Click Launcher
Simply double-click **`run.bat`** (or **`run.pyw`** for a console-less experience).

### Method 2: Command Line
Open PowerShell or Command Prompt in this folder:
```powershell
python app.py
```

---

## ⚙️ OpenVPN Engine Requirement

To route full system network traffic through VPN Gate, Windows requires an OpenVPN engine with a virtual network adapter driver (TAP or Wintun).

### Option A: 1-Click Silent Automated Installer (Fastest & Zero Interaction)
Simply double-click **`install_openvpn.bat`** in the project folder. It will:
1. Elevate automatically with Windows UAC.
2. Download the official signed OpenVPN WiX MSI installer.
3. Install OpenVPN and the Wintun adapter driver quietly in the background without any wizard dialogs.

### Option B: Windows Package Manager (Winget)
The exact Winget package ID is `OpenVPNTechnologies.OpenVPN`. Run this command in an elevated PowerShell terminal:
```powershell
winget install --id OpenVPNTechnologies.OpenVPN -e --accept-package-agreements --accept-source-agreements
```

### Option C: Official Community Installer
Download the manual installer directly from [openvpn.net/community-downloads](https://openvpn.net/community-downloads/).

> [!IMPORTANT]
> **Administrator Privileges:** Modifying Windows routing tables to redirect internet traffic requires Administrator rights. If you receive an elevation prompt or error when connecting, simply right-click `run.bat` and select **"Run as administrator"**.

---

## 📁 Project Structure

```
vpngate-desktop/
├── app.py                     # Main application entry point & CustomTkinter GUI
├── vpngate_client.py          # API client for VPN Gate (fetching, caching, parsing CSV, sorting)
├── vpn_manager.py             # OpenVPN process controller, path detector, connection lifecycle
├── ui/
│   ├── __init__.py
│   ├── server_card.py         # CustomTkinter server card widget
│   ├── log_drawer.py          # Real-time OpenVPN terminal log output
│   └── download_dialog.py     # OpenVPN setup guide dialog
├── run.bat                    # Windows batch launcher
├── run.pyw                    # Headless Windows launcher
└── README.md                  # Documentation and usage guide
```

---

## 🔒 Security & Privacy Notice
* VPN Gate servers are operated by public academic volunteers.
* Default authentication credentials for all VPN Gate servers are `vpn` / `vpn`.
* While your connection to the server is encrypted, your unencrypted traffic exits from the volunteer's server. Always ensure your web traffic uses HTTPS.
