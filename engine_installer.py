import os
import sys
import time
import tempfile
import requests
import subprocess
import threading
import logging
from typing import Callable, Optional

logger = logging.getLogger("vpngate.installer")

OPENVPN_MSI_URL = "https://swupdate.openvpn.org/community/releases/OpenVPN-2.7.7-I001-amd64.msi"

class EngineInstaller:
    """Handles downloading and silent installation of the OpenVPN engine."""

    def __init__(self):
        self._is_installing = False

    @property
    def is_installing(self) -> bool:
        return self._is_installing

    def install_async(
        self,
        on_progress: Optional[Callable[[float, str], None]] = None,
        on_status: Optional[Callable[[str], None]] = None,
        on_complete: Optional[Callable[[bool, str], None]] = None
    ):
        """Runs the installation in a background thread."""
        if self._is_installing:
            return

        self._is_installing = True
        thread = threading.Thread(
            target=self._worker,
            args=(on_progress, on_status, on_complete),
            daemon=True
        )
        thread.start()

    def _worker(
        self,
        on_progress: Optional[Callable[[float, str], None]],
        on_status: Optional[Callable[[str], None]],
        on_complete: Optional[Callable[[bool, str], None]]
    ):
        temp_msi = os.path.join(tempfile.gettempdir(), f"OpenVPN-Setup-{int(time.time())}.msi")
        success = False
        message = ""

        try:
            # ----------------------------------------------------
            # Step 1: Download MSI package
            # ----------------------------------------------------
            if on_status:
                on_status("Connecting to official OpenVPN release server...")
            if on_progress:
                on_progress(0.05, "Connecting...")

            resp = requests.get(OPENVPN_MSI_URL, stream=True, timeout=30)
            resp.raise_for_status()

            total_bytes = int(resp.headers.get("content-length", 5_865_472))
            downloaded = 0

            with open(temp_msi, "wb") as f:
                for chunk in resp.iter_content(chunk_size=32768):
                    if not chunk:
                        continue
                    f.write(chunk)
                    downloaded += len(chunk)

                    percent = min(downloaded / total_bytes, 1.0)
                    dl_mb = round(downloaded / (1024 * 1024), 1)
                    tot_mb = round(total_bytes / (1024 * 1024), 1)

                    if on_progress:
                        on_progress(0.1 + (percent * 0.5), f"Downloading: {dl_mb}MB / {tot_mb}MB")

            if on_status:
                on_status("Installing OpenVPN & Wintun adapter... (Click 'Yes' on the Windows prompt)")
            if on_progress:
                on_progress(0.7, "Running installer...")

            # ----------------------------------------------------
            # Step 2: Run msiexec silently with Administrator elevation
            # ----------------------------------------------------
            ps_command = (
                f"$msi = '{temp_msi}'; "
                f"$proc = Start-Process msiexec.exe -ArgumentList '/i', ('\"' + $msi + '\"'), '/quiet', '/norestart' -Verb RunAs -PassThru -Wait; "
                f"exit $proc.ExitCode"
            )

            res = subprocess.run(
                ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps_command],
                capture_output=True,
                text=True
            )

            # Exit code 0 or 3010 (reboot pending but installed)
            if res.returncode in (0, 3010):
                # Verify installation target
                candidates = [
                    r"C:\Program Files\OpenVPN\bin\openvpn.exe",
                    r"C:\Program Files (x86)\OpenVPN\bin\openvpn.exe"
                ]
                # Give filesystem a moment
                time.sleep(1)
                found = any(os.path.isfile(p) for p in candidates)

                if found:
                    success = True
                    message = "OpenVPN and Wintun driver installed successfully!"
                    if on_progress:
                        on_progress(1.0, "Completed!")
                    if on_status:
                        on_status(message)
                else:
                    success = True
                    message = "Installer completed. Restart the app if OpenVPN is not immediately detected."
            elif res.returncode == 1223:
                # User cancelled UAC prompt
                success = False
                message = "Installation was cancelled (Administrator approval was declined)."
                if on_status:
                    on_status(message)
            else:
                success = False
                message = f"Installer failed with exit code: {res.returncode}."
                if on_status:
                    on_status(message)

        except requests.RequestException as re:
            success = False
            message = f"Download failed: {re}"
            if on_status:
                on_status(message)
        except Exception as e:
            logger.error(f"Installation error: {e}")
            success = False
            message = f"Installation error: {e}"
            if on_status:
                on_status(message)
        finally:
            # Clean up temp MSI
            try:
                if os.path.exists(temp_msi):
                    os.remove(temp_msi)
            except Exception:
                pass

            self._is_installing = False
            if on_complete:
                on_complete(success, message)
