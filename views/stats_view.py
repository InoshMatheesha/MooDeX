from PySide6.QtWidgets import QWidget, QVBoxLayout, QHBoxLayout, QLabel, QGridLayout, QProgressBar, QFrame, QScrollArea, QGraphicsDropShadowEffect
from PySide6.QtCore import Qt
from PySide6.QtGui import QColor, QFont
from data_manager import DataManager

class StatsView(QWidget):
    def __init__(self, data_manager):
        super().__init__()
        self.data_manager = data_manager
        self.data_manager.data_changed.connect(self.update_stats)

        layout = QVBoxLayout(self)
        layout.setContentsMargins(40, 40, 40, 40)
        layout.setSpacing(30)

        # Header (Logo + Title)
        header_layout = QHBoxLayout()
        header_layout.setAlignment(Qt.AlignLeft | Qt.AlignVCenter)
        
        import os
        from PySide6.QtGui import QPixmap
        
        logo_label = QLabel()
        logo_path = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "Icon Logo", "logo with name.png")
        if os.path.exists(logo_path):
            pixmap = QPixmap(logo_path)
            logo_label.setPixmap(pixmap.scaled(180, 90, Qt.KeepAspectRatio, Qt.SmoothTransformation))
        
        header = QLabel("<h2>Commander Dashboard</h2>")
        header.setStyleSheet("color: white; font-size: 20px; font-weight: 800; letter-spacing: 2px; margin-left: 20px;")
        
        header_layout.addWidget(logo_label)
        header_layout.addWidget(header)
        layout.addLayout(header_layout)

        # Top Grid (Stat Cards)
        self.grid = QHBoxLayout()
        self.grid.setSpacing(25)
        layout.addLayout(self.grid)
        
        self.total_card = self.create_card("TOTAL GAMES", "0", "#6366f1")
        self.playing_card = self.create_card("PLAYING NOW", "0", "#10b981")
        self.completed_card = self.create_card("COMPLETED", "0", "#3b82f6")
        self.playtime_card = self.create_card("PLAYTIME (hrs)", "0", "#a855f7")
        self.rating_card = self.create_card("AVG RATING", "0.0", "#fbbf24")

        # Progress / Status Overview Segment
        middle_layout = QHBoxLayout()
        middle_layout.setSpacing(25)
        
        status_box, status_inner = self.create_container("Library Breakdown")
        box_layout = QVBoxLayout(status_inner)
        box_layout.setContentsMargins(0, 0, 0, 0)
        box_layout.setSpacing(15)

        self.bars = {}
        statuses = ["Playing", "Completed", "Like to Play", "Stopped"]
        colors = ["#10b981", "#3b82f6", "#f59e0b", "#ef4444"]
        
        for st, col in zip(statuses, colors):
            row = QHBoxLayout()
            lbl = QLabel(st.upper())
            lbl.setStyleSheet("color: #a0a0b0; font-weight: bold; width: 100px; font-size: 11px; letter-spacing: 1px;")
            lbl.setFixedWidth(120)
            
            bar = QProgressBar()
            bar.setTextVisible(False)
            bar.setFixedHeight(12)
            bar.setStyleSheet(f"""
                QProgressBar {{
                    border: none;
                    background-color: #1c1d24;
                    border-radius: 6px;
                }}
                QProgressBar::chunk {{
                    background-color: {col};
                    border-radius: 6px;
                }}
            """)
            
            val_lbl = QLabel("0")
            val_lbl.setStyleSheet("color: white; font-weight: bold; font-size: 12px;")
            val_lbl.setFixedWidth(30)
            val_lbl.setAlignment(Qt.AlignRight | Qt.AlignVCenter)
            
            row.addWidget(lbl)
            row.addWidget(bar)
            row.addWidget(val_lbl)
            box_layout.addLayout(row)
            
            self.bars[st] = (bar, val_lbl)

        middle_layout.addWidget(status_box, stretch=2)

        # Top Platforms Widget
        self.platforms_box, self.platforms_inner = self.create_container("Top Platforms")
        self.platforms_layout = QVBoxLayout(self.platforms_inner)
        self.platforms_layout.setContentsMargins(0, 0, 0, 0)
        self.platforms_layout.setSpacing(15)
        
        middle_layout.addWidget(self.platforms_box, stretch=1)

        layout.addLayout(middle_layout)
        layout.addStretch()

        self.update_stats()

    def create_container(self, title_text):
        container = QFrame()
        container.setStyleSheet("""
            QFrame {
                background-color: #23242d;
                border-radius: 20px;
                border: 1px solid #2d2e3a;
            }
        """)
        shadow = QGraphicsDropShadowEffect()
        shadow.setBlurRadius(20)
        shadow.setColor(QColor(0, 0, 0, 80))
        shadow.setOffset(0, 10)
        container.setGraphicsEffect(shadow)
        
        vbox = QVBoxLayout(container)
        vbox.setContentsMargins(25, 25, 25, 25)
        title = QLabel(title_text)
        title.setStyleSheet("color: white; font-size: 16px; font-weight: 800; border: none; letter-spacing: 1px; margin-bottom: 10px;")
        vbox.addWidget(title)
        
        inner = QWidget()
        inner.setStyleSheet("background: transparent; border: none;")
        vbox.addWidget(inner)
        return container, inner

    def create_card(self, title_text, value, accent_color):
        card = QFrame()
        card.setStyleSheet(f"""
            QFrame {{
                background-color: #23242d;
                border-radius: 20px;
                border: 1px solid #2d2e3a;
                border-top: 3px solid {accent_color};
            }}
        """)
        
        shadow = QGraphicsDropShadowEffect(self)
        shadow.setBlurRadius(25)
        shadow.setColor(QColor(0, 0, 0, 60))
        shadow.setOffset(0, 10)
        card.setGraphicsEffect(shadow)

        layout = QVBoxLayout(card)
        layout.setContentsMargins(25, 25, 25, 25)
        layout.setSpacing(5)
        
        title = QLabel(title_text)
        title.setStyleSheet("font-size: 12px; color: #a0a0b0; font-weight: 800; border: none; letter-spacing: 1px;")
        
        val_label = QLabel(value)
        val_label.setStyleSheet(f"font-size: 36px; font-weight: 900; color: white; border: none;")

        layout.addWidget(title)
        layout.addWidget(val_label)
        layout.addStretch()
        
        self.grid.addWidget(card)
        return val_label

    def update_stats(self):
        games = self.data_manager.get_games()
        total = len(games)
        
        playing = sum(1 for g in games if g.get("status") == "Playing")
        completed = sum(1 for g in games if g.get("status") == "Completed")
        playtime = sum(g.get("playtime", 0) for g in games)
        
        ratings = [g.get("rating", 0) for g in games if g.get("rating", 0) > 0]
        avg_rating = sum(ratings) / len(ratings) if ratings else 0.0

        self.total_card.setText(str(total))
        self.playing_card.setText(str(playing))
        self.completed_card.setText(str(completed))
        self.playtime_card.setText(str(playtime))
        self.rating_card.setText(f"{avg_rating:.1f}")

        for st in self.bars.keys():
            count = sum(1 for g in games if g.get("status") == st)
            bar, val_lbl = self.bars[st]
            val_lbl.setText(str(count))
            bar.setMaximum(max(total, 1))
            bar.setValue(count)
            
        # Update Top Platforms
        while self.platforms_layout.count():
            item = self.platforms_layout.takeAt(0)
            widget = item.widget()
            if widget:
                widget.deleteLater()
            elif item.layout():
                while item.layout().count():
                    inner_item = item.layout().takeAt(0)
                    if inner_item.widget(): inner_item.widget().deleteLater()
                
        plat_counts = {}
        for g in games:
            for p in g.get("platforms", []):
                if p: plat_counts[p] = plat_counts.get(p, 0) + 1
        
        sorted_plats = sorted(plat_counts.items(), key=lambda x: x[1], reverse=True)[:5]
        for p, c in sorted_plats:
            row = QHBoxLayout()
            lbl = QLabel(p)
            lbl.setStyleSheet("color: white; font-weight: bold; font-size: 13px;")
            val = QLabel(str(c))
            val.setStyleSheet("color: #a0a0b0; font-weight: bold; font-size: 13px;")
            row.addWidget(lbl)
            row.addWidget(val, alignment=Qt.AlignRight)
            self.platforms_layout.addLayout(row)
            
        if not sorted_plats:
            empty = QLabel("No platforms found")
            empty.setStyleSheet("color: #a0a0b0; font-style: italic;")
            self.platforms_layout.addWidget(empty)
            
        self.platforms_layout.addStretch()
