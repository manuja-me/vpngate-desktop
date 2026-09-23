"""
Headless Windows Launcher for VPN Gate Desktop
Can be double-clicked to start without opening a command prompt window.
"""
import sys
import os

# Ensure current directory is on sys.path
app_dir = os.path.dirname(os.path.abspath(__file__))
if app_dir not in sys.path:
    sys.path.insert(0, app_dir)

from app import VpnGateApp

if __name__ == "__main__":
    app = VpnGateApp()
    app.mainloop()
