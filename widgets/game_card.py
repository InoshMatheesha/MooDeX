from PySide6.QtWidgets import QWidget, QVBoxLayout, QLabel, QPushButton, QHBoxLayout, QFrame
from PySide6.QtCore import Qt, Signal, QPropertyAnimation, QEasingCurve, QParallelAnimationGroup, QPoint, QRect, QSize
from PySide6.QtGui import QPixmap, QColor, QPainter, QPainterPath, QIcon
from image_loader import ImageLoader
from icon_manager import IconManager

class GameCard(QWidget):
    add_clicked = Signal(dict)
    edit_clicked = Signal(dict)
    delete_clicked = Signal(dict)

    def __init__(self, game_data):
        super().__init__()
        self.game_data = game_data
        
        self.setFixedSize(250, 315)
        self.setCursor(Qt.PointingHandCursor)
        self.setAttribute(Qt.WA_Hover)

        # Main modern card frame
        self.card_frame = QFrame(self)
        self.card_frame.setFixedSize(240, 305)
        self.card_frame.move(5, 5)
        
        self.base_style = """
            QFrame#card {
                background-color: #1c1d24;
                border-radius: 20px;
                border: 1px solid #2d2e3a;
            }
            QLabel {
                color: #ffffff;
                border: none;
            }
        """
        self.hover_style = """
            QFrame#card {
                background-color: #23242d;
                border-radius: 20px;
                border: 1px solid #6366f1;
            }
            QLabel {
                color: #ffffff;
                border: none;
            }
        """
        self.card_frame.setObjectName("card")
        self.card_frame.setStyleSheet(self.base_style)

        layout = QVBoxLayout(self.card_frame)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)

        self.image_label = QLabel()
        self.image_label.setFixedSize(240, 135)
        self.image_label.setAlignment(Qt.AlignCenter)
        self.image_label.setStyleSheet("background-color: #0f1015; border-bottom-left-radius: 0px; border-bottom-right-radius: 0px; border-top-left-radius: 20px; border-top-right-radius: 20px;")
        layout.addWidget(self.image_label)

        image_url = game_data.get("image_url") or game_data.get("background_image")
        if image_url:
            ImageLoader.get_instance().load_image(image_url, self.set_image)
        else:
            self.image_label.setText("No Image")

        content = QWidget()
        content.setStyleSheet("background: transparent; border: none;")
        content_layout = QVBoxLayout(content)
        content_layout.setContentsMargins(15, 12, 15, 12)
        content_layout.setSpacing(6)

        title = game_data.get("name", "Unknown Game")
        if len(title) > 30:
            title = title[:27] + "..."
        
        self.title_label = QLabel(f"<b>{title}</b>")
        self.title_label.setWordWrap(True)
        self.title_label.setStyleSheet("font-size: 15px;")
        content_layout.addWidget(self.title_label)
        
        released_raw = game_data.get("released") or ""
        released = released_raw[:4]
        genres = ", ".join(game_data.get("genres", [])[:2])
        self.details_label = QLabel(f"{released} | {genres}")
        self.details_label.setStyleSheet("color: #a0a0b0; font-size: 12px; font-weight: 500;")
        content_layout.addWidget(self.details_label)

        rating = game_data.get("rating", 0)
        status = game_data.get("status", "Unknown")
        
        status_colors = {
            "Playing": "#10b981",
            "Completed": "#3b82f6",
            "Like to Play": "#f59e0b",
            "Stopped": "#ef4444"
        }
        color = status_colors.get(status, "#6b7280")
        
        bottom_row = QHBoxLayout()
        bottom_row.setContentsMargins(0, 8, 0, 0)
        
        self.rating_label = QLabel(f" {rating}")
        
        # Inject SVG Star instead of Unicode
        star_pixmap = IconManager.get_instance().get_icon("star").pixmap(QSize(14, 14))
        self.rating_label.setPixmap(star_pixmap)
        self.rating_text = QLabel(f" {rating}")
        self.rating_text.setStyleSheet("color: #fbbf24; font-weight: bold; font-size: 13px;")

        playtime = float(game_data.get("playtime", 0))

        hours = int(playtime)

        minutes = int((playtime - hours) * 60)

        if hours > 0:
            playtime_text = f"{hours}h {minutes}m"
        else:
            playtime_text = f"{minutes}m"

        self.playtime_text = QLabel(playtime_text)

        if playtime <= 0:
            self.playtime_text.hide()

        self.playtime_text.setStyleSheet("""
            color: #cfcfe6;
            font-size: 14px;
            font-weight: 600;
        """)
        
        rating_layout = QHBoxLayout()
        rating_layout.setContentsMargins(0, 0, 0, 0)
        rating_layout.setSpacing(2)
        rating_layout.addWidget(self.rating_label)
        rating_layout.addWidget(self.rating_text)
        rating_layout.addStretch()
        
        self.rating_widget = QWidget()
        self.rating_widget.setLayout(rating_layout)
        
        if status == "Like to Play":
            self.rating_widget.hide()
            
        self.status_label = QLabel(status.upper())
        self.status_label.setStyleSheet(f"color: {color}; font-weight: 800; font-size: 11px; letter-spacing: 1px;")

        info_vbox = QVBoxLayout()
        info_vbox.setSpacing(2)

        info_vbox.addWidget(self.status_label)
        info_vbox.addWidget(self.rating_widget)
        info_vbox.addWidget(self.playtime_text)

        self.action_layout = QHBoxLayout()
        self.action_layout.setSpacing(8)
        
        # Make the buttons actually visible
        self.add_btn = QPushButton()
        self.add_btn.setIcon(IconManager.get_instance().get_icon("add"))
        self.add_btn.setIconSize(QSize(16, 16))
        self.add_btn.setCursor(Qt.PointingHandCursor)
        self.add_btn.setToolTip("Add Game")
        self.add_btn.hide()
        
        self.edit_btn = QPushButton()
        self.edit_btn.setIcon(IconManager.get_instance().get_icon("edit"))
        self.edit_btn.setIconSize(QSize(16, 16))
        self.edit_btn.setCursor(Qt.PointingHandCursor)
        self.edit_btn.setToolTip("Edit Game")
        
        self.del_btn = QPushButton()
        self.del_btn.setIcon(IconManager.get_instance().get_icon("unlink"))
        self.del_btn.setIconSize(QSize(16, 16))
        self.del_btn.setCursor(Qt.PointingHandCursor)
        self.del_btn.setToolTip("Remove Game")
        
        for btn in [self.add_btn, self.edit_btn, self.del_btn]:
            btn.setFixedSize(32, 32)
            self.action_layout.addWidget(btn)

        self.add_btn.setStyleSheet("""
            QPushButton {
                background-color: #10b981;
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

        # Solid clean visibility by default. No more ghosting! 
        self.edit_btn.setStyleSheet("""
            QPushButton {
                background-color: #2d2e3a;
                border-radius: 16px;
                border: none;
                padding-left: 9px;
                padding-bottom: 10px;
            }

            QPushButton:hover {
                background-color: #4f46e5;
                border: 1px solid #818cf8;
            }

            QPushButton:pressed {
                background-color: #3730a3;
                padding-top: 2px;
            }
        """)
        
        self.del_btn.setStyleSheet("""
            QPushButton {
                background-color: transparent;
                border: 1px solid rgba(239, 68, 68, 0.3);
                border-radius: 16px;
                padding-left: 7px;
                padding-bottom: 10px;
            }

            QPushButton:hover {
                background-color: rgba(239, 68, 68, 0.2);
                border: 1px solid #ef4444;
            }

            QPushButton:pressed {
                background-color: rgba(239, 68, 68, 0.35);
                padding-top: 2px;
            }
        """)

        self.add_btn.clicked.connect(lambda: self.add_clicked.emit(self.game_data))
        self.edit_btn.clicked.connect(lambda: self.edit_clicked.emit(self.game_data))
        self.del_btn.clicked.connect(lambda: self.delete_clicked.emit(self.game_data))
        
        bottom_row.addLayout(info_vbox)
        bottom_row.addStretch()
        bottom_row.addLayout(self.action_layout)

        content_layout.addLayout(bottom_row)

        layout.addWidget(content)
        layout.addStretch()

        self.setup_animations()

    def update_ui(self, game_data):
        self.game_data = game_data
        
        status = game_data.get("status", "Like to Play")
        colors = {
            "Playing": "#10b981",
            "Completed": "#3b82f6",
            "Like to Play": "#f59e0b",
            "Stopped": "#ef4444"
        }
        color = colors.get(status, "#ffffff")
        
        if "exe_path" in game_data:
            last_played = game_data.get("last_played", "Never")
            self.status_label.setText(f"{last_played.upper()}\n")
            self.status_label.setStyleSheet("color: #a0a0b0; font-weight: 800; font-size: 11px; letter-spacing: 1px;")
        else:
            self.status_label.setText(status.upper())
            self.status_label.setStyleSheet(f"color: {color}; font-weight: 800; font-size: 11px; letter-spacing: 1px;")

        rating = game_data.get("rating", 0.0)
        self.rating_text.setText(f" {rating}")
        
        if status == "Like to Play" or "exe_path" in game_data:
            self.rating_widget.hide()
        else:
            self.rating_widget.show()
            
        playtime = float(game_data.get("playtime", 0))
        hours = int(playtime)
        minutes = int((playtime - hours) * 60)
        if hours > 0:
            self.playtime_text.setText(f"{hours}h {minutes}m")
        else:
            self.playtime_text.setText(f"{minutes}m")
            
        if playtime <= 0:
            self.playtime_text.hide()
        else:
            self.playtime_text.show()

    def set_image(self, pixmap):
        try:
            scaled = pixmap.scaled(240, 135, Qt.KeepAspectRatioByExpanding, Qt.SmoothTransformation)
            
            crop_rect = QRect(
                (scaled.width() - 240) // 2,
                (scaled.height() - 135) // 2,
                240, 135
            )
            cropped = scaled.copy(crop_rect)

            rounded = QPixmap(240, 135)
            rounded.fill(Qt.transparent)

            painter = QPainter(rounded)
            painter.setRenderHint(QPainter.Antialiasing)
            
            path = QPainterPath()
            path.setFillRule(Qt.WindingFill)
            path.addRoundedRect(0, 0, 240, 135, 20, 20)
            path.addRect(0, 20, 240, 115)
            
            painter.setClipPath(path)
            painter.drawPixmap(0, 0, cropped)
            painter.end()

            self.image_label.setPixmap(rounded)
        except RuntimeError:
            pass

    def setup_animations(self):
        self.anim_move = QPropertyAnimation(self.card_frame, b"pos")
        self.anim_move.setEasingCurve(QEasingCurve.OutCubic)
        self.anim_move.setDuration(250)
        self.hover_group = QParallelAnimationGroup()
        self.hover_group.addAnimation(self.anim_move)

    def enterEvent(self, event):
        self.card_frame.setStyleSheet(self.hover_style)

        self.anim_move.stop()
        self.anim_move.setStartValue(self.card_frame.pos())
        self.anim_move.setEndValue(QPoint(5, 0))
        self.hover_group.start()

        super().enterEvent(event)

    def leaveEvent(self, event):
        self.card_frame.setStyleSheet(self.base_style)

        self.anim_move.stop()
        self.anim_move.setStartValue(self.card_frame.pos())
        self.anim_move.setEndValue(QPoint(5, 5))
        self.hover_group.start()

        super().leaveEvent(event)


