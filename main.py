import sys
import os
from PySide6.QtWidgets import (QApplication, QMainWindow, QWidget, QVBoxLayout, 
                               QHBoxLayout, QPushButton, QStackedWidget, QLabel,
                               QSystemTrayIcon, QMenu, QMessageBox, QDialog,
                               QDialogButtonBox, QRadioButton, QButtonGroup)
from PySide6.QtCore import Qt, QCoreApplication, QTimer
from PySide6.QtGui import QIcon, QPixmap, QAction
from data_manager import DataManager

class CloseDialog(QDialog):
    """One-time dialog asking the user how they want X to behave."""
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Close MooDeX")
        self.setFixedWidth(380)
        self.setStyleSheet("""
            QDialog { background-color: #15161c; color: white; }
            QLabel  { color: white; }
        """)
        layout = QVBoxLayout(self)
        layout.setContentsMargins(24, 24, 24, 24)
        layout.setSpacing(12)

        title = QLabel("<b style='font-size:16px;'>How should MooDeX close?</b>")
        layout.addWidget(title)

        self.tray_radio = QRadioButton("Minimize to system tray (keep tracking)")
        self.quit_radio = QRadioButton("Quit completely")
        self.tray_radio.setChecked(True)
        for r in [self.tray_radio, self.quit_radio]:
            r.setStyleSheet("color: #c0c0d0; font-size: 13px;")
        layout.addWidget(self.tray_radio)
        layout.addWidget(self.quit_radio)

        btns = QDialogButtonBox(QDialogButtonBox.Ok | QDialogButtonBox.Cancel)
        btns.setStyleSheet("""
            QPushButton {
                background: #6366f1; color: white; border-radius: 8px;
                padding: 6px 18px; font-weight: bold;
            }
            QPushButton:hover { background: #4f46e5; }
        """)
        btns.accepted.connect(self.accept)
        btns.rejected.connect(self.reject)
        layout.addWidget(btns)

    def minimize_to_tray(self):
        return self.tray_radio.isChecked()


