from PySide6.QtWidgets import QWidget, QVBoxLayout, QHBoxLayout, QLabel, QFrame
from PySide6.QtCore import Qt, Signal
from settings_manager import load_settings, save_settings


class SettingsToggle(QWidget):
    toggled = Signal(bool)

    def __init__(self, initial=True, parent=None):
        super().__init__(parent)
        self.setFixedSize(52, 28)
        self._checked = initial
        self.setCursor(Qt.PointingHandCursor)
        self.setStyleSheet("border: none;")

    def is_checked(self):
        return self._checked

    def set_checked(self, value):
        self._checked = value
        self.update()

    def mousePressEvent(self, event):
        self._checked = not self._checked
        self.toggled.emit(self._checked)
        self.update()

    def paintEvent(self, event):
        from PySide6.QtGui import QPainter, QColor, QBrush
        from PySide6.QtCore import QRectF
        p = QPainter(self)
        p.setRenderHint(QPainter.Antialiasing)
        r = self.rect()
        track_rect = QRectF(0, 4, r.width(), r.height() - 8)
        p.setBrush(QBrush(QColor("#6366f1") if self._checked else QColor("#2d2e3a")))
        p.setPen(Qt.NoPen)
        p.drawRoundedRect(track_rect, 10, 10)
        thumb_x = r.width() - 26 if self._checked else 2
        thumb_rect = QRectF(thumb_x, 2, 24, 24)
        p.setBrush(QBrush(QColor("#ffffff")))
        p.drawEllipse(thumb_rect)
        p.end()


class SettingRow(QWidget):
    def __init__(self, title, subtitle, widget=None, parent=None):
        super().__init__(parent)
        layout = QHBoxLayout(self)
        layout.setContentsMargins(0, 10, 0, 10)

        text_box = QVBoxLayout()
        title_lbl = QLabel(title)
        title_lbl.setStyleSheet("color: #ffffff; font-size: 14px; font-weight: 600;")
        sub_lbl = QLabel(subtitle)
        sub_lbl.setStyleSheet("color: #6b7280; font-size: 12px;")
        text_box.addWidget(title_lbl)
        text_box.addWidget(sub_lbl)

        layout.addLayout(text_box)
        layout.addStretch()
        if widget:
            layout.addWidget(widget)


class SettingsView(QWidget):
    close_to_tray_changed = Signal(bool)

    def __init__(self, parent=None):
        super().__init__(parent)

        # ── Load persisted settings ──────────────────────────────
        self._settings = load_settings()

        layout = QVBoxLayout(self)
        layout.setContentsMargins(30, 20, 30, 20)
        layout.setSpacing(0)

        # Header
        header = QLabel("<h2>Settings</h2>")
        header.setStyleSheet("color: white; margin-bottom: 20px;")
        layout.addWidget(header)

        # ── Section: Behaviour ──────────────────────────────────
        self._add_section(layout, "Behaviour")

        self.close_to_tray_toggle = SettingsToggle(initial=self._settings["close_to_tray"])
        self.close_to_tray_toggle.toggled.connect(self._on_close_to_tray)
        layout.addWidget(SettingRow(
            "Minimize to Tray on Close",
            "When enabled, clicking X hides MooDeX to the system tray instead of quitting.",
            self.close_to_tray_toggle
        ))
        layout.addWidget(self._divider())

        self.autostart_toggle = SettingsToggle(initial=self._settings["autostart"])
        self.autostart_toggle.toggled.connect(self._on_autostart)
        layout.addWidget(SettingRow(
            "Launch at Windows Startup",
            "MooDeX will start minimized in the background when you log in.",
            self.autostart_toggle
        ))
        layout.addWidget(self._divider())

        self.launch_minimized_toggle = SettingsToggle(initial=self._settings["launch_minimized"])
        self.launch_minimized_toggle.toggled.connect(self._on_launch_minimized)
        layout.addWidget(SettingRow(
            "Launch Minimized",
            "When opened, MooDeX will start minimized to the system tray.",
            self.launch_minimized_toggle
        ))
        layout.addWidget(self._divider())

        # ── Section: Performance ─────────────────────────────────
        self._add_section(layout, "Performance")

        self.lazy_load_toggle = SettingsToggle(initial=self._settings["lazy_discover"])
        self.lazy_load_toggle.toggled.connect(self._on_lazy_discover)
        layout.addWidget(SettingRow(
            "Lazy-Load Discover Tab",
            "Only load Trending/Upcoming content when you visit the Discover tab.",
            self.lazy_load_toggle
        ))
        layout.addWidget(self._divider())

        self.cache_toggle = SettingsToggle(initial=self._settings["api_cache"])
        self.cache_toggle.toggled.connect(self._on_api_cache)
        layout.addWidget(SettingRow(
            "Enable API Cache",
            "Reduces startup time by loading previously fetched game data from disk.",
            self.cache_toggle
        ))
        layout.addWidget(self._divider())

        # ── Bottom stretch ───────────────────────────────────────
        layout.addStretch()

        # ── Section: About ───────────────────────────────────────
        about_text = QLabel("MooDeX  ·  v1.6")
        about_text.setStyleSheet("color: #4b5563; font-size: 12px; margin: 8px 0 0px 0;")
        layout.addWidget(about_text, alignment=Qt.AlignBottom | Qt.AlignLeft)

    # ── Toggle handlers (save on every change) ───────────────────
    def _on_close_to_tray(self, val):
        self._settings["close_to_tray"] = val
        save_settings(self._settings)
        self.close_to_tray_changed.emit(val)

    def _on_autostart(self, val):
        self._settings["autostart"] = val
        save_settings(self._settings)

    def _on_launch_minimized(self, val):
        self._settings["launch_minimized"] = val
        save_settings(self._settings)

    def _on_lazy_discover(self, val):
        self._settings["lazy_discover"] = val
        save_settings(self._settings)

    def _on_api_cache(self, val):
        self._settings["api_cache"] = val
        save_settings(self._settings)

    def is_close_to_tray(self):
        return self._settings["close_to_tray"]

    # ── Helpers ──────────────────────────────────────────────────
    def _add_section(self, layout, title):
        lbl = QLabel(title.upper())
        lbl.setStyleSheet(
            "color: #6366f1; font-size: 11px; font-weight: 800; "
            "letter-spacing: 2px; margin-top: 18px; margin-bottom: 4px;"
        )
        layout.addWidget(lbl)

    def _divider(self):
        line = QFrame()
        line.setFrameShape(QFrame.HLine)
        line.setStyleSheet("background-color: #1f2029; max-height: 1px; border: none;")
        return line
