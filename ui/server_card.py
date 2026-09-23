import customtkinter as ctk
from typing import Callable, Optional
from vpngate_client import VpnServer

class ServerCard(ctk.CTkFrame):
    """Interactive card widget representing a single VPN Gate server."""

    def __init__(
        self,
        master,
        server: VpnServer,
        on_select: Callable[[VpnServer], None],
        on_quick_connect: Optional[Callable[[VpnServer], None]] = None,
        is_selected: bool = False,
        **kwargs
    ):
        border_width = 2 if is_selected else 1
        border_color = ("#3B82F6", "#60A5FA") if is_selected else ("#E2E8F0", "#334155")
        fg_color = ("#EFF6FF", "#1E293B") if is_selected else ("#FFFFFF", "#0F172A")

        super().__init__(
            master,
            corner_radius=10,
            border_width=border_width,
            border_color=border_color,
            fg_color=fg_color,
            cursor="hand2",
            **kwargs
        )

        self.server = server
        self.on_select = on_select
        self.is_selected = is_selected

        self._build_ui()
        self._bind_click(self)

    def _build_ui(self):
        # Configure columns: [Flag & Info (weight=1)] [Ping & Speed] [Connect Button]
        self.grid_columnconfigure(0, weight=1)
        self.grid_columnconfigure(1, weight=0)
        self.grid_columnconfigure(2, weight=0)

        # Left: Country info and IP
        info_frame = ctk.CTkFrame(self, fg_color="transparent")
        info_frame.grid(row=0, column=0, padx=(12, 8), pady=10, sticky="w")
        self._bind_click(info_frame)

        # Flag + Country Name
        title_text = f"{self.server.flag}  {self.server.country_long}"
        self.lbl_title = ctk.CTkLabel(
            info_frame,
            text=title_text,
            font=ctk.CTkFont(size=14, weight="bold"),
            anchor="w"
        )
        self.lbl_title.pack(anchor="w")
        self._bind_click(self.lbl_title)

        # Subtitle: IP, Protocol, Sessions
        sub_text = f"{self.server.ip}:{self.server.port} • {self.server.proto} • {self.server.num_vpn_sessions} sessions"
        self.lbl_sub = ctk.CTkLabel(
            info_frame,
            text=sub_text,
            font=ctk.CTkFont(size=11),
            text_color=("gray40", "gray65"),
            anchor="w"
        )
        self.lbl_sub.pack(anchor="w")
        self._bind_click(self.lbl_sub)

        # Middle: Metrics (Ping & Speed badges)
        metrics_frame = ctk.CTkFrame(self, fg_color="transparent")
        metrics_frame.grid(row=0, column=1, padx=8, pady=10, sticky="e")
        self._bind_click(metrics_frame)

        # Speed badge
        speed_color = ("#0284C7", "#38BDF8")
        self.lbl_speed = ctk.CTkLabel(
            metrics_frame,
            text=f"⚡ {self.server.speed} Mbps",
            font=ctk.CTkFont(size=12, weight="bold"),
            text_color=speed_color
        )
        self.lbl_speed.pack(anchor="e")
        self._bind_click(self.lbl_speed)

        # Ping badge
        ping_val = self.server.ping
        if ping_val <= 60:
            ping_color = ("#16A34A", "#4ADE80")  # Green
        elif ping_val <= 150:
            ping_color = ("#D97706", "#FBBF24")  # Orange
        else:
            ping_color = ("#DC2626", "#F87171")  # Red

        self.lbl_ping = ctk.CTkLabel(
            metrics_frame,
            text=f"📶 {ping_val} ms",
            font=ctk.CTkFont(size=11),
            text_color=ping_color
        )
        self.lbl_ping.pack(anchor="e")
        self._bind_click(self.lbl_ping)

    def _bind_click(self, widget):
        widget.bind("<Button-1>", lambda e: self.on_select(self.server))

    def set_selected(self, selected: bool):
        self.is_selected = selected
        if selected:
            self.configure(
                border_width=2,
                border_color=("#3B82F6", "#60A5FA"),
                fg_color=("#EFF6FF", "#1E293B")
            )
        else:
            self.configure(
                border_width=1,
                border_color=("#E2E8F0", "#334155"),
                fg_color=("#FFFFFF", "#0F172A")
            )
