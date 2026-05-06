import json
import os
from PySide6.QtCore import QObject, Signal, Slot

class DataManager(QObject):
    data_changed = Signal()

    def __init__(self, filepath="games_data.json"):
        super().__init__()
        self.filepath = filepath
        self.data = {"games": [], "local_games": [], "now_playing": None}
        self.load_data()

    def load_data(self):
        if os.path.exists(self.filepath):
            try:
                with open(self.filepath, 'r', encoding='utf-8') as f:
                    self.data = json.load(f)
            except Exception:
                pass

    def save_data(self):
        with open(self.filepath, 'w', encoding='utf-8') as f:
            json.dump(self.data, f, indent=4)
        self.data_changed.emit()

    def add_game(self, game_dict):
        name = game_dict.get("name", "").lower()
        # Prevent duplicates
        for g in self.data["games"]:
            if g.get("name", "").lower() == name:
                return False
        # Insert at top
        self.data["games"].insert(0, game_dict)
        self.save_data()
        return True

    def remove_game(self, name):
        name_lower = name.lower()
        self.data["games"] = [g for g in self.data["games"] if g.get("name", "").lower() != name_lower]
        if self.data.get("now_playing") == name:
            self.data["now_playing"] = None
        self.save_data()

    def update_game(self, name, updates):
        name_lower = name.lower()
        for g in self.data["games"]:
            if g.get("name", "").lower() == name_lower:
                g.update(updates)
                break
        self.save_data()

    def get_games(self):
        return self.data["games"]

    def add_local_game(self, game_dict):
        if "local_games" not in self.data:
            self.data["local_games"] = []
            
        path = game_dict.get("exe_path", "").lower()
        for g in self.data["local_games"]:
            if g.get("exe_path", "").lower() == path:
                return False
        self.data["local_games"].insert(0, game_dict)
        self.save_data()
        return True

    def remove_local_game(self, exe_path):
        if "local_games" not in self.data: return
        path_lower = exe_path.lower()
        self.data["local_games"] = [g for g in self.data["local_games"] if g.get("exe_path", "").lower() != path_lower]
        self.save_data()

    def update_local_game(self, exe_path, updates):
        if "local_games" not in self.data: return
        path_lower = exe_path.lower()
        for g in self.data["local_games"]:
            if g.get("exe_path", "").lower() == path_lower:
                g.update(updates)
                break
        self.save_data()
