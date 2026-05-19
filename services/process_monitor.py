import psutil
import time
import os

from PySide6.QtCore import QThread, Signal
from datetime import datetime


class ProcessMonitor(QThread):

    game_started = Signal(str)
    game_stopped = Signal(str, int)

    def __init__(self, data_manager):
        super().__init__()

        self.data_manager = data_manager

        self.running = True

        # exe_name -> start_time
        self.active_games = {}

    # =========================================
    # FAST INTERRUPTIBLE SLEEP
    # =========================================

    def _sleep_interruptible(self, seconds):

        ticks = int(seconds / 0.5)

        for _ in range(ticks):

            if not self.running or self.isInterruptionRequested():
                return

            time.sleep(0.5)

    # =========================================
    # MAIN LOOP
    # =========================================

    def run(self):

        while self.running and not self.isInterruptionRequested():

            local_games = self.data_manager.data.get("local_games", [])

            # no games
            if not local_games:

                self._sleep_interruptible(1)

                continue

            # =========================================
            # BUILD TRACKED GAME MAP
            # exe_name -> game_data
            # =========================================

            tracked_games = {}

            for game in local_games:

                exe_path = game.get("exe_path", "")

                if exe_path:

                    exe_name = os.path.basename(exe_path).lower()

                    tracked_games[exe_name] = game

            # =========================================
            # CURRENT RUNNING GAMES
            # =========================================

            current_running = set()

            for proc in psutil.process_iter(['name']):

                try:

                    proc_name = proc.info.get("name")

                    if not proc_name:
                        continue

                    proc_name = proc_name.lower()

                    # game detected
                    if proc_name in tracked_games:

                        current_running.add(proc_name)

                except (
                    psutil.NoSuchProcess,
                    psutil.AccessDenied,
                    psutil.ZombieProcess
                ):
                    pass

            # =========================================
            # NEWLY STARTED GAMES
            # =========================================

            for proc_name in current_running:

                if proc_name not in self.active_games:

                    self.active_games[proc_name] = time.time()

                    game = tracked_games[proc_name]

                    game_name = game.get(
                        "name",
                        "Unknown Game"
                    )

                    print(f"[TRACKER] STARTED -> {game_name}")

                    self.game_started.emit(game_name)

            # =========================================
            # STOPPED GAMES
            # =========================================

            stopped_games = []

            for proc_name, start_time in self.active_games.items():

                if proc_name not in current_running:

                    duration = int(time.time() - start_time)

                    game = tracked_games.get(proc_name)

                    if game:

                        game_name = game.get(
                            "name",
                            "Unknown Game"
                        )

                        old_playtime = float(
                            game.get("playtime", 0)
                        )

                        added_hours = duration / 3600

                        new_playtime = round(
                            old_playtime + added_hours,
                            2
                        )

                        updates = {
                            "playtime": new_playtime,
                            "last_played": datetime.now().strftime("%Y-%m-%d")
                        }

                        # SAVE
                        self.data_manager.update_local_game(
                            game.get("exe_path"),
                            updates
                        )
                        self.data_manager.data_changed.emit()

                        print(
                            f"[TRACKER] STOPPED -> {game_name} | +{added_hours:.2f}h"
                        )

                        self.game_stopped.emit(
                            game_name,
                            duration
                        )

                    stopped_games.append(proc_name)

            # cleanup
            for proc_name in stopped_games:

                del self.active_games[proc_name]

            # =========================================
            # CHECK EVERY SECOND
            # =========================================

            self._sleep_interruptible(1)

    # =========================================
    # STOP THREAD
    # =========================================

    def stop(self):

        self.running = False

        self.requestInterruption()