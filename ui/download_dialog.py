import os
import webbrowser
import customtkinter as ctk
from engine_installer import EngineInstaller

class OpenVpnHelpDialog(ctk.CTkToplevel):
    """Interactive in-app OpenVPN installer and setup guide."""

    def __init__(self, parent):
        super().__init__(parent)

        self.title("OpenVPN Engine Setup")
        self.geometry("540x440")
        self.minsize(500, 400)
        self.resizable(False, False)

        # Center on parent
        self.transient(parent)
        self.grab_set()

        self.installer = EngineInstaller()

        self._build_ui()

    def _build_ui(self):
        container = ctk.CTkFrame(self, fg_color="transparent")
        container.pack(fill="both", expand=True, padx=24, pady=20)

        # Header
        lbl_title = ctk.CTkLabel(
            container,
            text="🛡️ OpenVPN Engine Setup",
            font=ctk.CTkFont(size=18, weight="bold")
        )
        lbl_title.pack(anchor="w", pady=(0, 4))

        lbl_desc = ctk.CTkLabel(
            container,
            text="To intercept and route network traffic through VPN Gate, Windows requires the OpenVPN engine and the Wintun adapter driver.",
            font=ctk.CTkFont(size=12),
            text_color=("gray30", "gray70"),
            justify="left",
            wraplength=480
        )
        lbl_desc.pack(anchor="w", pady=(0, 14))

        # ====================================================
        # Option 1: Embedded 1-Click Silent Automated Installer
        # ====================================================
        self.card_auto = ctk.CTkFrame(container, fg_color=("#EFF6FF", "#1E293B"), corner_radius=10, border_width=1, border_color=("#3B82F6", "#2563EB"))
        self.card_auto.pack(fill="x", pady=6)

        auto_header = ctk.CTkFrame(self.card_auto, fg_color="transparent")
        auto_header.pack(fill="x", padx=14, pady=(10, 4))

        lbl_opt1 = ctk.CTkLabel(
            auto_header,
            text="⚡ Method 1: Automatic 1-Click Install (Embedded)",
            font=ctk.CTkFont(size=13, weight="bold"),
            text_color=("#1D4ED8", "#60A5FA")
        )
        lbl_opt1.pack(side="left")

        # Status text
        self.lbl_install_status = ctk.CTkLabel(
            self.card_auto,
            text="Downloads official signed MSI & silently configures Wintun driver.",
            font=ctk.CTkFont(size=11),
            text_color=("gray40", "gray65"),
            anchor="w"
        )
        self.lbl_install_status.pack(fill="x", padx=14, pady=(0, 8))

        # Progress bar
        self.progress_bar = ctk.CTkProgressBar(self.card_auto, height=8)
        self.progress_bar.set(0.0)
        self.progress_bar.pack(fill="x", padx=14, pady=(0, 10))

        # Action button
        self.btn_auto_install = ctk.CTkButton(
            self.card_auto,
            text="⚡ Install OpenVPN Automatically",
            height=34,
            font=ctk.CTkFont(size=12, weight="bold"),
            fg_color=("#16A34A", "#22C55E"),
            hover_color=("#15803D", "#16A34A"),
            command=self._start_embedded_install
        )
        self.btn_auto_install.pack(anchor="w", padx=14, pady=(0, 12))

        # ====================================================
        # Option 2: Terminal (Winget)
        # ====================================================
        opt2_frame = ctk.CTkFrame(container, fg_color=("gray90", "#0F172A"), corner_radius=8)
        opt2_frame.pack(fill="x", pady=5)

        lbl_opt2 = ctk.CTkLabel(
            opt2_frame,
            text="Method 2: Windows Package Manager (Winget)",
            font=ctk.CTkFont(size=11, weight="bold")
        )
        lbl_opt2.pack(anchor="w", padx=12, pady=(6, 2))

        cmd_box = ctk.CTkEntry(
            opt2_frame,
            height=26,
            font=ctk.CTkFont(family="Consolas", size=10)
        )
        cmd_box.insert(0, "winget install --id OpenVPNTechnologies.OpenVPN -e --accept-package-agreements")
        cmd_box.configure(state="readonly")
        cmd_box.pack(fill="x", padx=12, pady=(0, 8))

        # ====================================================
        # Option 3: Official Website
        # ====================================================
        opt3_frame = ctk.CTkFrame(container, fg_color=("gray90", "#0F172A"), corner_radius=8)
        opt3_frame.pack(fill="x", pady=5)

        opt3_inner = ctk.CTkFrame(opt3_frame, fg_color="transparent")
        opt3_inner.pack(fill="x", padx=12, pady=6)

        lbl_opt3 = ctk.CTkLabel(
            opt3_inner,
            text="Method 3: Official Community Web Installer",
            font=ctk.CTkFont(size=11, weight="bold")
        )
        lbl_opt3.pack(side="left")

        btn_download = ctk.CTkButton(
            opt3_inner,
            text="🌐 Download from openvpn.net",
            height=24,
            width=160,
            font=ctk.CTkFont(size=10),
            fg_color=("#2563EB", "#3B82F6"),
            command=lambda: webbrowser.open("https://openvpn.net/community-downloads/")
        )
        btn_download.pack(side="right")

        # Bottom Bar: Refresh and Close
        bottom_bar = ctk.CTkFrame(container, fg_color="transparent")
        bottom_bar.pack(fill="x", pady=(12, 0))

        btn_recheck = ctk.CTkButton(
            bottom_bar,
            text="🔄 Re-check Engine",
            width=130,
            command=self._recheck_and_close
        )
        btn_recheck.pack(side="left")

        btn_close = ctk.CTkButton(
            bottom_bar,
            text="Close",
            width=80,
            fg_color=("gray75", "#475569"),
            command=self.destroy
        )
        btn_close.pack(side="right")

    def _start_embedded_install(self):
        """Launches the embedded background installer."""
        self.btn_auto_install.configure(state="disabled", text="⏳ Preparing installation...")
        self.progress_bar.set(0.05)

        def on_prog(pct: float, label: str):
            self.after(0, lambda: self._update_progress_ui(pct, label))

        def on_stat(status: str):
            self.after(0, lambda: self.lbl_install_status.configure(text=status))

        def on_done(success: bool, msg: str):
            self.after(0, lambda: self._on_install_finished(success, msg))

        self.installer.install_async(
            on_progress=on_prog,
            on_status=on_stat,
            on_complete=on_done
        )

    def _update_progress_ui(self, pct: float, label: str):
        self.progress_bar.set(pct)
        self.btn_auto_install.configure(text=f"⏳ {label}")

    def _on_install_finished(self, success: bool, msg: str):
        if success:
            self.progress_bar.set(1.0)
            self.lbl_install_status.configure(
                text=f"✅ {msg}",
                text_color=("#16A34A", "#4ADE80")
            )
            self.btn_auto_install.configure(
                state="disabled",
                text="✅ Installation Complete",
                fg_color=("#16A34A", "#22C55E")
            )
            # Recheck engine in parent app
            if hasattr(self.master, "_update_engine_status"):
                self.master._update_engine_status()
        else:
            self.progress_bar.set(0.0)
            self.lbl_install_status.configure(
                text=f"⚠️ {msg}",
                text_color=("#DC2626", "#F87171")
            )
            self.btn_auto_install.configure(
                state="normal",
                text="⚡ Retry Automatic Install",
                fg_color=("#2563EB", "#3B82F6")
            )

    def _recheck_and_close(self):
        if hasattr(self.master, "_update_engine_status"):
            self.master._update_engine_status()
        self.destroy()
