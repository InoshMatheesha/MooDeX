import sys
from PySide6.QtWidgets import QApplication
from PySide6.QtCore import QTimer
import rawg_api

app = QApplication(sys.argv)
api = rawg_api.RawgApi()

def on_upcoming(d):
    print("UPCOMING:", len(d))

api.fetch_async("upcoming", "", on_upcoming)

QTimer.singleShot(10000, lambda: app.quit())
sys.exit(app.exec())
