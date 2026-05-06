from PySide6.QtWidgets import QLabel, QWidget, QVBoxLayout
from PySide6.QtCore import Qt, QTimer, QPropertyAnimation, QEasingCurve, QPoint
from PySide6.QtGui import QColor, QPalette

class Toast(QWidget):
    def __init__(self, parent, message, type="success", duration=3000):
        super().__init__(parent)
        self.message = message
        self.duration = duration
        
        self.setWindowFlags(Qt.FramelessWindowHint | Qt.Tool | Qt.WindowStaysOnTopHint)
        self.setAttribute(Qt.WA_TranslucentBackground)
        self.setAttribute(Qt.WA_ShowWithoutActivating)

        layout = QVBoxLayout(self)
        self.label = QLabel(message)
        self.label.setAlignment(Qt.AlignCenter)
        
        # Colors: success, info, warn, error
        colors = {
            "success": "#4CAF50",
            "info": "#2196F3",
            "warn": "#FF9800",
            "error": "#F44336"
        }
        bg_color = colors.get(type, "#333333")
        
        self.label.setStyleSheet(f"""
            QLabel {{
                background-color: {bg_color};
                color: white;
                padding: 10px 20px;
                border-radius: 5px;
                font-weight: bold;
            }}
        """)
        layout.addWidget(self.label)
        self.adjustSize()

        # Animation setup
        self.animation = QPropertyAnimation(self, b"windowOpacity")
        self.animation.setDuration(300)
        self.animation.setStartValue(0.0)
        self.animation.setEndValue(1.0)
        self.animation.setEasingCurve(QEasingCurve.InOutQuad)

        self.timer = QTimer(self)
        self.timer.setSingleShot(True)
        self.timer.timeout.connect(self.hide_toast)

    def show_toast(self):
        if self.parent():
            parent_rect = self.parent().rect()
            # Position at bottom-right of parent
            x = parent_rect.width() - self.width() - 20
            y = parent_rect.height() - self.height() - 20
            self.move(self.parent().mapToGlobal(parent_rect.topLeft()) + QPoint(x, y))
            
        self.show()
        self.animation.start()
        self.timer.start(self.duration)

    def hide_toast(self):
        self.animation.setStartValue(1.0)
        self.animation.setEndValue(0.0)
        self.animation.finished.connect(self.close)
        self.animation.start()

# Global helper pattern
def show_toast(parent, message, type="success"):
    toast = Toast(parent, message, type)
    toast.show_toast()
