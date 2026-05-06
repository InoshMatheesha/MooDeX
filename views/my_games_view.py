from PySide6.QtWidgets import QWidget, QVBoxLayout, QHBoxLayout, QLabel, QLineEdit, QScrollArea, QPushButton, QFileDialog
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
        
        add_btn = QPushButton("+ Manually Add Game")
        add_btn.setStyleSheet("""
            QPushButton {
                background-color: #6366f1; 
                color: white; 
                border-radius: 8px; 
                padding: 8px 15px; 
                font-weight: bold;
            }
            QPushButton:hover {
                background-color: #4f46e5;
            }
        """)
        add_btn.clicked.connect(self.add_local_game_dialog)
        
        header_layout.addWidget(title)
        header_layout.addStretch()
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
        scroll_pos = self.scroll_area.verticalScrollBar().value()
        while self.flow_layout.count():
            item = self.flow_layout.takeAt(0)
            if item.widget():
                item.widget().setParent(None)
                item.widget().deleteLater()
                
        games = self.data_manager.data.get("local_games", [])
        term = self.search_input.text().lower()
        
        for game in games:
            if term and term not in game.get("name", "").lower():
                continue
            try:
                card = GameCard(game)
                # Completely clear GameCard's default action layout to build our custom premium row
                card.add_btn.hide()
                card.edit_btn.hide()
                card.del_btn.hide()
                while card.action_layout.count():
                    card.action_layout.takeAt(0)
                
                card.action_layout.setSpacing(6) # tighter spacing between the premium pills
                
                # Play button
                card.play_btn = QPushButton()
                card.play_btn.setIcon(IconManager.get_instance().get_icon("play"))
                card.play_btn.setIconSize(QSize(16, 16))
                card.play_btn.setCursor(Qt.PointingHandCursor)
                card.play_btn.setToolTip("Play Game")
                card.play_btn.setStyleSheet("""
                    QPushButton {
                        background-color: #10b981; 
                        border-radius: 16px; 
                        border: none;
                        padding-left: 9px;
                        padding-bottom: 10px;
                    }
                    QPushButton:hover { 
                        background-color: #059669; 
                    }
                """)
                card.play_btn.setFixedSize(32, 32)
                card.play_btn.clicked.connect(lambda checked=False, p=game.get("exe_path"): self.launch_game(p))
                
                # Relink button
                card.relink_btn = QPushButton()
                card.relink_btn.setIcon(IconManager.get_instance().get_icon("relink"))
                card.relink_btn.setIconSize(QSize(16, 16))
                card.relink_btn.setCursor(Qt.PointingHandCursor)
                card.relink_btn.setToolTip("Relink Metadata")
                card.relink_btn.setStyleSheet("""
                    QPushButton {
                        background-color: #4f46e5; 
                        border-radius: 16px; 
                        border: none;
                        padding-left: 9px;
                        padding-bottom: 10px;
                    }
                    QPushButton:hover { 
                        background-color: #3730a3; 
                    }
                """)
                card.relink_btn.setFixedSize(32, 32)
                card.relink_btn.clicked.connect(lambda checked=False, p=game.get("exe_path"), pt=game.get("playtime", 0), lp=game.get("last_played", "Never"): self.link_game(p, pt, lp))
                
                # Unlink button
                card.unlink_btn = QPushButton()
                card.unlink_btn.setIcon(IconManager.get_instance().get_icon("unlink"))
                card.unlink_btn.setIconSize(QSize(16, 16))
                card.unlink_btn.setCursor(Qt.PointingHandCursor)
                card.unlink_btn.setToolTip("Unlink Game")
                card.unlink_btn.setStyleSheet("""
                    QPushButton {
                        background-color: transparent;
                        border: 1px solid rgba(239, 68, 68, 0.3);
                        border-radius: 16px;
                        padding-left: 7px;
                        padding-bottom: 10px;
                    }
                    QPushButton:hover { 
                        background-color: rgba(239, 68, 68, 0.15); 
                        border: 1px solid #ef4444;
                    }
                """)
                card.unlink_btn.setFixedSize(32, 32)
                card.unlink_btn.clicked.connect(lambda checked=False, p=game.get("exe_path"): self.data_manager.remove_local_game(p))
                
                # Inject new premium row
                card.action_layout.addWidget(card.play_btn)
                card.action_layout.addWidget(card.relink_btn)
                card.action_layout.addWidget(card.unlink_btn)
                
                # Metadata Indicator
                has_meta = "id" in game or "background_image" in game
                indicator = "Linked" if has_meta else "Metadata ⚠️"
                
                # Override status label
                card.status_label.setText(f"{indicator}\n\n{game.get('last_played', 'Never').upper()}\n")
                card.status_label.setStyleSheet("color: #3b82f6; font-weight: 800; font-size: 11px;")
                
                # Override rating with exact playtime (hrs/mins)
                playtime_raw = float(game.get("playtime", 0))
                hours = int(playtime_raw)
                minutes = int((playtime_raw - hours) * 60)
                time_str = f"{hours}h {minutes}m" if hours > 0 else f"{minutes}m"
                
                clock_pixmap = IconManager.get_instance().get_icon("clock").pixmap(QSize(14, 14))
                card.rating_label.setPixmap(clock_pixmap)
                
                if hasattr(card, 'rating_text'):
                    card.rating_text.setText(f" {time_str}")
                    card.rating_text.setStyleSheet("color: #a0a0b0; font-weight: bold; font-size: 13px;")
                    card.rating_widget.show()
                else:
                    # Fallback just in case GameCard hasn't fully upgraded rating_text
                    card.rating_label.setText(f" {time_str}")
                    card.rating_label.setStyleSheet("color: #a0a0b0; font-weight: bold; font-size: 13px;")
                    card.rating_label.show()
                
                self.flow_layout.addWidget(card)
            except Exception as e:
                print(f"Error rendering game card: {e}")
            
        # Force layout update to prevent empty grid glitch
        self.flow_layout.invalidate()
        self.grid_widget.updateGeometry()
        
        QTimer.singleShot(0, lambda: self.scroll_area.verticalScrollBar().setValue(scroll_pos))
            
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
