from PySide6.QtWidgets import QWidget, QVBoxLayout, QHBoxLayout, QLabel, QLineEdit, QTabWidget, QScrollArea, QGraphicsDropShadowEffect
from PySide6.QtCore import Qt, QTimer
from PySide6.QtGui import QColor, QFont
from widgets.game_card import GameCard
from flow_layout import FlowLayout
from rawg_api import RawgApi

class DiscoverView(QWidget):
    def __init__(self, data_manager):
        super().__init__()
        self.data_manager = data_manager
        self.api = RawgApi()
        
        layout = QVBoxLayout(self)

        # Header
        header_layout = QHBoxLayout()
        title = QLabel("<h2>Discover</h2>")
        title.setStyleSheet("color: white; margin: 10px 0;")
        self.search_input = QLineEdit()
        self.search_input.setPlaceholderText("Search Game...")
        self.search_input.setFixedWidth(300)
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
        
        # Debounce timer for search
        self.search_timer = QTimer()
        self.search_timer.setSingleShot(True)
        self.search_timer.timeout.connect(self.perform_search)
        self.search_input.textChanged.connect(lambda: self.search_timer.start(500))

        header_layout.addWidget(title)
        header_layout.addStretch()
        header_layout.addWidget(self.search_input)
        layout.addLayout(header_layout)

        # Tabs
        self.tabs = QTabWidget()
        self.tabs.setStyleSheet("""
            QTabWidget::pane { 
                border: 0; 
                background: transparent;
                margin-top: 10px;
            }
            QTabWidget::tab-bar {
                alignment: center;
            }
            QTabBar::tab { 
                background: transparent; 
                color: #8b8f9e; 
                font-size: 16px; 
                font-weight: 800; 
                padding: 12px 30px; 
                margin: 0 10px;
                border-bottom: 3px solid transparent; 
                letter-spacing: 1px;
            }
            QTabBar::tab:selected { 
                color: #ffffff; 
                border-bottom: 3px solid #6366f1; 
            }
            QTabBar::tab:hover { 
                color: #ffffff; 
                background: #1c1d24;
                border-radius: 8px;
            }
        """)
        
        self.popular_tab = self.create_grid_tab()
        self.upcoming_tab = self.create_grid_tab()
        self.trending_tab = self.create_grid_tab()
        
        self.tabs.addTab(self.popular_tab["widget"], "POPULAR")
        self.tabs.addTab(self.upcoming_tab["widget"], "UPCOMING")
        self.tabs.addTab(self.trending_tab["widget"], "TRENDING")
        
        layout.addWidget(self.tabs)
        
        # Fetch initial data
        self.load_tab_data("popular", self.popular_tab["layout"])
        self.load_tab_data("upcoming", self.upcoming_tab["layout"])
        self.load_tab_data("trending", self.trending_tab["layout"])

    def create_grid_tab(self):
        scroll_area = QScrollArea()
        scroll_area.setWidgetResizable(True)
        scroll_area.setStyleSheet("QScrollArea { border: none; background-color: transparent; }")
        
        grid_widget = QWidget()
        grid_widget.setStyleSheet("background-color: transparent;")
        flow_layout = FlowLayout(grid_widget, margin=10, spacing=15)
        scroll_area.setWidget(grid_widget)
        
        return {"widget": scroll_area, "layout": flow_layout}

    def clear_layout(self, layout):
        while layout.count():
            item = layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()

    def load_tab_data(self, query_type, target_layout, search_term=""):
        self.clear_layout(target_layout)
        loading_label = QLabel("Loading...")
        loading_label.setStyleSheet("color: #6366f1; font-size: 18px; font-weight: bold;")
        loading_label.setAlignment(Qt.AlignCenter)
        target_layout.addWidget(loading_label)
        
        is_search = (query_type == "search")
        self.api.fetch_async(query_type, search_term, 
                             lambda data, is_s=is_search: self.populate_grid(data, target_layout, is_s))

    def populate_grid(self, games_data, target_layout, is_search=False):
        self.clear_layout(target_layout)
        
        existing_games = {g.get("name", "").lower() for g in self.data_manager.get_games()}
        
        for game in games_data:
            game_name = game.get("name", "").lower()
            
            if not is_search and game_name in existing_games:
                continue
                
            game["status"] = "Like to Play"
            card = GameCard(game)
            card.add_clicked.connect(lambda gd, c=card: self.on_add_game(gd, c))
            card.edit_btn.hide()
            card.del_btn.hide()
            card.add_btn.show()
            
            if is_search and game_name in existing_games:
                card.add_btn.setText("Added")
                card.add_btn.setEnabled(False)
                card.add_btn.setStyleSheet("""
                    QPushButton {
                        background-color: #2d2e3a;
                        color: #6b7280;
                        border-radius: 8px;
                        padding: 0 12px;
                        font-weight: bold;
                    }
                """)
                
            target_layout.addWidget(card)

    def perform_search(self):
        term = self.search_input.text().strip()
        idx = self.tabs.currentIndex()
        target_layout = None
        
        if idx == 0:
            target_layout = self.popular_tab["layout"]
            query_type = "search" if term else "popular"
        elif idx == 1:
            target_layout = self.upcoming_tab["layout"]
            query_type = "search" if term else "upcoming"
        elif idx == 2:
            target_layout = self.trending_tab["layout"]
            query_type = "search" if term else "trending"
            
        if target_layout:
            self.load_tab_data(query_type, target_layout, term)

    def on_add_game(self, game_data, card=None):
        success = self.data_manager.add_game(game_data)
        if success:
            from widgets.toast import Toast
            Toast(self.window(), f"Added {game_data.get('name')} to Library!", "success").show_toast()
            if card:
                term = self.search_input.text().strip()
                if term:
                    card.add_btn.setText("Added")
                    card.add_btn.setEnabled(False)
                    card.add_btn.setStyleSheet("""
                        QPushButton {
                            background-color: #2d2e3a;
                            color: #6b7280;
                            border-radius: 8px;
                            padding: 0 12px;
                            font-weight: bold;
                        }
                    """)
                else:
                    layout = card.parentWidget().layout() if card.parentWidget() else None
                    card.hide()
                    card.setParent(None)
                    card.deleteLater()
                    if layout:
                        layout.invalidate()
        else:
            from widgets.toast import Toast
            Toast(self.window(), f"{game_data.get('name')} is already in Library!", "warn").show_toast()
