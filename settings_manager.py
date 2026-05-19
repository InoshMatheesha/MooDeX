import os
import sys
import json
import winreg
from pathlib import Path

# ===============================
# APPDATA DIRECTORY
# ===============================

APP_DIR = Path(os.getenv("LOCALAPPDATA")) / "MooDeX"
APP_DIR.mkdir(parents=True, exist_ok=True)

# ===============================
# SETTINGS FILE
# ===============================

_SETTINGS_FILE = APP_DIR / "user_settings.json"

# ===============================
# DEFAULT SETTINGS
# ===============================

_DEFAULTS = {
    "close_to_tray": True,
    "autostart": False,
    "launch_minimized": False,
    "lazy_discover": True,
    "api_cache": True,
}

# ===============================
# WINDOWS STARTUP
# ===============================

APP_NAME = "MooDeX"

def set_startup(enable=True):

    import sys
    import os
    import winreg

    key_path = r"Software\Microsoft\Windows\CurrentVersion\Run"

    try:
        key = winreg.OpenKey(
            winreg.HKEY_CURRENT_USER,
            key_path,
            0,
            winreg.KEY_ALL_ACCESS
        )

        if enable:

            # Detect EXE path correctly
            if getattr(sys, 'frozen', False):
                app_path = sys.executable
            else:
                app_path = os.path.abspath(sys.argv[0])

            command = f'"{app_path}" --startup'

            winreg.SetValueEx(
                key,
                "MooDeX",
                0,
                winreg.REG_SZ,
                command
            )

            print("[Startup] Added:", command)

        else:

            try:
                winreg.DeleteValue(key, "MooDeX")
                print("[Startup] Removed")
            except FileNotFoundError:
                pass

        winreg.CloseKey(key)

    except Exception as e:
        print("[Startup Error]", e)

# ===============================
# LOAD SETTINGS
# ===============================

def load_settings() -> dict:

    if _SETTINGS_FILE.exists():

        try:
            with open(_SETTINGS_FILE, "r", encoding="utf-8") as f:
                data = json.load(f)

            for k, v in _DEFAULTS.items():
                data.setdefault(k, v)

            return data

        except Exception as e:
            print(f"[Settings] Load error: {e}")

    return dict(_DEFAULTS)

# ===============================
# SAVE SETTINGS
# ===============================

def save_settings(settings: dict):

    try:

        with open(_SETTINGS_FILE, "w", encoding="utf-8") as f:
            json.dump(settings, f, indent=2)

        # Apply startup instantly
        set_startup(settings.get("autostart", False))

    except Exception as e:
        print(f"[Settings] Could not save: {e}")