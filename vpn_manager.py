import os
import sys
import shutil
import tempfile
import threading
import subprocess
import logging
from enum import Enum
from typing import Optional, Callable
from vpngate_client import VpnServer

logger = logging.getLogger("vpngate.manager")

class ConnectionState(Enum):
    DISCONNECTED = "Disconnected"
    CONNECTING = "Connecting..."
    CONNECTED = "Connected"
    DISCONNECTING = "Disconnecting..."
    ERROR = "Error"

# Common installation paths for OpenVPN on Windows
COMMON_OPENVPN_PATHS = [
    r"C:\Program Files\OpenVPN\bin\openvpn.exe",
    r"C:\Program Files (x86)\OpenVPN\bin\openvpn.exe",
    r"C:\Program Files\OpenVPN Connect\OpenVPNConnect.exe",
    os.path.expandvars(r"%LOCALAPPDATA%\Programs\OpenVPN\bin\openvpn.exe"),
]

class VpnManager:
    """Manages the OpenVPN tunnel process lifecycle and configuration."""

    def __init__(self, custom_openvpn_path: Optional[str] = None):
        self.custom_openvpn_path = custom_openvpn_path
        self.state = ConnectionState.DISCONNECTED
        self.current_server: Optional[VpnServer] = None
        self.process: Optional[subprocess.Popen] = None
        self.temp_dir: Optional[str] = None

        # Callbacks
        self.on_state_change: Optional[Callable[[ConnectionState, Optional[str]], None]] = None
        self.on_log: Optional[Callable[[str], None]] = None

        self._monitor_thread: Optional[threading.Thread] = None
        self._stop_event = threading.Event()

    def find_openvpn_binary(self) -> Optional[str]:
        """Searches for openvpn.exe on the system."""
        # 1. Custom configured path
        if self.custom_openvpn_path and os.path.isfile(self.custom_openvpn_path):
            return self.custom_openvpn_path

        # 2. Check PATH environment variable
        which_path = shutil.which("openvpn")
        if which_path and os.path.isfile(which_path):
            return which_path

        # 3. Check common program files locations
        for candidate in COMMON_OPENVPN_PATHS:
            if os.path.isfile(candidate):
                return candidate

        return None

    def is_engine_installed(self) -> bool:
        return self.find_openvpn_binary() is not None

    def export_config(self, server: VpnServer, destination_path: str) -> bool:
        """Exports the decoded .ovpn profile to a user-specified path."""
        try:
            config_content = server.get_ovpn_config()
            if not config_content:
                return False

            header = (
                f"# ========================================================\n"
                f"# VPN Gate Server: {server.country_long} ({server.ip})\n"
                f"# Speed: {server.speed} Mbps | Ping: {server.ping} ms\n"
                f"# Credentials: Username='vpn', Password='vpn'\n"
                f"# ========================================================\n\n"
            )
            with open(destination_path, "w", encoding="utf-8") as f:
                f.write(header + config_content)
            return True
        except Exception as e:
            logger.error(f"Failed to export config: {e}")
            return False

    def connect(self, server: VpnServer):
        """Starts connecting to the specified VPN Gate server."""
        if self.state in (ConnectionState.CONNECTED, ConnectionState.CONNECTING):
            logger.warning("Already connected or connecting. Disconnect first.")
            return

        openvpn_bin = self.find_openvpn_binary()
        if not openvpn_bin:
            self._set_state(ConnectionState.ERROR, "OpenVPN executable not found on this system.")
            return

        self.current_server = server
        self._set_state(ConnectionState.CONNECTING)
        self._log(f"Preparing tunnel for {server.flag} {server.country_long} ({server.ip}:{server.port})...")

        # Create temporary working directory
        self.temp_dir = tempfile.mkdtemp(prefix="vpngate_")
        config_path = os.path.join(self.temp_dir, "profile.ovpn")
        auth_path = os.path.join(self.temp_dir, "auth.txt")

        # Write credentials (vpn / vpn)
        with open(auth_path, "w", encoding="utf-8") as f:
            f.write("vpn\nvpn\n")

        # Write config
        raw_config = server.get_ovpn_config()
        if not raw_config:
            self._set_state(ConnectionState.ERROR, "Failed to decode OpenVPN profile.")
            return

        # Strip problematic directives that cause escape errors or interactive prompts
        tweaked_lines = []
        for line in raw_config.splitlines():
            line_s = line.strip()
            if line_s == "auth-user-pass" or line_s.startswith("auth-user-pass "):
                continue
            if line_s == "persist-key":
                continue
            tweaked_lines.append(line)

        with open(config_path, "w", encoding="utf-8") as f:
            f.write("\n".join(tweaked_lines))

        # Launch OpenVPN process in background thread
        self._stop_event.clear()
        self._monitor_thread = threading.Thread(
            target=self._run_openvpn_process,
            args=(openvpn_bin, self.temp_dir),
            daemon=True
        )
        self._monitor_thread.start()

    def _run_openvpn_process(self, openvpn_bin: str, temp_dir: str):
        """Worker thread to run OpenVPN and monitor logs."""
        cmd = [
            openvpn_bin,
            "--config", "profile.ovpn",
            "--auth-user-pass", "auth.txt",
            "--verb", "3"
        ]

        creation_flags = 0
        if sys.platform == "win32":
            creation_flags = subprocess.CREATE_NO_WINDOW

        try:
            self._log(f"Spawning OpenVPN engine: {openvpn_bin} ...")
            self.process = subprocess.Popen(
                cmd,
                cwd=temp_dir,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                bufsize=1,
                creationflags=creation_flags
            )

            # Read stdout line by line
            for line in iter(self.process.stdout.readline, ""):
                if self._stop_event.is_set():
                    break
                line_clean = line.strip()
                if line_clean:
                    self._log(line_clean)

                    # State transitions
                    if "Initialization Sequence Completed" in line_clean:
                        self._set_state(ConnectionState.CONNECTED)
                        self._log(">>> Tunnel Established! Internet traffic is now routed through VPN Gate.")
                    elif "AUTH_FAILED" in line_clean:
                        self._set_state(ConnectionState.ERROR, "Authentication Failed.")
                    elif "Cannot resolve host address" in line_clean or "Connection refused" in line_clean:
                        self._set_state(ConnectionState.ERROR, "Server unreachable. Try another relay.")

            self.process.wait()
            ret = self.process.returncode
            if not self._stop_event.is_set():
                if ret != 0:
                    self._set_state(ConnectionState.ERROR, f"OpenVPN exited with error code {ret}.")
                else:
                    self._set_state(ConnectionState.DISCONNECTED)

        except PermissionError:
            self._set_state(ConnectionState.ERROR, "Administrator permissions required to configure network adapter.")
        except Exception as e:
            logger.error(f"Process error: {e}")
            self._set_state(ConnectionState.ERROR, str(e))
        finally:
            self._cleanup()

    def disconnect(self):
        """Disconnects the active VPN tunnel cleanly."""
        if self.state == ConnectionState.DISCONNECTED:
            return

        self._set_state(ConnectionState.DISCONNECTING)
        self._log("Stopping VPN tunnel and restoring system routing tables...")
        self._stop_event.set()

        if self.process:
            try:
                if sys.platform == "win32":
                    # Force kill child processes and openvpn tree
                    subprocess.run(
                        ["taskkill", "/F", "/PID", str(self.process.pid), "/T"],
                        stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL,
                        check=False
                    )
                else:
                    self.process.terminate()
            except Exception as e:
                logger.warning(f"Error terminating openvpn: {e}")

        self._cleanup()
        self._set_state(ConnectionState.DISCONNECTED)
        self._log("VPN Disconnected.")

    def _cleanup(self):
        """Removes temporary files."""
        if self.temp_dir and os.path.exists(self.temp_dir):
            try:
                shutil.rmtree(self.temp_dir, ignore_errors=True)
            except Exception:
                pass
            self.temp_dir = None
        self.process = None

    def _set_state(self, new_state: ConnectionState, message: Optional[str] = None):
        self.state = new_state
        if message:
            self._log(f"[{new_state.value}] {message}")
        if self.on_state_change:
            try:
                self.on_state_change(new_state, message)
            except Exception as e:
                logger.error(f"Callback error in on_state_change: {e}")

    def _log(self, text: str):
        if self.on_log:
            try:
                self.on_log(text)
            except Exception as e:
                logger.error(f"Callback error in on_log: {e}")
