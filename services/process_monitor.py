import psutil
import time
import os
from PySide6.QtCore import QThread, Signal
from datetime import datetime

class ProcessMonitor(QThread):
    game_started = Signal(str) # name of game
    game_stopped = Signal(str, int) # name, duration in seconds
    
    def __init__(self, data_manager):
        super().__init__()
        self.data_manager = data_manager
        self.running = True
        self.active_games = {} # path -> start_time
        
    def _sleep_interruptible(self, seconds):
        """Sleep in small ticks so we can exit quickly when stopped."""
        ticks = int(seconds / 0.5)
        for _ in range(ticks):
            if not self.running or self.isInterruptionRequested():
                return
            time.sleep(0.5)

    def run(self):
        while self.running and not self.isInterruptionRequested():
            local_games = self.data_manager.data.get("local_games", [])
            if not local_games:
                self._sleep_interruptible(5)
                continue
                
            # build map of exe_path -> game dict
            tracked_exes = {g.get("exe_path", "").lower(): g for g in local_games if g.get("exe_path")}
            
            # Map of filenames to check against process name to avoid querying full path for every process
            tracked_filenames = {os.path.basename(path): path for path in tracked_exes.keys()}
            
            # get current running exes
            current_exes = set()
            for proc in psutil.process_iter(['name']):
                try:
                    p_name = proc.info.get('name')
                    if p_name and p_name.lower() in tracked_filenames:
                        exe = proc.exe()
                        if exe:
                            current_exes.add(exe.lower())
                except (psutil.NoSuchProcess, psutil.AccessDenied, psutil.ZombieProcess, AttributeError):
                    pass
                    
            # check newly started
            for exe in current_exes:
                if exe in tracked_exes and exe not in self.active_games:
                    self.active_games[exe] = time.time()
                    game_name = tracked_exes[exe].get("name", "Unknown Game")
                    self.game_started.emit(game_name)
                    
            # check stopped
            stopped = []
            for exe, start_time in self.active_games.items():
                if exe not in current_exes:
                    duration = int(time.time() - start_time)
                    game_name = tracked_exes.get(exe, {}).get("name", "Unknown Game")
                    self.game_stopped.emit(game_name, duration)
                    
                    # Auto update playtime in data_manager
                    if exe in tracked_exes:
                        game = tracked_exes[exe]
                        added_hours = round(duration / 3600.0, 2)
                        updates = {
                            "playtime": game.get("playtime", 0) + added_hours,
                            "last_played": datetime.now().strftime("%Y-%m-%d")
                        }
                        self.data_manager.update_local_game(exe, updates)
                    stopped.append(exe)
                    
            for exe in stopped:
                del self.active_games[exe]
                
            self._sleep_interruptible(5)  # Check every 5 seconds
            
    def stop(self):
        self.running = False
        self.requestInterruption()
        # Don't block – main.py gives us 500 ms via QTimer before force-quit
