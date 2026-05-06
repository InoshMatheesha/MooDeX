import sys
from PySide6.QtWidgets import QApplication
from PySide6.QtCore import QTimer
import rawg_api

app = QApplication(sys.argv)
api = rawg_api.RawgApi()

def on_trending(d):
    print('trending count:', len(d))

def on_upcoming(d):
    print('upcoming count:', len(d))

api.fetch_async('trending', '', on_trending)
api.fetch_async('upcoming', '', on_upcoming)

QTimer.singleShot(3000, app.quit)
sys.exit(app.exec())
