from PySide6.QtWidgets import QDialog, QVBoxLayout, QHBoxLayout, QLabel, QComboBox, QSlider, QTextEdit, QPushButton, QFrame, QGraphicsDropShadowEffect
from PySide6.QtCore import Qt
from PySide6.QtGui import QColor

class EditDialog(QDialog):
    def __init__(self, game_data, parent=None):
        super().__init__(parent)
        self.game_data = game_data
        self.setWindowTitle(f"Edit - {game_data.get('name', 'Game')}")
        self.setFixedSize(400, 480)
        
        # Transparent background for the dialog itself to allow custom frameless styling if we wanted
        self.setStyleSheet("""
            QDialog {
                background-color: #0f1015;
            }
            QLabel {
                color: #ffffff;
                font-size: 14px;
                font-weight: 500;
            }
            QComboBox {
                padding: 10px 15px;
                background-color: #1c1d24;
                color: white;
                border: 1px solid #2d2e3a;
                border-radius: 8px;
                font-size: 14px;
            }
            QComboBox:focus {
                border: 1px solid #6366f1;
            }
            QComboBox::drop-down {
                border: none;
                width: 30px;
            }
            QTextEdit {
                background-color: #1c1d24;
                color: white;
                border: 1px solid #2d2e3a;
                border-radius: 8px;
                padding: 10px;
                font-size: 14px;
            }
            QTextEdit:focus {
                border: 1px solid #6366f1;
            }
            QSlider::groove:horizontal {
                border-radius: 4px;
                height: 8px;
                background: #2d2e3a;
            }
            QSlider::handle:horizontal {
                background: #6366f1;
                width: 18px;
                height: 18px;
                margin: -5px 0;
                border-radius: 9px;
            }
            QSlider::sub-page:horizontal {
                background: #6366f1;
                border-radius: 4px;
            }
        """)
        
        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(25, 25, 25, 25)
        main_layout.setSpacing(15)

        # Header Title
        title_label = QLabel("Edit Game Properties")
        title_label.setStyleSheet("font-size: 20px; font-weight: bold; color: #ffffff; margin-bottom: 5px;")
        main_layout.addWidget(title_label)
        
        game_name = QLabel(f"<span style='color: #a0a0b0;'>Editing: {game_data.get('name', 'Unknown')}</span>")
        game_name.setStyleSheet("font-size: 13px;")
        main_layout.addWidget(game_name)

        main_layout.addSpacing(10)

        # Status Dropdown
        status_lbl = QLabel("Status")
        status_lbl.setStyleSheet("color: #a0a0b0;")
        main_layout.addWidget(status_lbl)
        
        self.status_cb = QComboBox()
        self.status_cb.addItems(["Playing", "Completed", "Like to Play", "Stopped"])
        self.status_cb.setCurrentText(str(game_data.get("status", "Like to Play")))
        main_layout.addWidget(self.status_cb)
        
        main_layout.addSpacing(5)

        # Rating Slider
        rating_lbl = QHBoxLayout()
        self.r_title = QLabel("Personal Rating")
        self.r_title.setStyleSheet("color: #a0a0b0;")
        self.rating_label = QLabel(f"⭐ {int(game_data.get('rating', 0))}/10")
        self.rating_label.setStyleSheet("color: #fbbf24; font-weight: bold;")
        self.rating_label.setAlignment(Qt.AlignRight)
        
        rating_lbl.addWidget(self.r_title)
        rating_lbl.addWidget(self.rating_label)
        main_layout.addLayout(rating_lbl)
        
        self.rating_slider = QSlider(Qt.Horizontal)
        self.rating_slider.setRange(0, 10)
        self.rating_slider.setValue(int(game_data.get("rating", 0)))
        self.rating_slider.valueChanged.connect(lambda v: self.rating_label.setText(f"⭐ {v}/10"))
        main_layout.addWidget(self.rating_slider)
        
        # Hide rating if "Like to Play"
        self.status_cb.currentTextChanged.connect(self._on_status_changed)
        self._on_status_changed(self.status_cb.currentText())
        
        main_layout.addSpacing(5)

        # Notes
        notes_lbl = QLabel("Personal Notes")
        notes_lbl.setStyleSheet("color: #a0a0b0;")
        main_layout.addWidget(notes_lbl)
        
        self.notes_edit = QTextEdit()
        self.notes_edit.setPlaceholderText("Write your review or notes here...")
        self.notes_edit.setText(str(game_data.get("notes", "")))
        main_layout.addWidget(self.notes_edit)
        
        main_layout.addStretch()

        # Action Buttons
        btn_layout = QHBoxLayout()
        btn_layout.setSpacing(10)
        
        cancel_btn = QPushButton("Cancel")
        cancel_btn.setCursor(Qt.PointingHandCursor)
        cancel_btn.setStyleSheet("""
            QPushButton {
                padding: 10px 20px; 
                background-color: transparent; 
                color: #a0a0b0; 
                border-radius: 8px;
                border: 1px solid #2d2e3a;
                font-weight: bold;
                font-size: 14px;
            }
            QPushButton:hover {
                background-color: #2d2e3a;
                color: #ffffff;
            }
        """)
        cancel_btn.clicked.connect(self.reject)
        
        save_btn = QPushButton("Save Changes")
        save_btn.setCursor(Qt.PointingHandCursor)
        save_btn.setStyleSheet("""
            QPushButton {
                padding: 10px 20px; 
                background-color: #6366f1; 
                color: white; 
                border-radius: 8px;
                font-weight: bold;
                font-size: 14px;
            }
            QPushButton:hover {
                background-color: #4f46e5;
            }
        """)
        save_btn.clicked.connect(self.accept)
        
        btn_layout.addStretch()
        btn_layout.addWidget(cancel_btn)
        btn_layout.addWidget(save_btn)
        main_layout.addLayout(btn_layout)
        
    def _on_status_changed(self, text):
        visible = (text != "Like to Play")
        self.r_title.setVisible(visible)
        self.rating_label.setVisible(visible)
        self.rating_slider.setVisible(visible)

    def get_updates(self):
        return {
            "status": self.status_cb.currentText(),
            "rating": self.rating_slider.value() if self.status_cb.currentText() != "Like to Play" else 0,
            "notes": self.notes_edit.toPlainText()
        }
