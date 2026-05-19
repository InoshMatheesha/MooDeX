from PySide6.QtWidgets import QWidget, QVBoxLayout, QHBoxLayout, QLabel, QLineEdit, QScrollArea, QPushButton, QFileDialog, QComboBox
from PySide6.QtCore import Qt, QTimer
from widgets.game_card import GameCard
from flow_layout import FlowLayout
from icon_manager import IconManager
from PySide6.QtCore import QSize
import os

class MyGamesView(QWidget):
    def __init__(self, data_manager):
        super().__init__()
        self.data_manager = data_manager
        self.data_manager.data_changed.connect(self.refresh_grid)
        self.card_cache = {}
        
        layout = QVBoxLayout(self)
        
        # Header
        header_layout = QHBoxLayout()
        title = QLabel("<h2>My Games</h2>")
        title.setStyleSheet("color: white; margin: 10px 0;")
        
        self.search_input = QLineEdit()
        self.search_input.setPlaceholderText("Search local games...")
        self.search_input.setFixedWidth(250)
        self.search_input.setStyleSheet("""
            QLineEdit {
                background-color: #1c1d24;
                color: white;
                border: 1px solid #2d2e3a;
                border-radius: 12px;
                padding: 8px 15px;
                font-size: 14px;
            }
            QLineEdit:focus {
                border: 1px solid #6366f1;
            }
        """)
        self.search_input.textChanged.connect(self.filter_games)
        
        add_btn = QPushButton("╋")
        add_btn.setStyleSheet("""
            QPushButton {
                background-color: #6366f1; 
                color: white; 
                border-radius: 17px; 
                padding: 8px 12px; 
                font-weight: bold;
            }
            QPushButton:hover {
                background-color: #4f46e5;
            }
        """)
        add_btn.clicked.connect(self.add_local_game_dialog)
        
        self.sort_combo = QComboBox()
        self.sort_combo.addItems([
            "Recently Played",
            "Playtime",
            "Name (A-Z)"
        ])
        self.sort_combo.setStyleSheet("""
            QComboBox {
                background-color: #1c1d24;
                color: white;
                border: 1px solid #2d2e3a;
                border-radius: 12px;
                padding: 8px 15px;
                font-size: 14px;
            }
        """)
        self.sort_combo.currentTextChanged.connect(self.filter_games)

        header_layout.addWidget(title)
        header_layout.addStretch()
        header_layout.addWidget(self.sort_combo)
        header_layout.addWidget(self.search_input)
        header_layout.addWidget(add_btn)
        
        layout.addLayout(header_layout)
        
        # Grid
        self.scroll_area = QScrollArea()
        self.scroll_area.setWidgetResizable(True)
        self.scroll_area.setStyleSheet("QScrollArea { border: none; background-color: transparent; }")
        
        self.grid_widget = QWidget()
        self.grid_widget.setStyleSheet("background-color: transparent;")
        self.flow_layout = FlowLayout(self.grid_widget, margin=10, spacing=15)
        self.scroll_area.setWidget(self.grid_widget)
        
        layout.addWidget(self.scroll_area)
        self.refresh_grid()
        
    def add_local_game_dialog(self):
        file_path, _ = QFileDialog.getOpenFileName(self, "Select Game Executable", "", "Executables (*.exe)")
        if file_path:
            self.link_game(file_path)

    def link_game(self, file_path, existing_playtime=0, existing_last_played="Never"):
        from widgets.smart_link_dialog import SmartLinkDialog
        dialog = SmartLinkDialog(file_path, self.window())
        if dialog.exec():
            if dialog.selected_game_data:
                game_data = dialog.selected_game_data.copy()
                game_data["exe_path"] = file_path
                game_data["playtime"] = existing_playtime
                game_data["last_played"] = existing_last_played
                game_data["status"] = "Installed"
                game_data["is_local"] = True
                
                # if relinking, we must remove old then add new. DataManager's update is by path.
                # add_local_game handles duplicates by rejecting, so we remove first if it exists.
                self.data_manager.remove_local_game(file_path)
                self.data_manager.add_local_game(game_data)
                
                from widgets.toast import Toast
                Toast(self.window(), f"Successfully linked {game_data.get('name')}!", "success").show_toast()
            
    def refresh_grid(self):
        self.setUpdatesEnabled(False)

        scroll_pos = self.scroll_area.verticalScrollBar().value()

        games = self.data_manager.data.get("local_games", [])[:]
        
        sort_mode = self.sort_combo.currentText()
        if sort_mode == "Recently Played":
            def sort_date(g):
                lp = g.get("last_played", "Never")
                return lp if lp != "Never" else ""
            games.sort(key=sort_date, reverse=True)
        elif sort_mode == "Playtime":
            games.sort(key=lambda g: float(g.get("playtime", 0)), reverse=True)
        elif sort_mode == "Name (A-Z)":
            games.sort(key=lambda g: g.get("name", "").lower())

        term = self.search_input.text().lower()

        # ----------------------------------------
        # 1) Prepare new visible paths
        # ----------------------------------------
        visible_paths = set()

        for game in games:
            if term and term not in game.get("name", "").lower():
                continue

            exe_path = game.get("exe_path")
            visible_paths.add(exe_path)

        # Instead of removing widgets, we will keep them and reorder the layout's internal itemList.

        # ----------------------------------------
        # 2) Show/Create cards
        # ----------------------------------------
        for game in games:
            exe_path = game.get("exe_path")
            
            # create missing cards
            if exe_path not in self.card_cache:
                card = GameCard(game)
                self.card_cache[exe_path] = card
                self.flow_layout.addWidget(card)
                
                # customize header for local games
                card.add_btn.hide()
                card.edit_btn.hide()
                card.del_btn.hide()
                card.rating_widget.hide()

                while card.action_layout.count():
                    card.action_layout.takeAt(0)

                card.action_layout.setSpacing(6)

                # PLAY BUTTON
                card.play_btn = QPushButton()
                card.play_btn.setIcon(IconManager.get_instance().get_icon("play"))
                card.play_btn.setIconSize(QSize(16, 16))
                card.play_btn.setFixedSize(32, 32)

                card.play_btn.setStyleSheet("""
                    QPushButton {
                        background-color: #2D2E3A;
                        border-radius: 16px;
                        border: none;
                        padding-left: 8px;
                        padding-bottom: 10px;
                    }

                    QPushButton:hover {
                        background-color: #34d399;
                        border: 1px solid #6ee7b7;
                    }

                    QPushButton:pressed {
                        background-color: #059669;
                        padding-top: 2px;
                    }
                """)

                card.play_btn.clicked.connect(
                    lambda checked=False, p=exe_path: self.launch_game(p)
                )

                # RELINK BUTTON
                card.relink_btn = QPushButton()
                card.relink_btn.setIcon(IconManager.get_instance().get_icon("relink"))
                card.relink_btn.setIconSize(QSize(16, 16))
                card.relink_btn.setFixedSize(32, 32)

                card.relink_btn.setStyleSheet("""
                    QPushButton {
                        background-color: #2D2E3A;
                        border-radius: 16px;
                        border: none;
                        padding-left: 8px;
                        padding-bottom: 10px;
                    }

                    QPushButton:hover {
                        background-color: #05335e;
                        border: 1px solid #143968;
                    }

                    QPushButton:pressed {
                        background-color: #02025E;
                        padding-top: 2px;
                    }
                """)
                card.relink_btn.clicked.connect(
                    lambda checked=False,
                    p=exe_path,
                    pt=game.get("playtime", 0),
                    lp=game.get("last_played", "Never"):
                        self.link_game(p, pt, lp)
                )

                # UNLINK BUTTON
                card.unlink_btn = QPushButton()
                card.unlink_btn.setIcon(IconManager.get_instance().get_icon("unlink"))
                card.unlink_btn.setIconSize(QSize(16, 16))
                card.unlink_btn.setFixedSize(32, 32)

                card.unlink_btn.setStyleSheet("""
                    QPushButton {
                        background-color: #2D2E3A;
                        border-radius: 16px;
                        border: none;
                        padding-left: 8px;
                        padding-bottom: 10px;
                    }

                    QPushButton:hover {
                        background-color: #f87171;
                        border: 1px solid #fca5a5;
                    }

                    QPushButton:pressed {
                        background-color: #ef4444;
                        padding-top: 2px;
                    }
                """)

                card.unlink_btn.clicked.connect(
                    lambda checked=False, p=exe_path:
                        self.data_manager.remove_local_game(p)
                )

                card.action_layout.addWidget(card.play_btn)
                card.action_layout.addWidget(card.relink_btn)
                card.action_layout.addWidget(card.unlink_btn)

            # =========================
            # UPDATE UI
            # =========================
            card = self.card_cache[exe_path]
            card.update_ui(game)

        # ----------------------------------------
        # REORDER THE LAYOUT
        # ----------------------------------------
        new_item_list = []
        # First append items in sorted 'games' order
        for game in games:
            exe_path = game.get("exe_path")
            card = self.card_cache.get(exe_path)
            if card:
                for item in self.flow_layout.itemList:
                    if item.widget() == card:
                        new_item_list.append(item)
                        break
        
        # Then append any remaining items (like removed games that are not in 'games' anymore)
        for item in self.flow_layout.itemList:
            if item not in new_item_list:
                new_item_list.append(item)
                
        self.flow_layout.itemList = new_item_list

        # ----------------------------------------
        # 3) Hide removed/filtered cards
        # ----------------------------------------
        for path, card in self.card_cache.items():
            if path not in visible_paths:
                card.hide()
            else:
                card.show()

        self.flow_layout.invalidate()
        self.scroll_area.verticalScrollBar().setValue(scroll_pos)
        self.setUpdatesEnabled(True)
            
    def filter_games(self):
        self.refresh_grid()
        
    def launch_game(self, path):
        import subprocess
        try:
            cwd = os.path.dirname(path)
            subprocess.Popen([path], cwd=cwd)
            from widgets.toast import Toast
            Toast(self.window(), "Game Launched! Tracking playtime...", "success").show_toast()
        except Exception as e:
            from widgets.toast import Toast
            Toast(self.window(), f"Failed to launch: {e}", "warn").show_toast()
