import os
import sys
import time
import threading
from typing import List, Optional
from tkinter import filedialog, messagebox

import customtkinter as ctk

from vpngate_client import VpnGateClient, VpnServer
from vpn_manager import VpnManager, ConnectionState
from ui.server_card import ServerCard
from ui.log_drawer import LogDrawer
from ui.download_dialog import OpenVpnHelpDialog

# Setup appearance
ctk.set_appearance_mode("Dark")
ctk.set_default_color_theme("blue")

class VpnGateApp(ctk.CTk):
    """Main Windows Native VPN Gate Desktop Application."""

    def __init__(self):
        super().__init__()

        self.title("VPN Gate Client for Windows")
        self.geometry("1100x740")
        self.minsize(920, 620)

        # Core logic managers
        self.client = VpnGateClient()
        self.manager = VpnManager()

        # State tracking
        self.all_servers: List[VpnServer] = []
        self.filtered_servers: List[VpnServer] = []
        self.selected_server: Optional[VpnServer] = None
        self.selected_country = "All"
        self.current_sort = "speed"
        self.connection_start_time: Optional[float] = None
        self.displayed_count = 50

        # Hook manager callbacks
        self.manager.on_state_change = self._on_vpn_state_change
        self.manager.on_log = self._on_vpn_log

        # Build UI layout
        self._build_layout()

        # Handle window close
        self.protocol("WM_DELETE_WINDOW", self._on_closing)

        # Initial load in background thread
        self.after(200, self.refresh_server_list)

        # Start 1-second timer loop for connection duration
        self._timer_tick()

    def _build_layout(self):
        # Configure root grid: [Sidebar (320px)] [Main Content (weight=1)]
        self.grid_columnconfigure(0, weight=0, minsize=320)
        self.grid_columnconfigure(1, weight=1)
        self.grid_rowconfigure(0, weight=1)

        # ----------------------------------------------------
        # 1. LEFT SIDEBAR
        # ----------------------------------------------------
        self.sidebar = ctk.CTkFrame(self, corner_radius=0, fg_color=("#F1F5F9", "#0F172A"))
        self.sidebar.grid(row=0, column=0, sticky="nsew", padx=0, pady=0)
        self.sidebar.grid_rowconfigure(3, weight=1)

        # App Brand Header
        brand_frame = ctk.CTkFrame(self.sidebar, fg_color="transparent")
        brand_frame.pack(fill="x", padx=18, pady=(18, 12))

        lbl_logo = ctk.CTkLabel(
            brand_frame,
            text="🛡️ VPN Gate Desktop",
            font=ctk.CTkFont(size=18, weight="bold")
        )
        lbl_logo.pack(anchor="w")

        lbl_sub = ctk.CTkLabel(
            brand_frame,
            text="Community Relay Network • OpenVPN",
            font=ctk.CTkFont(size=11),
            text_color=("gray40", "gray60")
        )
        lbl_sub.pack(anchor="w")

        # Engine Status Banner
        self.engine_frame = ctk.CTkFrame(self.sidebar, fg_color=("gray90", "#1E293B"), corner_radius=8)
        self.engine_frame.pack(fill="x", padx=16, pady=(0, 12))

        self.lbl_engine = ctk.CTkLabel(
            self.engine_frame,
            text="Checking OpenVPN...",
            font=ctk.CTkFont(size=11)
        )
        self.lbl_engine.pack(side="left", padx=10, pady=6)

        self.btn_engine_help = ctk.CTkButton(
            self.engine_frame,
            text="Setup",
            width=50,
            height=22,
            font=ctk.CTkFont(size=10),
            command=self._show_openvpn_help
        )
        self.btn_engine_help.pack(side="right", padx=8, pady=6)
        self._update_engine_status()

        # Connection Control Box
        self.conn_box = ctk.CTkFrame(self.sidebar, fg_color=("white", "#1E293B"), corner_radius=12)
        self.conn_box.pack(fill="x", padx=16, pady=6)

        # Status badge & timer
        status_row = ctk.CTkFrame(self.conn_box, fg_color="transparent")
        status_row.pack(fill="x", padx=14, pady=(12, 6))

        self.lbl_status_dot = ctk.CTkLabel(
            status_row,
            text="● Disconnected",
            font=ctk.CTkFont(size=13, weight="bold"),
            text_color=("gray50", "gray60")
        )
        self.lbl_status_dot.pack(side="left")

        self.lbl_duration = ctk.CTkLabel(
            status_row,
            text="",
            font=ctk.CTkFont(family="Consolas", size=11),
            text_color=("gray40", "gray60")
        )
        self.lbl_duration.pack(side="right")

        # Selected Server Summary
        self.lbl_selected_title = ctk.CTkLabel(
            self.conn_box,
            text="No server selected",
            font=ctk.CTkFont(size=14, weight="bold"),
            anchor="w"
        )
        self.lbl_selected_title.pack(fill="x", padx=14, pady=(4, 0))

        self.lbl_selected_info = ctk.CTkLabel(
            self.conn_box,
            text="Pick a relay from the list to connect",
            font=ctk.CTkFont(size=11),
            text_color=("gray40", "gray60"),
            anchor="w"
        )
        self.lbl_selected_info.pack(fill="x", padx=14, pady=(0, 12))

        # Big Connect / Disconnect Button
        self.btn_connect = ctk.CTkButton(
            self.conn_box,
            text="⚡ Connect",
            height=42,
            font=ctk.CTkFont(size=14, weight="bold"),
            fg_color=("#2563EB", "#3B82F6"),
            hover_color=("#1D4ED8", "#2563EB"),
            command=self.toggle_connection,
            state="disabled"
        )
        self.btn_connect.pack(fill="x", padx=14, pady=(0, 8))

        # Secondary Actions (Export .ovpn)
        self.btn_export = ctk.CTkButton(
            self.conn_box,
            text="📁 Export .OVPN Profile",
            height=28,
            font=ctk.CTkFont(size=11),
            fg_color=("gray85", "#334155"),
            text_color=("gray10", "white"),
            hover_color=("gray75", "#475569"),
            command=self.export_selected_ovpn,
            state="disabled"
        )
        self.btn_export.pack(fill="x", padx=14, pady=(0, 12))

        # Country Filter Sidebar Section
        lbl_countries = ctk.CTkLabel(
            self.sidebar,
            text="LOCATION FILTER",
            font=ctk.CTkFont(size=10, weight="bold"),
            text_color=("gray50", "gray50")
        )
        lbl_countries.pack(anchor="w", padx=18, pady=(16, 4))

        self.country_scroll = ctk.CTkScrollableFrame(
            self.sidebar,
            fg_color="transparent",
            height=240
        )
        self.country_scroll.pack(fill="both", expand=True, padx=12, pady=(0, 12))

        # ----------------------------------------------------
        # 2. MAIN CONTENT (RIGHT PANEL)
        # ----------------------------------------------------
        self.main_frame = ctk.CTkFrame(self, fg_color="transparent")
        self.main_frame.grid(row=0, column=1, sticky="nsew", padx=16, pady=16)
        self.main_frame.grid_rowconfigure(1, weight=1)
        self.main_frame.grid_columnconfigure(0, weight=1)

        # Top Control Toolbar
        toolbar = ctk.CTkFrame(self.main_frame, fg_color="transparent")
        toolbar.grid(row=0, column=0, sticky="ew", pady=(0, 10))
        toolbar.grid_columnconfigure(0, weight=1)

        # Search Box
        self.search_entry = ctk.CTkEntry(
            toolbar,
            placeholder_text="🔍 Search country, IP, or operator...",
            height=36,
            corner_radius=8
        )
        self.search_entry.grid(row=0, column=0, sticky="ew", padx=(0, 10))
        self.search_entry.bind("<KeyRelease>", lambda e: self.apply_filters())

        # Sort Dropdown
        self.sort_var = ctk.StringVar(value="Highest Speed")
        self.sort_menu = ctk.CTkOptionMenu(
            toolbar,
            values=["Highest Speed", "Lowest Ping", "Most Sessions", "Score"],
            variable=self.sort_var,
            command=self._on_sort_change,
            height=36,
            width=140
        )
        self.sort_menu.grid(row=0, column=1, padx=(0, 10))

        # Refresh Button
        self.btn_refresh = ctk.CTkButton(
            toolbar,
            text="🔄 Refresh",
            width=90,
            height=36,
            font=ctk.CTkFont(size=12, weight="bold"),
            command=self.refresh_server_list
        )
        self.btn_refresh.grid(row=0, column=2)

        # Server List Scrollable Container
        self.servers_container = ctk.CTkScrollableFrame(
            self.main_frame,
            fg_color="transparent",
            corner_radius=0
        )
        self.servers_container.grid(row=1, column=0, sticky="nsew", pady=(0, 10))
        self.servers_container.grid_columnconfigure(0, weight=1)

        # Loading / Status message
        self.lbl_server_status = ctk.CTkLabel(
            self.servers_container,
            text="Loading servers from VPN Gate network...",
            font=ctk.CTkFont(size=14),
            text_color=("gray50", "gray50")
        )
        self.lbl_server_status.pack(pady=40)

        # Bottom: Collapsible Log Drawer
        self.log_drawer = LogDrawer(self.main_frame)
        self.log_drawer.grid(row=2, column=0, sticky="ew")

    # ----------------------------------------------------
    # UI Logic & Data Rendering
    # ----------------------------------------------------
    def _update_engine_status(self):
        installed = self.manager.is_engine_installed()
        if installed:
            self.lbl_engine.configure(
                text="✅ OpenVPN Ready",
                text_color=("#16A34A", "#4ADE80")
            )
            self.btn_engine_help.configure(
                text="Details",
                fg_color=("gray80", "#334155"),
                hover_color=("gray70", "#475569")
            )
        else:
            self.lbl_engine.configure(
                text="⚠️ OpenVPN Missing",
                text_color=("#D97706", "#FBBF24")
            )
            self.btn_engine_help.configure(
                text="⚡ Install",
                fg_color=("#16A34A", "#22C55E"),
                hover_color=("#15803D", "#16A34A")
            )

    def _show_openvpn_help(self):
        OpenVpnHelpDialog(self)

    def refresh_server_list(self):
        """Fetches servers in a background worker thread."""
        self.btn_refresh.configure(state="disabled", text="⏳ Loading...")
        self.lbl_server_status.configure(text="Fetching live relay servers from VPN Gate...")

        def worker():
            try:
                servers = self.client.fetch_servers(timeout=15, force_refresh=True)
                self.after(0, lambda: self._on_fetch_success(servers))
            except Exception as e:
                self.after(0, lambda: self._on_fetch_error(str(e)))

        threading.Thread(target=worker, daemon=True).start()

    def _on_fetch_success(self, servers: List[VpnServer]):
        self.all_servers = servers
        self.btn_refresh.configure(state="normal", text="🔄 Refresh")
        self._populate_country_filters()
        self.apply_filters()
        self.log_drawer.append_log(f"Successfully loaded {len(servers)} live servers across {len(self.client.get_countries())} countries.")

    def _on_fetch_error(self, err_msg: str):
        self.btn_refresh.configure(state="normal", text="🔄 Refresh")
        self.lbl_server_status.configure(text=f"Failed to fetch servers: {err_msg}")
        self.log_drawer.append_log(f"API Fetch Error: {err_msg}")

    def _populate_country_filters(self):
        # Clear existing filter buttons
        for w in self.country_scroll.winfo_children():
            w.destroy()

        countries = self.client.get_countries()

        # "All" button
        btn_all = ctk.CTkButton(
            self.country_scroll,
            text=f"🌍  All ({len(self.all_servers)})",
            height=28,
            anchor="w",
            font=ctk.CTkFont(size=12),
            fg_color=("#3B82F6", "#2563EB") if self.selected_country == "All" else "transparent",
            text_color="white" if self.selected_country == "All" else ("gray20", "gray80"),
            hover_color=("#60A5FA", "#1E3A8A"),
            command=lambda: self._select_country_filter("All")
        )
        btn_all.pack(fill="x", pady=2)

        # Individual countries
        for country, count in countries.items():
            # Get flag
            flag = "🌐"
            for s in self.all_servers:
                if s.country_long == country:
                    flag = s.flag
                    break

            is_sel = self.selected_country == country
            btn = ctk.CTkButton(
                self.country_scroll,
                text=f"{flag}  {country} ({count})",
                height=28,
                anchor="w",
                font=ctk.CTkFont(size=12),
                fg_color=("#3B82F6", "#2563EB") if is_sel else "transparent",
                text_color="white" if is_sel else ("gray20", "gray80"),
                hover_color=("#60A5FA", "#1E3A8A"),
                command=lambda c=country: self._select_country_filter(c)
            )
            btn.pack(fill="x", pady=1)

    def _select_country_filter(self, country: str):
        self.selected_country = country
        self._populate_country_filters()
        self.apply_filters()

    def _on_sort_change(self, choice: str):
        mapping = {
            "Highest Speed": "speed",
            "Lowest Ping": "ping",
            "Most Sessions": "sessions",
            "Score": "score"
        }
        self.current_sort = mapping.get(choice, "speed")
        self.apply_filters()

    def apply_filters(self):
        query = self.search_entry.get().strip()
        self.filtered_servers = self.client.filter_and_sort(
            search_query=query,
            country=self.selected_country,
            sort_by=self.current_sort
        )
        self._render_server_list()

    def _render_server_list(self):
        for w in self.servers_container.winfo_children():
            w.destroy()

        if not self.filtered_servers:
            lbl_empty = ctk.CTkLabel(
                self.servers_container,
                text="No servers matched your filter or search query.",
                font=ctk.CTkFont(size=13),
                text_color=("gray40", "gray60")
            )
            lbl_empty.pack(pady=30)
            return

        # Render top N servers for buttery smooth UI performance
        to_render = self.filtered_servers[:self.displayed_count]
        for server in to_render:
            is_selected = (self.selected_server is not None and self.selected_server.ip == server.ip)
            card = ServerCard(
                self.servers_container,
                server=server,
                on_select=self._on_server_selected,
                is_selected=is_selected
            )
            card.pack(fill="x", pady=4, padx=4)

        if len(self.filtered_servers) > self.displayed_count:
            btn_more = ctk.CTkButton(
                self.servers_container,
                text=f"Show More ({len(self.filtered_servers) - self.displayed_count} remaining)",
                height=32,
                fg_color=("gray80", "#334155"),
                command=self._show_more_servers
            )
            btn_more.pack(pady=10)

    def _show_more_servers(self):
        self.displayed_count += 40
        self._render_server_list()

    def _on_server_selected(self, server: VpnServer):
        self.selected_server = server

        # Update sidebar
        self.lbl_selected_title.configure(text=f"{server.flag} {server.country_long}")
        self.lbl_selected_info.configure(
            text=f"IP: {server.ip} • Speed: {server.speed} Mbps • Ping: {server.ping} ms"
        )
        self.btn_export.configure(state="normal")

        # If disconnected, enable connect button
        if self.manager.state == ConnectionState.DISCONNECTED:
            self.btn_connect.configure(state="normal", text="⚡ Connect")

        # Re-render cards to show selection border
        self._render_server_list()

    # ----------------------------------------------------
    # VPN Connection Lifecycle
    # ----------------------------------------------------
    def toggle_connection(self):
        if self.manager.state == ConnectionState.CONNECTED:
            self.manager.disconnect()
        elif self.manager.state == ConnectionState.DISCONNECTED:
            if not self.selected_server:
                return

            if not self.manager.is_engine_installed():
                self._show_openvpn_help()
                return

            self.manager.connect(self.selected_server)

    def _on_vpn_state_change(self, state: ConnectionState, message: Optional[str]):
        """Thread-safe UI updates on VPN state changes."""
        self.after(0, lambda: self._apply_state_change_ui(state, message))

    def _apply_state_change_ui(self, state: ConnectionState, message: Optional[str]):
        if state == ConnectionState.CONNECTED:
            self.connection_start_time = time.time()
            self.lbl_status_dot.configure(text="● Connected", text_color=("#16A34A", "#4ADE80"))
            self.btn_connect.configure(
                state="normal",
                text="🛑 Disconnect",
                fg_color=("#DC2626", "#EF4444"),
                hover_color=("#B91C1C", "#DC2626")
            )
        elif state == ConnectionState.CONNECTING:
            self.lbl_status_dot.configure(text="⏳ Connecting...", text_color=("#D97706", "#FBBF24"))
            self.btn_connect.configure(state="disabled", text="Connecting...")
        elif state == ConnectionState.DISCONNECTING:
            self.lbl_status_dot.configure(text="⏳ Disconnecting...", text_color=("gray50", "gray50"))
            self.btn_connect.configure(state="disabled", text="Disconnecting...")
        elif state == ConnectionState.ERROR:
            self.connection_start_time = None
            self.lbl_duration.configure(text="")
            self.lbl_status_dot.configure(text="⚠️ Error", text_color=("#DC2626", "#F87171"))
            self.btn_connect.configure(
                state="normal" if self.selected_server else "disabled",
                text="⚡ Retry Connect",
                fg_color=("#2563EB", "#3B82F6"),
                hover_color=("#1D4ED8", "#2563EB")
            )
            if message and "Administrator permissions required" in message:
                messagebox.showerror(
                    "Administrator Elevation Required",
                    "Windows requires Administrator permissions to modify network adapter routing tables.\n\n"
                    "Please run the app or terminal as Administrator."
                )
        elif state == ConnectionState.DISCONNECTED:
            self.connection_start_time = None
            self.lbl_duration.configure(text="")
            self.lbl_status_dot.configure(text="● Disconnected", text_color=("gray50", "gray60"))
            self.btn_connect.configure(
                state="normal" if self.selected_server else "disabled",
                text="⚡ Connect",
                fg_color=("#2563EB", "#3B82F6"),
                hover_color=("#1D4ED8", "#2563EB")
            )

    def _on_vpn_log(self, text: str):
        self.after(0, lambda: self.log_drawer.append_log(text))

    def _timer_tick(self):
        """Updates the connected duration timer every second."""
        if self.manager.state == ConnectionState.CONNECTED and self.connection_start_time:
            elapsed = int(time.time() - self.connection_start_time)
            mins, secs = divmod(elapsed, 60)
            hours, mins = divmod(mins, 60)
            self.lbl_duration.configure(text=f"{hours:02d}:{mins:02d}:{secs:02d}")

        self.after(1000, self._timer_tick)

    def export_selected_ovpn(self):
        if not self.selected_server:
            return

        default_name = f"vpngate_{self.selected_server.country_short}_{self.selected_server.ip}.ovpn"
        file_path = filedialog.asksaveasfilename(
            parent=self,
            title="Export OpenVPN Configuration",
            initialfile=default_name,
            filetypes=[("OpenVPN Config", "*.ovpn"), ("All Files", "*.*")]
        )
        if file_path:
            success = self.manager.export_config(self.selected_server, file_path)
            if success:
                messagebox.showinfo(
                    "Config Exported",
                    f"Saved profile for {self.selected_server.country_long} to:\n{file_path}\n\n"
                    "Credentials:\nUsername: vpn\nPassword: vpn"
                )
                self.log_drawer.append_log(f"Exported .ovpn config: {file_path}")
            else:
                messagebox.showerror("Export Failed", "Could not export .ovpn profile.")

    def _on_closing(self):
        """Clean up on window close."""
        if self.manager.state in (ConnectionState.CONNECTED, ConnectionState.CONNECTING):
            self.manager.disconnect()
        self.destroy()

if __name__ == "__main__":
    app = VpnGateApp()
    app.mainloop()
