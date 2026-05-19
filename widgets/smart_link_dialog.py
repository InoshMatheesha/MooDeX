from PySide6.QtWidgets import (QDialog, QVBoxLayout, QHBoxLayout, QLabel, 
                               QPushButton, QLineEdit, QListWidget, QListWidgetItem, QWidget)
from PySide6.QtCore import Qt
from PySide6.QtGui import QPixmap
from rawg_api import RawgApi
from PySide6.QtCore import QTimer
import os
import re

KNOWN_ALIASES = {
    "gta5": "Grand Theft Auto V",
    "rdr2": "Red Dead Redemption 2",
    "tlou": "The Last of Us Part I",
    "re9": "Resident Evil 9",
    "re4": "Resident Evil 4",
    "acshadows": "Assassin's Creed Shadows",
    "cyberpunk2077": "Cyberpunk 2077",
    "witcher3": "The Witcher 3: Wild Hunt"
}

class SmartLinkDialog(QDialog):
    def __init__(self, exe_path, parent=None):
        super().__init__(parent)
        self.exe_path = exe_path
        self.api = RawgApi()
        self.selected_game_data = None
        self.generic_name = os.path.basename(exe_path).replace('.exe', '')
        
        self.setWindowTitle("Smart Game Match")
        self.setFixedSize(500, 600)
        self.setStyleSheet("""
            QDialog { background-color: #0f1015; color: white; }
            QLabel { color: white; }
            QPushButton {
                background-color: #2d2e3a; color: white; border-radius: 8px;
                padding: 10px; font-weight: bold;
            }
            QPushButton:hover { background-color: #6366f1; }
            QLineEdit {
                background-color: #1c1d24; border: 1px solid #2d2e3a;
                border-radius: 8px; color: white; padding: 8px;
            }
            QListWidget {
                background-color: #1c1d24; border: 1px solid #2d2e3a;
                border-radius: 8px; padding: 5px; color: #a0a0b0;
                font-size: 14px;
            }
            QListWidget::item:selected {
                background-color: #6366f1; color: white; border-radius: 4px;
            }
        """)
        
        self.layout = QVBoxLayout(self)
        
        # UI
        title_lbl = QLabel(f"<h2>Link Game: {self.generic_name}</h2>")
        title_lbl.setAlignment(Qt.AlignCenter)
        self.layout.addWidget(title_lbl)
        
        # Build query
        base_name = self.generic_name.lower()
        self.query = KNOWN_ALIASES.get(base_name, re.sub(r'[^a-zA-Z0-9]', ' ', base_name))
        
        self.search_bar = QLineEdit(self.query)
        self.search_btn = QPushButton("Search")
        h_layout = QHBoxLayout()
        h_layout.addWidget(self.search_bar)
        h_layout.addWidget(self.search_btn)
        self.layout.addLayout(h_layout)
        
        self.results_list = QListWidget()
        self.layout.addWidget(self.results_list)
        
        btn_layout = QHBoxLayout()
        
        self.manual_btn = QPushButton("Skip (Offline Mode)")
        self.manual_btn.setStyleSheet("background-color: #3b82f6;")
        self.manual_btn.clicked.connect(self.use_manual)
        
        self.confirm_btn = QPushButton("Link Selected Data")
        self.confirm_btn.setStyleSheet("background-color: #10b981;")
        self.confirm_btn.setEnabled(False)
        self.confirm_btn.clicked.connect(self.accept)
        
        btn_layout.addWidget(self.manual_btn)
        btn_layout.addStretch()
        btn_layout.addWidget(self.confirm_btn)
        self.layout.addLayout(btn_layout)
        
        self.search_btn.clicked.connect(self.perform_search)

        # REALTIME SEARCH
        self.search_timer = QTimer()
        self.search_timer.setSingleShot(True)
        self.search_timer.timeout.connect(self.perform_search)

        self.search_bar.textChanged.connect(self.on_search_changed)
        self.results_list.itemSelectionChanged.connect(self.on_select)
        
        # Auto trigger
        self.perform_search()

    def perform_search(self):

        term = self.search_bar.text().strip()

        if not term:
            return

        self.results_list.clear()

        self.results_list.addItem("Searching global database...")

        self.api.fetch_async(
            "search",
            term,
            self.populate_results
        )

    def on_search_changed(self):

        text = self.search_bar.text().strip()

        if len(text) < 2:
            return

        # wait 400ms after typing stops
        self.search_timer.start(400)
        
    def populate_results(self, games):
        self.results_list.clear()
        self.current_results = games
        if not games:
            self.results_list.addItem("No results found on RAWG. Try refining search.")
            return
            
        for idx, g in enumerate(games):
            name = g.get("name", "Unknown")
            year = (g.get("released") or "")[:4]
            item = QListWidgetItem(f"{name} ({year})")
            item.setData(Qt.UserRole, idx)
            self.results_list.addItem(item)
            
    def on_select(self):
        sel = self.results_list.selectedItems()
        if sel and sel[0].data(Qt.UserRole) is not None:
            idx = sel[0].data(Qt.UserRole)
            self.selected_game_data = self.current_results[idx]
            self.confirm_btn.setEnabled(True)
        else:
            self.confirm_btn.setEnabled(False)
            
    def use_manual(self):
        self.selected_game_data = {
            "name": self.generic_name.replace("_", " ").title()
        }
        self.accept()
