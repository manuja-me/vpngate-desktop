import customtkinter as ctk

class LogDrawer(ctk.CTkFrame):
    """Collapsible terminal-style log output drawer for OpenVPN."""

    def __init__(self, master, **kwargs):
        super().__init__(master, corner_radius=8, fg_color=("gray95", "#0F172A"), **kwargs)

        self.is_expanded = True
        self.auto_scroll = True

        self._build_ui()

    def _build_ui(self):
        # Header bar
        self.header_frame = ctk.CTkFrame(self, fg_color="transparent", height=32)
        self.header_frame.pack(fill="x", padx=10, pady=(6, 4))

        # Title
        self.lbl_title = ctk.CTkLabel(
            self.header_frame,
            text="📋 Tunnel Logs",
            font=ctk.CTkFont(size=12, weight="bold")
        )
        self.lbl_title.pack(side="left")

        # Controls on right: Toggle, Auto-scroll, Clear
        self.btn_clear = ctk.CTkButton(
            self.header_frame,
            text="Clear",
            width=50,
            height=22,
            font=ctk.CTkFont(size=11),
            fg_color=("gray80", "#334155"),
            text_color=("gray10", "white"),
            command=self.clear_logs
        )
        self.btn_clear.pack(side="right", padx=(4, 0))

        self.btn_toggle = ctk.CTkButton(
            self.header_frame,
            text="▲ Collapse",
            width=80,
            height=22,
            font=ctk.CTkFont(size=11),
            fg_color=("gray80", "#334155"),
            text_color=("gray10", "white"),
            command=self.toggle_collapse
        )
        self.btn_toggle.pack(side="right", padx=4)

        # Content area (Log text box)
        self.textbox = ctk.CTkTextbox(
            self,
            height=120,
            font=ctk.CTkFont(family="Consolas", size=10),
            corner_radius=6,
            fg_color=("white", "#020617"),
            text_color=("#1E293B", "#94A3B8"),
            wrap="none"
        )
        self.textbox.pack(fill="both", expand=True, padx=10, pady=(0, 8))

    def append_log(self, text: str):
        """Appends a new line to the log terminal safely."""
        self.textbox.configure(state="normal")
        self.textbox.insert("end", text + "\n")
        if self.auto_scroll:
            self.textbox.see("end")
        self.textbox.configure(state="disabled")

    def clear_logs(self):
        self.textbox.configure(state="normal")
        self.textbox.delete("1.0", "end")
        self.textbox.configure(state="disabled")

    def toggle_collapse(self):
        if self.is_expanded:
            self.textbox.pack_forget()
            self.btn_toggle.configure(text="▼ Expand")
            self.is_expanded = False
        else:
            self.textbox.pack(fill="both", expand=True, padx=10, pady=(0, 8))
            self.btn_toggle.configure(text="▲ Collapse")
            self.is_expanded = True