class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("MooDeX")
        icon_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "Icon Logo", "icon.ico")
        if os.path.exists(icon_path):
            self.setWindowIcon(QIcon(icon_path))
        self.resize(1200, 800)

        # ── State ──────────────────────────────────────────────────
        from settings_manager import load_settings
        _saved = load_settings()
        self._close_to_tray   = _saved["close_to_tray"]
        self._discover_loaded = False

        # ── Data ───────────────────────────────────────────────────
        self.data_manager = DataManager()

        # ── System Tray ────────────────────────────────────────────
        self.tray_icon = QSystemTrayIcon(self)
        if os.path.exists(icon_path):
            self.tray_icon.setIcon(QIcon(icon_path))

        tray_menu = QMenu()
        restore_action = QAction("Open MooDeX", self)
        restore_action.triggered.connect(lambda: [self.showNormal(), self.activateWindow()])
        quit_action = QAction("Quit", self)
        quit_action.triggered.connect(self.quit_app)

        tray_menu.addAction(restore_action)
        tray_menu.addSeparator()
        tray_menu.addAction(quit_action)
        self.tray_icon.setContextMenu(tray_menu)
        self.tray_icon.activated.connect(self.on_tray_activated)
        self.tray_icon.show()

        # ── Process Monitor (background thread) ────────────────────
        from services.process_monitor import ProcessMonitor
        self.monitor = ProcessMonitor(self.data_manager)
        self.monitor.game_started.connect(
            lambda name: self.tray_icon.showMessage(
                "MooDeX Engine", f"▶ Started playing {name}",
                QSystemTrayIcon.Information, 3000))
        self.monitor.game_stopped.connect(
            lambda name, dur: self.tray_icon.showMessage(
                "MooDeX Engine", f"Session tracked: {int(dur/60)}m",
                QSystemTrayIcon.Information, 3000))
        self.monitor.start()

        # ── Global stylesheet ──────────────────────────────────────
        self.setStyleSheet("""
            QMainWindow { background-color: #0f1015; }
            QWidget {
                font-family: 'Segoe UI', -apple-system, BlinkMacSystemFont, Roboto, Helvetica, Arial, sans-serif;
            }
            QPushButton {
                background-color: transparent;
                color: #a0a0b0;
                border-radius: 8px;
                padding: 10px 15px;
                font-size: 14px;
                font-weight: 500;
                text-align: left;
            }
            QPushButton:hover { background-color: #1c1d24; color: #ffffff; }
            QPushButton:checked { background-color: #262730; color: #6366f1; font-weight: bold; }
            QLineEdit {
                background-color: #1c1d24;
                border: 1px solid #2d2e3a;
                border-radius: 8px;
                color: white;
                padding: 8px 12px;
                font-size: 14px;
            }
            QLineEdit:focus { border: 1px solid #6366f1; }
            QComboBox {
                background-color: #1c1d24;
                border: 1px solid #2d2e3a;
                border-radius: 8px;
                color: white;
                padding: 8px 12px;
                min-width: 120px;
            }
            QComboBox::drop-down { border: none; }
            QTabWidget::pane { border: none; background-color: transparent; margin-top: 10px; }
            QTabBar::tab {
                background: transparent; color: #a0a0b0;
                padding: 8px 16px; font-size: 14px; font-weight: 500;
                border-bottom: 2px solid transparent;
            }
            QTabBar::tab:hover { color: white; }
            QTabBar::tab:selected { color: #6366f1; border-bottom: 2px solid #6366f1; }
            QScrollBar:vertical {
                border: none; background: #0f1015;
                width: 8px; margin: 0px;
            }
            QScrollBar::handle:vertical {
                background: #2d2e3a; min-height: 20px; border-radius: 4px;
            }
            QScrollBar::handle:vertical:hover { background: #3e3f4f; }
            QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical { border: none; background: none; }
        """)

        # ── Layout ─────────────────────────────────────────────────
        main_widget = QWidget()
        main_layout = QHBoxLayout(main_widget)
        main_layout.setContentsMargins(0, 0, 0, 0)
        main_layout.setSpacing(0)
        self.setCentralWidget(main_widget)

        # Sidebar
        sidebar = QWidget()
        sidebar.setFixedWidth(240)
        sidebar.setStyleSheet("background-color: #15161c; border-right: 1px solid #1f2029;")
        sidebar_layout = QVBoxLayout(sidebar)
        sidebar_layout.setContentsMargins(20, 30, 20, 30)
        sidebar_layout.setSpacing(10)

        logo = QLabel()
        logo_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "Icon Logo", "logo with name.png")
        if os.path.exists(logo_path):
            pixmap = QPixmap(logo_path)
            logo.setPixmap(pixmap.scaled(180, 90, Qt.KeepAspectRatio, Qt.SmoothTransformation))
        else:
            logo.setText("<h2>🎮 MooDeX</h2>")
        logo.setStyleSheet("color: #ffffff; padding-bottom: 20px; border: none;")
        logo.setAlignment(Qt.AlignLeft | Qt.AlignVCenter)
        sidebar_layout.addWidget(logo)

        from icon_manager import IconManager
        from PySide6.QtCore import QSize as _QSize

        self.nav_buttons = []
        nav_items = [
            ("My Games",    "nav_mygames",  0),
            ("My Library",  "nav_library",  1),
            ("Discover",    "nav_discover", 2),
            ("Stats",       "nav_stats",    3),
            ("Settings",    "nav_settings", 4),
        ]
        for label, icon_key, idx in nav_items:
            btn = QPushButton(f"  {label}")
            btn.setCheckable(True)
            btn.setIcon(IconManager.get_instance().get_icon(icon_key))
            btn.setIconSize(_QSize(18, 18))
            if idx == 0:
                btn.setChecked(True)
            self.nav_buttons.append(btn)
            sidebar_layout.addWidget(btn)

        sidebar_layout.addStretch()

        add_btn = QPushButton("+ Add Game")
        add_btn.setStyleSheet("""
            QPushButton {
                background-color: #6366f1; color: white;
                border-radius: 8px; padding: 12px;
                font-weight: bold; text-align: center;
            }
            QPushButton:hover { background-color: #4f46e5; }
        """)
        sidebar_layout.addWidget(add_btn)
        main_layout.addWidget(sidebar)

        # Stacked Widget
        self.stack = QStackedWidget()
        self.stack.setStyleSheet("background-color: transparent;")
        main_layout.addWidget(self.stack)

        # ── Views (lazy on Discover) ────────────────────────────────
        from views.library_view import LibraryView
        from views.stats_view import StatsView
        from views.my_games_view import MyGamesView
        from views.settings_view import SettingsView

        self.my_games_view = MyGamesView(self.data_manager)
        self.discover_view = None          # created on first visit
        self.stats_view    = StatsView(self.data_manager)
        self.library_view  = LibraryView(self.data_manager)
        self.settings_view = SettingsView()

        # Placeholder for Discover slot
        self._discover_placeholder = QWidget()
        self._discover_placeholder.setStyleSheet("background: transparent;")

        self.stack.addWidget(self.my_games_view)          # index 0
        self.stack.addWidget(self.library_view)           # index 1
        self.stack.addWidget(self._discover_placeholder)  # index 2
        self.stack.addWidget(self.stats_view)             # index 3
        self.stack.addWidget(self.settings_view)          # index 4

        # Settings: wire close-to-tray toggle
        self.settings_view.close_to_tray_changed.connect(
            lambda val: setattr(self, '_close_to_tray', val))

        # Nav helper
        def set_nav_active(index):
            for i, bn in enumerate(self.nav_buttons):
                bn.setChecked(i == index)
            # Lazy-load Discover on first visit (slot 2)
            if index == 2 and not self._discover_loaded:
                self._load_discover()
            self.stack.setCurrentIndex(index)

        for i, (_, _icon, idx) in enumerate(nav_items):
            self.nav_buttons[i].clicked.connect(lambda checked=False, n=i: set_nav_active(n))

        add_btn.clicked.connect(lambda: [set_nav_active(2),
                                         (self.discover_view.search_input.setFocus()
                                          if self.discover_view else None)])

    # ── Lazy Discover loader ──────────────────────────────────────────
    def _load_discover(self):
        from views.discover_view import DiscoverView
        self.discover_view = DiscoverView(self.data_manager)
        self.stack.removeWidget(self._discover_placeholder)
        self.stack.insertWidget(2, self.discover_view)  # slot 2
        self._discover_loaded = True

    # ── Tray ─────────────────────────────────────────────────────────
    def on_tray_activated(self, reason):
        if reason == QSystemTrayIcon.DoubleClick:
            self.showNormal()
            self.activateWindow()

    # ── Close event ──────────────────────────────────────────────────
    def closeEvent(self, event):
        if self._close_to_tray:
            event.ignore()
            self.hide()
            self.tray_icon.showMessage(
                "MooDeX is still running",
                "Tracking your playtime in the background. Use the tray icon to quit.",
                QSystemTrayIcon.Information, 2500)
        else:
            event.accept()
            self.quit_app()

    # ── Quit cleanly ─────────────────────────────────────────────────
    def quit_app(self):
        # Signal the monitor loop to stop; give it 500 ms, then force-quit.
        self.monitor.running = False
        self.monitor.requestInterruption()
        # QTimer lets the current event loop iteration finish first
        QTimer.singleShot(500, self._force_quit)

    def _force_quit(self):
        self.tray_icon.hide()
        QCoreApplication.instance().quit()


if __name__ == "__main__":
    app = QApplication(sys.argv)
    app.setQuitOnLastWindowClosed(False)   # keep alive when minimised to tray
    window = MainWindow()
    window.show()
    sys.exit(app.exec())
