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

        self.filter_combo.addItems([
            "All",
            "Playing",
            "Completed",
            "Like to Play",
            "Stopped",

            "Name (A-Z)",
            "Name (Z-A)",

            "Rating (Highest)",
            "Rating (Lowest)",

            "Release Year (Newest)",
            "Release Year (Oldest)"
        ])

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
        self.card_cache = {}
        self.games_to_render = []
        self.creation_timer = QTimer()
        self.creation_timer.timeout.connect(self._process_render_chunk)
        
        self.refresh_grid()

    def refresh_grid(self):
        games = self.data_manager.get_games()
        selected = self.filter_combo.currentText()

        if selected == "Name (A-Z)":
            games.sort(key=lambda g: g.get("name", "").lower())
        elif selected == "Name (Z-A)":
            games.sort(key=lambda g: g.get("name", "").lower(), reverse=True)
        elif selected == "Rating (Highest)":
            games.sort(key=lambda g: g.get("rating", 0), reverse=True)
        elif selected == "Rating (Lowest)":
            games.sort(key=lambda g: g.get("rating", 0))
        elif selected == "Release Year (Newest)":
            games.sort(key=lambda g: g.get("released", ""), reverse=True)
        elif selected == "Release Year (Oldest)":
            games.sort(key=lambda g: g.get("released", ""))

        search_term = self.search_input.text().lower()
        filter_status = self.filter_combo.currentText()

        # Build stats array map
        counts = {s: 0 for s in self.stats_labels.keys()}
        counts["All"] = len(games)

        visible_games = []
        
        local_games = self.data_manager.data.get("local_games", [])
        
        for game in games:
            # Sync playtime from local_games if available
            game_name = game.get("name", "").lower()
            for lg in local_games:
                if lg.get("name", "").lower() == game_name:
                    game["playtime"] = lg.get("playtime", 0)
                    break
                    
            # Count the stat
            st = game.get("status", "Like to Play")
            if st in counts:
                counts[st] += 1

            # Filter normally from view bounds
            if search_term and search_term not in game.get("name", "").lower():
                continue
            
            normal_filters = ["All", "Playing", "Completed", "Like to Play", "Stopped"]

            if filter_status in normal_filters:
                if filter_status != "All" and st != filter_status:
                    continue
                    
            visible_games.append(game)

        # Update the visual labels
        for s, lbl in self.stats_labels.items():
            if counts[s] > 0:
                lbl.setText(f"{s}: <span style='color: #6366f1;'>{counts[s]}</span>")
                lbl.show()
            else:
                lbl.setText(f"{s}: 0")
                lbl.show()

        self.visible_games = visible_games

        # Determine missing cards that need to be generated
        missing_games = [g for g in games if g.get("name") not in self.card_cache]

        if missing_games:
            self.games_to_render.extend(missing_games)
            if not self.creation_timer.isActive():
                self.creation_timer.start(5) # 5ms interval
        else:
            self._reorder_and_filter()

    def _process_render_chunk(self):
        if not self.games_to_render:
            self.creation_timer.stop()
            self._reorder_and_filter()
            return
            
        # process up to 10 cards per tick
        chunk = self.games_to_render[:10]
        self.games_to_render = self.games_to_render[10:]
        
        for game in chunk:
            name = game.get("name")
            if name not in self.card_cache:
                card = GameCard(game)
                card.delete_clicked.connect(self.on_delete_game)
                card.edit_clicked.connect(self.on_edit_game)
                self.card_cache[name] = card
                self.flow_layout.addWidget(card)

    def _reorder_and_filter(self):
        scroll_pos = self.scroll_area.verticalScrollBar().value()
        self.setUpdatesEnabled(False)
        
        # Build new item list to enforce sorting order
        new_item_list = []
        for game in self.visible_games:
            name = game.get("name")
            card = self.card_cache.get(name)
            if card:
                card.update_ui(game)
                for item in self.flow_layout.itemList:
                    if item.widget() == card:
                        new_item_list.append(item)
                        break
                        
        # add remaining so we don't lose items
        for item in self.flow_layout.itemList:
            if item not in new_item_list:
                new_item_list.append(item)
                
        self.flow_layout.itemList = new_item_list
        
        # Show/Hide
        visible_names = {g.get("name") for g in self.visible_games}
        for name, card in self.card_cache.items():
            if name in visible_names:
                card.show()
            else:
                card.hide()
                
        self.flow_layout.invalidate()
        self.scroll_area.verticalScrollBar().setValue(scroll_pos)
        self.setUpdatesEnabled(True)

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
