from PySide6.QtWidgets import QWidget, QVBoxLayout, QHBoxLayout, QLabel, QLineEdit, QComboBox, QScrollArea
from PySide6.QtCore import Qt, QTimer
from widgets.game_card import GameCard
from flow_layout import FlowLayout

class LibraryView(QWidget):
    def __init__(self, data_manager):
        super().__init__()
        self.data_manager = data_manager
        self.data_manager.data_changed.connect(self.refresh_grid)

        layout = QVBoxLayout(self)

        # Header
        header_layout = QHBoxLayout()
        title = QLabel("<h2>My Library</h2>")
        title.setStyleSheet("color: white; margin: 10px 0;")
        self.search_input = QLineEdit()
        self.search_input.setPlaceholderText("Search games...")
        self.search_input.setFixedWidth(250)
        self.search_input.textChanged.connect(self.filter_games)
        
        self.filter_combo = QComboBox()
        self.filter_combo.addItems(["All", "Playing", "Completed", "Like to Play", "Stopped"])
        self.filter_combo.currentTextChanged.connect(self.filter_games)

        header_layout.addWidget(title)
        header_layout.addStretch()
        header_layout.addWidget(self.search_input)
        header_layout.addWidget(self.filter_combo)
        layout.addLayout(header_layout)

        # Dynamic Stats Bar for game counts
        self.stats_bar = QWidget()
        self.stats_bar.setStyleSheet("background-color: transparent;")
        self.stats_layout = QHBoxLayout(self.stats_bar)
        self.stats_layout.setContentsMargins(0, 5, 0, 15)
        self.stats_layout.setSpacing(10)
        
        self.stats_labels = {}
        statuses = ["All", "Playing", "Completed", "Like to Play", "Stopped"]
        
        for s in statuses:
            lbl = QLabel(f"{s}: 0")
            lbl.setStyleSheet("""
                background-color: #1c1d24; 
                color: #a0a0b0; 
                padding: 6px 14px; 
                border-radius: 12px; 
                font-size: 13px; 
                font-weight: bold;
                border: 1px solid #2d2e3a;
            """)
            self.stats_layout.addWidget(lbl)
            self.stats_labels[s] = lbl
            
        self.stats_layout.addStretch()
        layout.addWidget(self.stats_bar)

        # Scroll Area for FlowLayout
        self.scroll_area = QScrollArea()
        self.scroll_area.setWidgetResizable(True)
        self.scroll_area.setStyleSheet("QScrollArea { border: none; background-color: transparent; }")
        
        self.grid_widget = QWidget()
        self.grid_widget.setStyleSheet("background-color: transparent;")
        self.flow_layout = FlowLayout(self.grid_widget, margin=10, spacing=15)
        self.scroll_area.setWidget(self.grid_widget)
        
        layout.addWidget(self.scroll_area)
        
        # Initial population
        self.refresh_grid()

    def refresh_grid(self):
        scroll_pos = self.scroll_area.verticalScrollBar().value()

        # Clear existing
        while self.flow_layout.count():
            item = self.flow_layout.takeAt(0)
            if item.widget():
                item.widget().setParent(None)
                item.widget().deleteLater()

        games = self.data_manager.get_games()
        search_term = self.search_input.text().lower()
        filter_status = self.filter_combo.currentText()

        # Build stats array map
        counts = {s: 0 for s in self.stats_labels.keys()}
        counts["All"] = len(games)

        for game in games:
            # Count the stat
            st = game.get("status", "Like to Play")
            if st in counts:
                counts[st] += 1

            # Filter normally from view bounds
            if search_term and search_term not in game.get("name", "").lower():
                continue
            if filter_status != "All" and st != filter_status:
                continue

            card = GameCard(game)
            card.delete_clicked.connect(self.on_delete_game)
            card.edit_clicked.connect(self.on_edit_game)
            self.flow_layout.addWidget(card)

        # Update the visual labels
        for s, lbl in self.stats_labels.items():
            if counts[s] > 0:
                lbl.setText(f"{s}: <span style='color: #6366f1;'>{counts[s]}</span>")
                lbl.show()
            else:
                # Optional: Hide pills with 0, or just show 0
                lbl.setText(f"{s}: 0")
                lbl.show()

        # Force layout update
        self.flow_layout.invalidate()
        self.grid_widget.updateGeometry()
        
        # Restore scroll bar position after event loop processes the new layout
        QTimer.singleShot(0, lambda: self.scroll_area.verticalScrollBar().setValue(scroll_pos))

    def filter_games(self, text=""):
        self.refresh_grid()

    def on_delete_game(self, game_data):
        self.data_manager.remove_game(game_data.get("name"))

    def on_edit_game(self, game_data):
        from widgets.edit_dialog import EditDialog
        dialog = EditDialog(game_data, self)
        if dialog.exec():
            updates = dialog.get_updates()
            self.data_manager.update_game(game_data.get("name"), updates)
