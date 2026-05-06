import customtkinter as ctk
import json
import os
import threading
import time
from datetime import datetime
import requests
from io import BytesIO
from concurrent.futures import ThreadPoolExecutor
from PIL import Image, ImageOps, ImageDraw

from rawg_api import RAWGApiClient

# --- Constants ---
T = {
    "bg":       "#080b14",
    "sidebar":  "#0d1120",
    "card":     "#111827",
    "card_h":   "#1a2236",
    "surface":  "#0f1623",
    "surface2": "#161e2e",
    "border":   "#1e2d45",
    "border2":  "#253448",
    "accent":   "#6366f1",
    "accent_h": "#818cf8",
    "accent2":  "#06b6d4",
    "green":    "#10b981",
    "purple":   "#a78bfa",
    "amber":    "#fbbf24",
    "red":      "#f87171",
    "blue":     "#38bdf8",
    "text":     "#e2e8f0",
    "text2":    "#94a3b8",
    "text3":    "#4b5e78",
}

STATUS = {
    "Playing":      {"c": T["green"],  "i": "▶", "bg": "#0b3d2b"},
    "Completed":    {"c": T["purple"], "i": "✓", "bg": "#2c1b4d"},
    "Like to Play": {"c": T["amber"],  "i": "♡", "bg": "#4f3e12"},
    "Stopped":      {"c": T["red"],    "i": "■", "bg": "#4d1f1f"},
    "Wishlist":     {"c": T["blue"],   "i": "★", "bg": "#12394f"},
}

SORT_OPTIONS = ["Recently Added", "Name A–Z", "Name Z–A", "Rating ↑", "Rating ↓", "Release Date"]
FILTER_OPTIONS = ["All"] + list(STATUS.keys())

FONTS = {
    "h1": ("Segoe UI", 24, "bold"),
    "h2": ("Segoe UI", 18, "bold"),
    "body": ("Segoe UI", 13),
    "small": ("Segoe UI", 11),
    "bold": ("Segoe UI", 13, "bold"),
    "stat_num": ("Segoe UI", 28, "bold")
}

CARD_MIN = 180
CARD_GAP = 20

# Shared thread pool for image loading (prevents thread explosion)
_img_executor = ThreadPoolExecutor(max_workers=4)

# Placeholder image cache
_placeholder_cache = {}

# --- Helpers ---
def star_str(rating):
    if not rating: return "No rating"
    return "★" * (rating // 2) + "☆" * (5 - (rating // 2)) + f" {rating}/10"

def make_placeholder(w, h):
    key = (w, h)
    if key in _placeholder_cache:
        return _placeholder_cache[key]
    img = Image.new("RGB", (w, h), color="#0d1829")
    draw = ImageDraw.Draw(img)
    for x in range(0, w, 40):
        draw.line([(x, 0), (x, h)], fill="#1a2236")
    for y in range(0, h, 40):
        draw.line([(0, y), (w, y)], fill="#1a2236")
    result = ctk.CTkImage(light_image=img, dark_image=img, size=(w, h))
    _placeholder_cache[key] = result
    return result

# --- Toast ---
class Toast(ctk.CTkToplevel):
    def __init__(self, parent, message, kind="info", duration=2600):
        super().__init__(parent)
        self.overrideredirect(True)
        self.attributes("-topmost", True)
        
        colors = {"info": T["accent"], "success": T["green"], "warn": T["amber"], "error": T["red"]}
        color = colors.get(kind, T["accent"])
        
        # Windows transparent window trick for true rounded corners
        tc = "#000001"
        self.configure(fg_color=tc)
        try:
            self.attributes("-transparentcolor", tc)
        except Exception:
            pass
            
        frame = ctk.CTkFrame(self, fg_color=T["surface2"], border_width=2, border_color=color, corner_radius=20)
        frame.pack(fill="both", expand=True, padx=10, pady=10)
        
        lbl = ctk.CTkLabel(frame, text=message, font=FONTS["bold"], text_color=T["text"], padx=15, pady=10)
        lbl.pack(side="left", fill="both", expand=True)
        
        self.update_idletasks()
        w, h = 330, 70
        
        parent_x = parent.winfo_rootx()
        parent_y = parent.winfo_rooty()
        parent_w = parent.winfo_width()
        parent_h = parent.winfo_height()
        
        x = parent_x + parent_w - w - 20
        y = parent_y + parent_h - h - 20
        self.geometry(f"{w}x{h}+{x}+{y}")
        
        self.after(duration, self.destroy)

# --- DataManager ---
class DataManager:
    def __init__(self):
        self.file = os.path.join(os.path.dirname(os.path.abspath(__file__)), "games_data.json")
        self.data = {"games": [], "now_playing": None}
        self.load()

    def load(self):
        if os.path.exists(self.file):
            try:
                with open(self.file, "r", encoding="utf-8") as f:
                    self.data = json.load(f)
            except Exception:
                pass

    def save(self):
        with open(self.file, "w", encoding="utf-8") as f:
            json.dump(self.data, f, indent=2, ensure_ascii=False)

    def add(self, name, status, rating, released, image_url, genres, platforms, playtime):
        for g in self.data["games"]:
            if g["name"].lower() == name.lower():
                return False
        
        self.data["games"].insert(0, {
            "name": name, "status": status, "rating": rating, "released": released,
            "image_url": image_url, "added": datetime.now().strftime("%Y-%m-%d"),
            "notes": "", "genres": genres, "platforms": platforms, "playtime": playtime,
            "favorite": False
        })
        if status == "Playing":
            self.set_playing(name)
        self.save()
        return True

    def remove(self, name):
        self.data["games"] = [g for g in self.data["games"] if g["name"] != name]
        if self.data.get("now_playing") == name:
            self.data["now_playing"] = None
        self.save()

    def update(self, name, status, rating, notes):
        for g in self.data["games"]:
            if g["name"] == name:
                g["status"] = status
                g["rating"] = rating
                g["notes"] = notes
                if status == "Playing":
                    self.set_playing(name)
                elif self.data.get("now_playing") == name and status != "Playing":
                    self.data["now_playing"] = None
                break
        self.save()

    def toggle_favorite(self, name):
        for g in self.data["games"]:
            if g["name"] == name:
                g["favorite"] = not g.get("favorite", False)
                break
        self.save()

    def set_playing(self, name):
        self.data["now_playing"] = name
        for g in self.data["games"]:
            if g["name"] == name:
                g["status"] = "Playing"
                break
        self.save()

    def get_filtered_sorted(self, status_filter, sort_by, search):
        games = self.data["games"]
        if search:
            q = search.lower()
            games = [g for g in games if q in g["name"].lower()]
        if status_filter and status_filter != "All":
            games = [g for g in games if g["status"] == status_filter]
            
        if sort_by == "Name A–Z":
            games.sort(key=lambda x: x["name"].lower())
        elif sort_by == "Name Z–A":
            games.sort(key=lambda x: x["name"].lower(), reverse=True)
        elif sort_by == "Rating ↑":
            games.sort(key=lambda x: x["rating"])
        elif sort_by == "Rating ↓":
            games.sort(key=lambda x: x["rating"], reverse=True)
        elif sort_by == "Release Date":
            games.sort(key=lambda x: x["released"], reverse=True)
        else: # Recently Added
            games.sort(key=lambda x: x["added"], reverse=True)
            
        return games

    def stats(self):
        total = len(self.data["games"])
        by_status = {k: 0 for k in STATUS.keys()}
        total_rating = 0
        rated_count = 0
        favs = 0
        for g in self.data["games"]:
            by_status[g["status"]] = by_status.get(g["status"], 0) + 1
            if g["rating"] > 0:
                total_rating += g["rating"]
                rated_count += 1
            if g.get("favorite"):
                favs += 1
                
        avg = round(total_rating / rated_count, 1) if rated_count > 0 else 0
        return {
            "total": total, "by_status": by_status,
            "avg_rating": avg, "favorites": favs
        }

# --- GameCard ---
class GameCard(ctk.CTkFrame):
    _img_cache = {}

    def __init__(self, parent, game, card_w, mode="library", on_action=None):
        super().__init__(parent, width=card_w, fg_color=T["card"], corner_radius=8)
        self.game = game
        self.mode = mode
        self.on_action = on_action
        self._hov = False
        
        self.grid_propagate(False)
        self.pack_propagate(False)
        
        img_h = int(card_w * 0.60)   # slightly taller like your screenshot
        self.configure(width=card_w, height=img_h + 110, border_width=1, border_color=T["border"])
        
        # Image — pinned to exact pixel size so it never over/underflows
        self.img_lbl = ctk.CTkLabel(self, text="", width=card_w, height=img_h, image=make_placeholder(card_w, img_h))
        self.img_lbl.pack_propagate(False)
        self.img_lbl.pack(fill="x")
        
        self.load_image(game.get("image_url") or game.get("background_image"), card_w, img_h)
        
        if mode == "library":
            # Status Badge
            stat = STATUS.get(game["status"], STATUS["Stopped"])
            badge = ctk.CTkFrame(self.img_lbl, fg_color=stat["bg"], corner_radius=6)
            badge.place(x=10, y=10)
            ctk.CTkLabel(badge, text=f"{stat['i']} {game['status']}", text_color=stat["c"], 
                         font=FONTS["small"], width=1, height=1).pack(padx=6, pady=2)
            
            # Favorite
            fav = game.get("favorite", False)
            fav_text = "★" if fav else "☆"
            fav_color = T["amber"] if fav else T["text2"]
            self.fav_lbl = ctk.CTkLabel(self.img_lbl, text=fav_text, text_color=fav_color, font=FONTS["h2"])
            self.fav_lbl.place(relx=1.0, y=10, anchor="ne", x=-10)
            self.fav_lbl.bind("<Button-1>", lambda e: self.on_action("fav", self.game))
            
            # Action Overlay
            self.overlay = ctk.CTkFrame(self.img_lbl, fg_color="#050811", width=40, height=img_h, corner_radius=0)
            b_play = ctk.CTkButton(self.overlay, text="▶", width=30, fg_color="transparent", hover_color=T["green"], command=lambda: self.on_action("play", self.game))
            b_play.pack(pady=(5,2))
            b_edit = ctk.CTkButton(self.overlay, text="✎", width=30, fg_color="transparent", hover_color=T["accent"], command=lambda: self.on_action("edit", self.game))
            b_edit.pack(pady=2)
            b_del = ctk.CTkButton(self.overlay, text="✕", width=30, fg_color="transparent", hover_color=T["red"], command=lambda: self.on_action("delete", self.game))
            b_del.pack(pady=2)
            
        # Meta info
        meta_f = ctk.CTkFrame(self, fg_color="transparent")
        meta_f.pack(fill="x", padx=10, pady=8)
        
        name = game["name"]
        if len(name) > 42:
            name = name[:40] + "..."
            
        title_lbl = ctk.CTkLabel(meta_f, text=name, font=FONTS["bold"], text_color=T["text"], anchor="w", justify="left", wraplength=card_w - 28, height=40)
        title_lbl.pack(fill="x")
        
        bot_f = ctk.CTkFrame(meta_f, fg_color="transparent")
        bot_f.pack(fill="x", side="bottom")
        
        year = game.get("released", "Unknown")[:4] if game.get("released") else "Unknown"
        genre = game.get("genres", [""])[0] if game.get("genres") else ""
        sub_text = f"{year} • {genre}" if genre else year
        ctk.CTkLabel(bot_f, text=sub_text, font=FONTS["small"], text_color=T["text2"]).pack(side="left")
        
        if mode == "library":
            rating = game.get("rating", 0)
            ctk.CTkLabel(bot_f, text=f"★ {rating}/10" if rating else "Unrated", font=FONTS["small"], text_color=T["accent"] if rating else T["text3"]).pack(side="right")
        else:
            add_btn = ctk.CTkButton(bot_f, text="+ Add", width=50, height=20, font=FONTS["small"], fg_color=T["surface2"], hover_color=T["accent"], command=lambda: self.on_action("add", self.game))
            add_btn.pack(side="right")
            
        self.bind("<Enter>", self._on_enter)
        self.bind("<Leave>", self._on_leave)
        for w in self.winfo_children():
            w.bind("<Enter>", self._on_enter)
            w.bind("<Leave>", self._on_leave)

    def load_image(self, url, w, h):
        if not url:
            return

        cache_key = f"{url}_{w}x{h}"
        if cache_key in self._img_cache:
            self.img_lbl.configure(image=self._img_cache[cache_key])
            return

        def fetch():
            try:
                headers = {"User-Agent": "MooDeX/1.0"}
                resp = requests.get(url, timeout=10, headers=headers)
                resp.raise_for_status()
                img = Image.open(BytesIO(resp.content)).convert("RGB")

                img = ImageOps.fit(img, (w, h))

                ctk_img = ctk.CTkImage(light_image=img, dark_image=img, size=(w, h))
                self._img_cache[cache_key] = ctk_img

                if self.winfo_exists():
                    self.img_lbl.after(0, lambda: self.img_lbl.configure(image=ctk_img))

            except:
                pass

        _img_executor.submit(fetch)

    def _on_enter(self, e):
        if not self._hov:
            self.configure(fg_color=T["card_h"])
        self._hov = True
        if self.mode == "library":
            self.overlay.place(relx=1, rely=0, anchor="ne")

    def _on_leave(self, e):
        self._hov = False
        self.after(40, self._check_leave)

    def _check_leave(self):
        if not self.winfo_exists() or self._hov: return
        x, y = self.winfo_pointerx(), self.winfo_pointery()
        rx, ry = self.winfo_rootx(), self.winfo_rooty()
        w, h = self.winfo_width(), self.winfo_height()
        if not (rx <= x <= rx + w and ry <= y <= ry + h):
            self.configure(fg_color=T["card"], border_color=T["border"])
            if self.mode == "library":
                self.overlay.place_forget()

# --- StatsBar ---
class StatsBar(ctk.CTkFrame):
    def __init__(self, parent, dm):
        super().__init__(parent, fg_color="transparent")
        stats = dm.stats()
        
        items = [
            ("Total", str(stats["total"]), "🎮", T["text"]),
            ("Playing", str(stats["by_status"].get("Playing", 0)), "▶", T["green"]),
            ("Completed", str(stats["by_status"].get("Completed", 0)), "✓", T["purple"]),
            ("Favourites", str(stats["favorites"]), "★", T["amber"]),
            ("Avg Score", f"{stats['avg_rating']}/10", "⭐", T["accent"]),
        ]
        
        for i, (lbl, val, icon, color) in enumerate(items):
            self.grid_columnconfigure(i, weight=1)
            f = ctk.CTkFrame(self, fg_color=T["surface"], corner_radius=8)
            f.grid(row=0, column=i, padx=4, sticky="ew")
            
            top = ctk.CTkFrame(f, fg_color="transparent")
            top.pack(pady=(10, 0))
            ctk.CTkLabel(top, text=icon, text_color=color, font=FONTS["stat_num"]).pack(side="left", padx=4)
            ctk.CTkLabel(top, text=val, text_color=T["text"], font=FONTS["stat_num"]).pack(side="left")
            
            ctk.CTkLabel(f, text=lbl, text_color=T["text2"], font=FONTS["small"]).pack(pady=(0, 10))

# --- Dialogs ---
class EditDialog(ctk.CTkToplevel):
    def __init__(self, parent, game, on_save):
        super().__init__(parent)
        self.title("Edit Game")
        self.geometry("420x440")
        self.configure(fg_color=T["surface"])
        self.attributes("-topmost", True)
        self.grab_set()
        
        self.update_idletasks()
        x = parent.winfo_rootx() + (parent.winfo_width() - 420) // 2
        y = parent.winfo_rooty() + (parent.winfo_height() - 440) // 2
        self.geometry(f"+{x}+{y}")
        
        ctk.CTkLabel(self, text="Edit Game", font=FONTS["h1"], text_color=T["text"]).pack(pady=(20, 5))
        ctk.CTkLabel(self, text=game["name"], font=FONTS["h2"], text_color=T["accent"]).pack()
        
        main = ctk.CTkFrame(self, fg_color="transparent")
        main.pack(fill="both", expand=True, padx=30, pady=20)
        
        # Status
        ctk.CTkLabel(main, text="Status", text_color=T["text2"], font=FONTS["small"]).pack(anchor="w")
        self.status_var = ctk.StringVar(value=game["status"])
        ctk.CTkOptionMenu(main, variable=self.status_var, values=list(STATUS.keys()), fg_color=T["surface2"]).pack(fill="x", pady=(0, 15))
        
        # Rating
        rating_f = ctk.CTkFrame(main, fg_color="transparent")
        rating_f.pack(fill="x", pady=(0, 5))
        ctk.CTkLabel(rating_f, text="Rating", text_color=T["text2"], font=FONTS["small"]).pack(side="left")
        self.rating_lbl = ctk.CTkLabel(rating_f, text=star_str(game.get("rating", 0)), text_color=T["accent"])
        self.rating_lbl.pack(side="right")
        
        self.rating_var = ctk.IntVar(value=game.get("rating", 0))
        slider = ctk.CTkSlider(main, from_=1, to=10, number_of_steps=9, variable=self.rating_var, command=self._update_rating)
        slider.pack(fill="x", pady=(0, 15))
        
        # Notes
        ctk.CTkLabel(main, text="Notes", text_color=T["text2"], font=FONTS["small"]).pack(anchor="w")
        self.notes = ctk.CTkTextbox(main, height=60, fg_color=T["surface2"])
        self.notes.pack(fill="x", pady=(0, 20))
        self.notes.insert("1.0", game.get("notes", ""))
        
        # Buttons
        btn_f = ctk.CTkFrame(self, fg_color="transparent")
        btn_f.pack(fill="x", padx=30, pady=(0, 20))
        ctk.CTkButton(btn_f, text="Cancel", fg_color=T["surface2"], hover_color=T["border"], command=lambda: self.after(10, self.destroy), width=100).pack(side="left")
        ctk.CTkButton(btn_f, text="Save Changes", fg_color=T["accent"], hover_color=T["accent_h"], command=lambda: self._save(on_save)).pack(side="right")

    def _update_rating(self, val):
        self.rating_lbl.configure(text=star_str(int(val)))

    def _save(self, on_save):
        s = self.status_var.get()
        r = int(self.rating_var.get())
        n = self.notes.get("1.0", "end-1c")
        
        def _execute():
            self.destroy()
            on_save(s, r, n)
            
        self.after(10, _execute)

class AddDialog(ctk.CTkToplevel):
    def __init__(self, parent, api, on_add):
        super().__init__(parent)
        self.title("Add a Game")
        self.geometry("460x510")
        self.configure(fg_color=T["surface"])
        self.attributes("-topmost", True)
        self.grab_set()
        
        self.api = api
        self.on_add = on_add
        self._selected = None
        
        self.update_idletasks()
        x = parent.winfo_rootx() + (parent.winfo_width() - 460) // 2
        y = parent.winfo_rooty() + (parent.winfo_height() - 510) // 2
        self.geometry(f"+{x}+{y}")
        
        ctk.CTkLabel(self, text="Add a Game", font=FONTS["h1"], text_color=T["text"]).pack(pady=15)
        
        search_f = ctk.CTkFrame(self, fg_color="transparent")
        search_f.pack(fill="x", padx=20)
        self.entry = ctk.CTkEntry(search_f, placeholder_text="Search RAWG...", fg_color=T["surface2"])
        self.entry.pack(side="left", fill="x", expand=True, padx=(0, 10))
        self.entry.bind("<Return>", lambda e: self._search())
        ctk.CTkButton(search_f, text="Search", width=80, fg_color=T["accent"], command=self._search).pack(side="right")
        
        self.results_f = ctk.CTkScrollableFrame(self, height=150, fg_color=T["surface2"])
        self.results_f.pack(fill="x", padx=20, pady=10)
        
        bot_f = ctk.CTkFrame(self, fg_color="transparent")
        bot_f.pack(fill="both", expand=True, padx=20)
        
        ctk.CTkLabel(bot_f, text="Status", text_color=T["text2"], font=FONTS["small"]).pack(anchor="w")
        self.status_var = ctk.StringVar(value="Like to Play")
        ctk.CTkOptionMenu(bot_f, variable=self.status_var, values=list(STATUS.keys()), fg_color=T["surface2"]).pack(fill="x", pady=(0, 10))
        
        rating_f = ctk.CTkFrame(bot_f, fg_color="transparent")
        rating_f.pack(fill="x")
        ctk.CTkLabel(rating_f, text="Rating", text_color=T["text2"], font=FONTS["small"]).pack(side="left")
        self.rating_lbl = ctk.CTkLabel(rating_f, text=star_str(0), text_color=T["accent"])
        self.rating_lbl.pack(side="right")
        
        self.rating_var = ctk.IntVar(value=0)
        slider = ctk.CTkSlider(bot_f, from_=0, to=10, number_of_steps=10, variable=self.rating_var, command=self._update_rating)
        slider.pack(fill="x", pady=(0, 15))
        
        btn_f = ctk.CTkFrame(self, fg_color="transparent")
        btn_f.pack(fill="x", padx=20, pady=(0, 20))
        ctk.CTkButton(btn_f, text="Cancel", fg_color=T["surface2"], hover_color=T["border"], command=lambda: self.after(10, self.destroy), width=100).pack(side="left")
        self.add_btn = ctk.CTkButton(btn_f, text="Add to Library", fg_color=T["accent"], hover_color=T["accent_h"], state="disabled", command=self._do_add)
        self.add_btn.pack(side="right")

    def _update_rating(self, val):
        self.rating_lbl.configure(text=star_str(int(val)))

    def _search(self):
        q = self.entry.get().strip()
        if not q: return
        for w in self.results_f.winfo_children(): w.destroy()
        ctk.CTkLabel(self.results_f, text="Searching...", text_color=T["text2"]).pack(pady=20)
        
        def fetch():
            res = self.api.search_games(q, count=6)
            if self.winfo_exists():
                self.after(0, lambda: self._render_results(res))
        threading.Thread(target=fetch, daemon=True).start()

    def _render_results(self, results):
        for w in self.results_f.winfo_children(): w.destroy()
        self._selected = None
        self.add_btn.configure(state="disabled")
        
        if not results:
            ctk.CTkLabel(self.results_f, text="No results found.", text_color=T["text2"]).pack(pady=20)
            return
            
        self.rows = []
        for g in results:
            f = ctk.CTkFrame(self.results_f, fg_color=T["surface"], corner_radius=4, cursor="hand2")
            f.pack(fill="x", pady=2)
            year = g.get("released", "")[:4]
            text = f"{g['name']} ({year})" if year else g["name"]
            ctk.CTkLabel(f, text=text, text_color=T["text"]).pack(side="left", padx=10, pady=5)
            f.bind("<Button-1>", lambda e, game=g, frame=f: self._select(game, frame))
            for child in f.winfo_children():
                child.bind("<Button-1>", lambda e, game=g, frame=f: self._select(game, frame))
            self.rows.append(f)

    def _select(self, game, frame):
        self._selected = game
        for f in self.rows:
            f.configure(border_width=0)
        frame.configure(border_width=1, border_color=T["accent"])
        self.add_btn.configure(state="normal")

    def _do_add(self):
        s = self.status_var.get()
        r = int(self.rating_var.get())
        sel = self._selected
        
        def _execute():
            self.destroy()
            if sel:
                self.on_add(sel, s, r)
                
        self.after(10, _execute)

# --- App ---
class MooDexApp(ctk.CTk):
    def __init__(self):
        super().__init__()
        self.title("MooDex")
        self.geometry("1120x760")
        self.minsize(680, 520)
        self.configure(fg_color=T["bg"])
        
        icon_path = os.path.join(os.path.dirname(__file__), "Icon Logo", "icon.ico")
        if os.path.exists(icon_path):
            try:
                self.iconbitmap(icon_path)
            except Exception:
                pass
            
        self.api = RAWGApiClient()
        self.dm = DataManager()
        
        self.grid_rowconfigure(0, weight=1)
        self.grid_columnconfigure(1, weight=1)
        
        self._view = "library"
        self._filter = "All"
        self._sort = "Recently Added"
        self._lib_search = ""
        self._disc_tab = "popular"
        self._lib_view_mode = "tile"  # "tile" or "list"
        
        self._cols = 0
        self._rid = None
        self._rf = None # For discover thread safety
        self._discover_request_id = 0
        
        self._build_ui()
        self.cards_frame = ctk.CTkFrame(self._scroll, fg_color="transparent")
        self.cards_frame.pack(fill="both", expand=True, padx=20, pady=10)
        self.list_frame = ctk.CTkFrame(self._scroll, fg_color="transparent")
        self.card_widgets = {}
        self.list_widgets = {}
        self.stats_bar = StatsBar(self._scroll, self.dm)
        self.stats_bar.pack(fill="x", padx=16, pady=(12,8))
        self._update_np()
        self.after(120, self._show_library)

    def _calc(self):
        ww = self._scroll.winfo_width()

        if ww <= 1:
            ww = self.winfo_width() - 250

        GAP = 20
        SIDE_PADDING = 40   # left + right padding

        # exact 5 columns like your screenshot
        cols = 5

        usable = ww - SIDE_PADDING
        card_w = (usable - (GAP * (cols - 1))) // cols

        return cols, card_w

    def _on_resize(self, e):
        if e.widget != self._main:
            return

        if hasattr(self, "_resize_job"):
            self.after_cancel(self._resize_job)

        self._resize_job = self.after(150, self._regrid)

    def _regrid(self):
        if self._view == "library":
            self._show_library()
        elif self._view == "discover":
            self._show_discover()

    def _build_ui(self):
        # Sidebar
        self._sidebar = ctk.CTkFrame(self, width=230, fg_color=T["sidebar"], corner_radius=0)
        self._sidebar.grid(row=0, column=0, sticky="nsew")
        self._sidebar.grid_propagate(False)
        
        top = ctk.CTkFrame(self._sidebar, fg_color="transparent")
        top.pack(fill="x", padx=10, pady=20)
        
        logo_path = os.path.join(os.path.dirname(__file__), "Icon Logo", "logo with name.png")
        if os.path.exists(logo_path):
            from PIL import Image
            logo_img = ctk.CTkImage(light_image=Image.open(logo_path), dark_image=Image.open(logo_path), size=(180, 90))
            self.lbl_brand = ctk.CTkLabel(top, text="", image=logo_img)
        else:
            self.lbl_brand = ctk.CTkLabel(top, text="MooDex", font=FONTS["h1"], text_color=T["accent"])
        self.lbl_brand.pack(side="left", padx=10)
        
        self.nav_btns = {}
        for icon, txt, view in [("📚", "My Library", "library"), ("🔍", "Discover", "discover"), ("📊", "Stats", "stats")]:
            b = ctk.CTkButton(self._sidebar, text=f"{icon}  {txt}", 
                              fg_color="transparent", text_color=T["text"], font=FONTS["bold"], anchor="w",
                              command=lambda v=view: self._set_view(v))
            b.pack(fill="x", padx=10, pady=5)
            self.nav_btns[view] = b
            
        self._update_nav_style()
        
        self.btn_add = ctk.CTkButton(self._sidebar, text="+ Add Game", fg_color=T["accent"], hover_color=T["accent_h"], font=FONTS["bold"], command=self._open_add)
        self.btn_add.pack(fill="x", padx=10, pady=20)
        
        ctk.CTkFrame(self._sidebar, fg_color="transparent").pack(fill="both", expand=True) # Spacer
        
        self.np_frame = ctk.CTkFrame(self._sidebar, fg_color=T["surface2"], corner_radius=8)
        self.np_frame.pack(fill="x", padx=10, pady=20)
        ctk.CTkLabel(self.np_frame, text="NOW PLAYING", text_color=T["accent2"], font=FONTS["small"]).pack(pady=(10,0))
        self.np_lbl = ctk.CTkLabel(self.np_frame, text="", text_color=T["text"], font=FONTS["bold"], wraplength=180)
        self.np_lbl.pack(pady=(0,10), padx=10)
        
        # Main Area
        self._main = ctk.CTkFrame(self, fg_color="transparent", corner_radius=0)
        self._main.grid(row=0, column=1, sticky="nsew")
        self._main.grid_rowconfigure(2, weight=1)
        self._main.grid_columnconfigure(0, weight=1)
        self._main.bind("<Configure>", self._on_resize)
        
        # Header (row 0)
        self._header = ctk.CTkFrame(self._main, height=56, fg_color="transparent")
        self._header.grid(row=0, column=0, sticky="ew", padx=20)
        self._header.pack_propagate(False)
        
        self.lbl_title = ctk.CTkLabel(self._header, text="My Library", font=FONTS["h1"], text_color=T["text"])
        self.lbl_title.pack(side="left", pady=10)
        
        self.search_var = ctk.StringVar()
        self.search_entry = ctk.CTkEntry(self._header, width=210, height=32, placeholder_text="Search library...", textvariable=self.search_var, fg_color=T["surface"])
        self.search_entry.pack(side="right", pady=12)
        self.search_entry.bind("<KeyRelease>", self._on_lib_search)
        self._lib_search_rid = None
        
        # Filter Bar (row 1)
        self._bar = ctk.CTkFrame(self._main, height=44, fg_color="transparent")
        self._bar.grid(row=1, column=0, sticky="ew", padx=20)
        self._bar.pack_propagate(False)
        
        self.pill_f = ctk.CTkFrame(self._bar, fg_color="transparent")
        self.pill_f.pack(side="left", fill="y")
        self._pills = []
        for opt in FILTER_OPTIONS:
            color = STATUS.get(opt, {}).get("c", T["text2"])
            btn = ctk.CTkButton(self.pill_f, text=opt, width=0, height=28, fg_color="transparent", text_color=color, hover_color=T["surface"], border_width=1, border_color=T["bg"], corner_radius=14, command=lambda o=opt: self._set_filter(o))
            btn.pack(side="left", padx=4, pady=8)
            self._pills.append((opt, btn))
        self._update_pills()
        
        self.sort_menu = ctk.CTkOptionMenu(self._bar, values=SORT_OPTIONS, width=160, fg_color=T["surface"], button_color=T["surface2"], command=self._set_sort)
        self.sort_menu.pack(side="right", pady=8)
        
        # View toggle buttons
        self.view_toggle_f = ctk.CTkFrame(self._bar, fg_color="transparent")
        self.view_toggle_f.pack(side="right", padx=(0, 10), pady=8)
        self.btn_tile = ctk.CTkButton(self.view_toggle_f, text="▦", width=32, height=28, font=FONTS["bold"],
                                       fg_color=T["accent"], hover_color=T["accent_h"], corner_radius=6,
                                       command=lambda: self._set_lib_view("tile"))
        self.btn_tile.pack(side="left", padx=(0, 2))
        self.btn_list = ctk.CTkButton(self.view_toggle_f, text="☰", width=32, height=28, font=FONTS["bold"],
                                       fg_color=T["surface2"], hover_color=T["accent_h"], corner_radius=6,
                                       command=lambda: self._set_lib_view("list"))
        self.btn_list.pack(side="left")
        
        # Scroll Area (row 2)
        self._scroll = ctk.CTkScrollableFrame(self._main, fg_color="transparent")
        self._scroll.grid(row=2, column=0, sticky="nsew", padx=10, pady=10)
        self._scroll._parent_canvas.configure(yscrollincrement=10)

    def _set_view(self, view):
        self._view = view
        self._rf = None
        self._update_nav_style()
        self.search_entry.pack(side="right", pady=12) if view == "library" else self.search_entry.pack_forget()
        self._bar.grid(row=1, column=0, sticky="ew", padx=20) if view == "library" else self._bar.grid_forget()
        if view == "library":
            self.cards_frame.pack(fill="both", expand=True, padx=20, pady=10)
        else:
            self.cards_frame.pack_forget()
        
        titles = {"library": "My Library", "discover": "Discover", "stats": "Statistics"}
        self.lbl_title.configure(text=titles[view])

        # Destroy only non-persistent children
        for w in self._scroll.winfo_children():
            if w is not self.cards_frame and w is not self.list_frame and w is not self.stats_bar:
                w.destroy()
        
        self.stats_bar.pack_forget()
        self.list_frame.pack_forget()
        self.cards_frame.pack_forget()
        
        if view == "library":
            self.stats_bar.pack(fill="x", padx=16, pady=(12,8))
            self._show_library()
        elif view == "discover":
            self._show_discover()
        elif view == "stats":
            self._show_stats()

    def _update_nav_style(self):
        for v, b in self.nav_btns.items():
            if v == self._view:
                b.configure(fg_color=T["surface2"], text_color=T["accent"])
            else:
                b.configure(fg_color="transparent", text_color=T["text"])

    def _update_np(self):
        np = self.dm.data.get("now_playing")
        if np:
            self.np_frame.pack(fill="x", padx=10, pady=20)
            self.np_lbl.configure(text=np)
        else:
            self.np_frame.pack_forget()

    # --- Library ---
    def _set_filter(self, opt):
        self._filter = opt
        self._update_pills()
        self._show_library()

    def _update_pills(self):
        for opt, btn in self._pills:
            if opt == self._filter:
                color = STATUS.get(opt, {}).get("c", T["accent"])
                bg = STATUS.get(opt, {}).get("bg", T["surface2"])
                btn.configure(border_color=color, fg_color=bg)
            else:
                btn.configure(border_color=T["bg"], fg_color="transparent")

    def _set_sort(self, opt):
        self._sort = opt
        self._show_library()

    def _on_lib_search(self, e):
        if hasattr(self, "_search_job"):
            self.after_cancel(self._search_job)

        self._search_job = self.after(250, self._do_lib_search)

    def _do_lib_search(self):
        self._lib_search = self.search_var.get().strip()
        self._show_library()

    def _set_lib_view(self, mode):
        self._lib_view_mode = mode
        if mode == "tile":
            self.btn_tile.configure(fg_color=T["accent"])
            self.btn_list.configure(fg_color=T["surface2"])
        else:
            self.btn_tile.configure(fg_color=T["surface2"])
            self.btn_list.configure(fg_color=T["accent"])
        # Clear both views
        for w in self.card_widgets.values(): w.destroy()
        self.card_widgets.clear()
        for w in self.list_widgets.values(): w.destroy()
        self.list_widgets.clear()
        self._show_library()

    def _show_library(self):
        if self._view != "library": return

        games = self.dm.get_filtered_sorted(self._filter, self._sort, self._lib_search)

        if self._lib_view_mode == "tile":
            self._show_library_tiles(games)
        else:
            self._show_library_list(games)

    def _show_library_tiles(self, games):
        self.list_frame.pack_forget()
        self.cards_frame.pack(fill="both", expand=True, padx=20, pady=10)

        cols, cw = self._calc()

        if cols != getattr(self, "_last_cols", None):
            for widget in self.cards_frame.winfo_children():
                widget.grid_forget()
            self._last_cols = cols

        self._cols = cols

        existing = set(self.card_widgets.keys())
        current = set(g["name"] for g in games)

        # REMOVE only what's needed
        for name in existing - current:
            self.card_widgets[name].destroy()
            del self.card_widgets[name]

        # CREATE or UPDATE
        for i, g in enumerate(games):
            name = g["name"]

            if name not in self.card_widgets:
                card = GameCard(self.cards_frame, g, cw, mode="library", on_action=self._card_action)
                self.card_widgets[name] = card
            else:
                card = self.card_widgets[name]

            card.grid(
                row=i // cols,
                column=i % cols,
                padx=(10, 10),
                pady=(12, 12),
                sticky="n"
            )

        # center the whole grid
        total_width = (cw * cols) + (20 * (cols - 1))
        container_width = self._scroll.winfo_width()

        left_space = max(0, (container_width - total_width) // 2)

        self.cards_frame.pack_configure(padx=(left_space, left_space))

        for i in range(cols):
            self.cards_frame.grid_columnconfigure(i, weight=1)

    def _show_library_list(self, games):
        # Hide grid, show list
        self.cards_frame.pack_forget()
        self.list_frame.pack(fill="both", expand=True, padx=20, pady=10)

        existing = set(self.list_widgets.keys())
        current = set(g["name"] for g in games)

        # REMOVE deleted rows
        for name in existing - current:
            self.list_widgets[name].destroy()
            del self.list_widgets[name]

        # Unpack all to reorder
        for w in self.list_frame.pack_slaves():
            w.pack_forget()

        # Header row
        if not hasattr(self, '_list_header') or not self._list_header.winfo_exists():
            self._list_header = ctk.CTkFrame(self.list_frame, fg_color=T["surface"], height=36, corner_radius=6)
            hdr_cols = [("Game", 0.35), ("Status", 0.15), ("Rating", 0.12), ("Genre", 0.15), ("Released", 0.10), ("Actions", 0.13)]
            for txt, w_pct in hdr_cols:
                ctk.CTkLabel(self._list_header, text=txt, font=FONTS["bold"], text_color=T["text2"],
                             anchor="w").pack(side="left", padx=10, expand=True, fill="x")
        self._list_header.pack(fill="x", pady=(0, 4))

        for g in games:
            name = g["name"]
            if name not in self.list_widgets:
                row = self._create_list_row(g)
                self.list_widgets[name] = row
            self.list_widgets[name].pack(fill="x", pady=2)

    def _create_list_row(self, game):
        row = ctk.CTkFrame(self.list_frame, fg_color=T["card"], height=52, corner_radius=6,
                           border_width=1, border_color=T["border"])
        row.pack_propagate(False)

        # Thumbnail
        thumb_w, thumb_h = 70, 40
        thumb_lbl = ctk.CTkLabel(row, text="", width=thumb_w, height=thumb_h,
                                  image=make_placeholder(thumb_w, thumb_h))
        thumb_lbl.pack(side="left", padx=(10, 8))

        # Load thumbnail async
        url = game.get("image_url") or game.get("background_image")
        if url:
            cache_key = f"{url}_{thumb_w}x{thumb_h}"
            if cache_key in GameCard._img_cache:
                thumb_lbl.configure(image=GameCard._img_cache[cache_key], text="")
                thumb_lbl.image = GameCard._img_cache[cache_key]
            else:
                def _load_thumb(u=url, lbl=thumb_lbl, tw=thumb_w, th=thumb_h, ck=cache_key):
                    try:
                        headers = {"User-Agent": "MooDeX/1.0"}
                        resp = requests.get(u, timeout=10, headers=headers)
                        resp.raise_for_status()
                        raw = Image.open(BytesIO(resp.content)).convert("RGB")
                        
                        if hasattr(Image, 'Resampling'):
                            resample_method = Image.Resampling.LANCZOS
                        else:
                            resample_method = Image.LANCZOS
                            
                        canvas = ImageOps.fit(raw, (tw, th), method=resample_method)
                        ctk_img = ctk.CTkImage(light_image=canvas, dark_image=canvas, size=(tw, th))
                        GameCard._img_cache[ck] = ctk_img
                        if lbl.winfo_exists():
                            def _apply(img=ctk_img):
                                lbl.configure(image=img, text="")
                                lbl.image = img
                            lbl.after(0, _apply)
                    except Exception:
                        pass
                _img_executor.submit(_load_thumb)

        # Name
        ctk.CTkLabel(row, text=game["name"], font=FONTS["bold"], text_color=T["text"],
                     anchor="w", width=180).pack(side="left", padx=8, expand=True, fill="x")

        # Status badge
        stat = STATUS.get(game["status"], STATUS["Stopped"])
        stat_f = ctk.CTkFrame(row, fg_color=stat["bg"], corner_radius=4, width=90)
        stat_f.pack(side="left", padx=8)
        stat_f.pack_propagate(False)
        ctk.CTkLabel(stat_f, text=f"{stat['i']} {game['status']}", text_color=stat["c"],
                     font=FONTS["small"], width=90, height=24).pack(padx=4, pady=2)

        # Rating
        rating = game.get("rating", 0)
        r_text = f"★ {rating}/10" if rating else "—"
        r_color = T["accent"] if rating else T["text3"]
        ctk.CTkLabel(row, text=r_text, font=FONTS["small"], text_color=r_color,
                     width=60, anchor="center").pack(side="left", padx=8)

        # Genre
        genre = game.get("genres", [""])[0] if game.get("genres") else "—"
        ctk.CTkLabel(row, text=genre, font=FONTS["small"], text_color=T["text2"],
                     width=80, anchor="w").pack(side="left", padx=8)

        # Release year
        year = game.get("released", "—")[:4] if game.get("released") else "—"
        ctk.CTkLabel(row, text=year, font=FONTS["small"], text_color=T["text2"],
                     width=50, anchor="center").pack(side="left", padx=8)

        # Action buttons
        btn_f = ctk.CTkFrame(row, fg_color="transparent")
        btn_f.pack(side="right", padx=10)
        ctk.CTkButton(btn_f, text="▶", width=28, height=24, fg_color="transparent",
                      hover_color=T["green"], font=FONTS["small"],
                      command=lambda: self._card_action("play", game)).pack(side="left", padx=2)
        ctk.CTkButton(btn_f, text="✎", width=28, height=24, fg_color="transparent",
                      hover_color=T["accent"], font=FONTS["small"],
                      command=lambda: self._card_action("edit", game)).pack(side="left", padx=2)
        ctk.CTkButton(btn_f, text="✕", width=28, height=24, fg_color="transparent",
                      hover_color=T["red"], font=FONTS["small"],
                      command=lambda: self._card_action("delete", game)).pack(side="left", padx=2)

        # Hover effect
        def _enter(e): row.configure(fg_color=T["card_h"], border_color=T["border2"])
        def _leave(e): row.configure(fg_color=T["card"], border_color=T["border"])
        row.bind("<Enter>", _enter)
        row.bind("<Leave>", _leave)
        for w in row.winfo_children():
            w.bind("<Enter>", _enter)
            w.bind("<Leave>", _leave)

        return row

    def _evict_cache(self, name):
        self._last_games = None
        if name in self.card_widgets:
            self.card_widgets[name].destroy()
            del self.card_widgets[name]
        if name in self.list_widgets:
            self.list_widgets[name].destroy()
            del self.list_widgets[name]

    def _card_action(self, action, game):
        name = game["name"]
        if action == "play":
            self.dm.set_playing(name)
            self._evict_cache(name)
            self._update_np()
            self._show_library()
            Toast(self, f"Now playing {name}", "success")
        elif action == "delete":
            self.dm.remove(name)
            self._evict_cache(name)
            self._update_np()
            self._show_library()
            Toast(self, f"Removed {name}", "info")
        elif action == "fav":
            self.dm.toggle_favorite(name)
            self._evict_cache(name)
            self._show_library()
        elif action == "edit":
            EditDialog(self, game, lambda s, r, n: self._save_edit(name, s, r, n))

    def _save_edit(self, name, status, rating, notes):
        self.dm.update(name, status, rating, notes)
        self._evict_cache(name)
        self._update_np()
        self._show_library()
        Toast(self, "Game updated", "success")

    # --- Add / Discover ---
    def _open_add(self):
        AddDialog(self, self.api, self._add_game)

    def _add_game(self, game, status, rating):
        ok = self.dm.add(
            name=game["name"], status=status, rating=rating,
            released=game.get("released", "Unknown"),
            image_url=game.get("background_image", ""),
            genres=game.get("genres", []),
            platforms=game.get("platforms", []),
            playtime=game.get("playtime", 0)
        )
        if ok:
            Toast(self, f"Added {game['name']}", "success")
            self._update_np()
            # Removed self._set_view("library") to stay on Discover
        else:
            Toast(self, f"{game['name']} is already in your library", "warn")

    def _show_discover(self):
        if self._view != "discover": return
        for w in self._scroll.winfo_children():
            if w is not self.cards_frame and w is not self.list_frame and w is not self.stats_bar:
                w.destroy()
        
        top_f = ctk.CTkFrame(self._scroll, fg_color="transparent")
        top_f.pack(fill="x", padx=16, pady=10)
        
        self.d_search_var = ctk.StringVar()
        s_entry = ctk.CTkEntry(top_f, placeholder_text="Search games...", textvariable=self.d_search_var, height=40, font=FONTS["body"])
        s_entry.pack(side="left", fill="x", expand=True, padx=(0, 10))
        s_entry.bind("<KeyRelease>", self._on_disc_search)
        ctk.CTkButton(top_f, text="Search", height=40, fg_color=T["accent"], command=self._do_disc_search).pack(side="right")
        self._disc_search_rid = None
        
        # Beautiful segmented control for tabs
        tab_f = ctk.CTkFrame(self._scroll, fg_color="transparent")
        tab_f.pack(fill="x", padx=16, pady=(10, 20))
        
        tab_bg = ctk.CTkFrame(tab_f, fg_color=T["surface"], corner_radius=20)
        tab_bg.pack(side="left")
        
        for t, icon, lbl, acc in [("popular", "🔥", "Popular", T["red"]), ("upcoming", "📅", "Upcoming", T["amber"]), ("trending", "⚡", "Trending", T["purple"])]:
            is_active = (self._disc_tab == t)
            bg_color = acc if is_active else "transparent"
            text_col = "#ffffff" if is_active else T["text2"]
            hover_col = acc if is_active else T["surface2"]
            
            b = ctk.CTkButton(
                tab_bg, text=f"{icon} {lbl}", font=("Segoe UI", 14, "bold" if is_active else "normal"),
                fg_color=bg_color, text_color=text_col, hover_color=hover_col,
                corner_radius=16, height=36, width=120,
                command=lambda x=t: self._set_disc_tab(x)
            )
            b.pack(side="left", padx=4, pady=4)
            
        self.d_lbl = ctk.CTkLabel(self._scroll, text="", font=("Segoe UI", 24, "bold"), text_color=T["text"], anchor="w")
        self.d_lbl.pack(fill="x", padx=16, pady=(10,0))
        
        self._rf = ctk.CTkFrame(self._scroll, fg_color="transparent")
        self._rf.pack(anchor="nw", padx=20, pady=10)
        
        self.update_idletasks()
        self._rf.configure(width=self._scroll.winfo_width() - 40)
        try:
            self._rf.grid_anchor("n")
        except AttributeError:
            pass
        
        self._load_discover()

    def _set_disc_tab(self, tab):
        self._disc_tab = tab
        self.d_search_var.set("")
        self._show_discover()

    def _on_disc_search(self, e):
        if self._disc_search_rid: self.after_cancel(self._disc_search_rid)
        self._disc_search_rid = self.after(600, self._do_disc_search)

    def _do_disc_search(self):
        q = self.d_search_var.get().strip()
        if not q:
            self._load_discover()
            return
            
        self.d_lbl.configure(text=f"Search Results for '{q}'")
        for w in self._rf.winfo_children(): w.destroy()
        ctk.CTkLabel(self._rf, text="Searching...", text_color=T["text2"]).pack(pady=40)
        
        self._discover_request_id += 1
        req_id = self._discover_request_id
        
        def fetch():
            res = self.api.search_games(q, count=12)
            if req_id != self._discover_request_id:
                return  # ignore outdated response
            self.after(0, lambda: self._render_discover(res))
        threading.Thread(target=fetch, daemon=True).start()

    def _load_discover(self):
        for w in self._rf.winfo_children(): w.destroy()
        ctk.CTkLabel(self._rf, text="Loading...", text_color=T["text2"]).pack(pady=40)
        
        titles = {"popular": "🔥 Top Rated Games", "upcoming": "📅 Upcoming Releases", "trending": "⚡ Trending Now"}
        self.d_lbl.configure(text=titles[self._disc_tab])
        
        self._discover_request_id += 1
        req_id = self._discover_request_id
        
        def fetch():
            if self._disc_tab == "popular": res = self.api.get_popular_games()
            elif self._disc_tab == "upcoming": res = self.api.get_upcoming_games()
            else: res = self.api.get_trending_games()
            if req_id != self._discover_request_id:
                return  # ignore outdated response
            self.after(0, lambda: self._render_discover(res))
        threading.Thread(target=fetch, daemon=True).start()

    def _render_discover(self, games):
        if not self._rf or not self._rf.winfo_exists(): return
        
        for widget in self._rf.winfo_children():
            widget.destroy()
        
        if not games:
            ctk.CTkLabel(self._rf, text="No games found.", text_color=T["text2"]).pack(pady=40)
            return
            
        cols, cw = self._calc()
        # Reset old grid columns to stop scroll glitch
        for i in range(self._rf.grid_size()[0]):
            self._rf.grid_columnconfigure(i, weight=0, uniform="")
        for i in range(cols):
            self._rf.grid_columnconfigure(i, weight=1, uniform="col")
            
        for i, g in enumerate(games):
            card = GameCard(self._rf, g, cw, mode="discover", on_action=self._disc_action)
            card.grid(row=i//cols, column=i%cols, padx=10, pady=12, sticky="nsew")

    def _disc_action(self, action, game):
        if action == "add":
            self._add_game(game, "Like to Play", 0)

    # --- Stats View ---
    def _show_stats(self):
        if self._view != "stats": return
        for w in self._scroll.winfo_children():
            if w is not self.cards_frame and w is not self.list_frame and w is not self.stats_bar:
                w.destroy()
        
        stats = self.dm.stats()
        games = self.dm.data.get("games", [])
        
        genre_counts = {}
        plat_counts = {}
        for g in games:
            for genre in g.get("genres", []):
                if genre: genre_counts[genre] = genre_counts.get(genre, 0) + 1
            for plat in g.get("platforms", []):
                if plat: plat_counts[plat] = plat_counts.get(plat, 0) + 1
                
        fav_genre = max(genre_counts, key=genre_counts.get) if genre_counts else "None"
        total_playtime = sum(g.get("playtime", 0) for g in games)
        
        # --- Hero Section ---
        hero = ctk.CTkFrame(self._scroll, fg_color=T["surface"], corner_radius=16)
        hero.pack(fill="x", padx=16, pady=(10, 20))
        
        hl = ctk.CTkFrame(hero, fg_color="transparent")
        hl.pack(side="left", padx=30, pady=30)
        
        logo_path = os.path.join(os.path.dirname(__file__), "Icon Logo", "logo with name.png")
        if os.path.exists(logo_path):
            from PIL import Image
            logo_img = ctk.CTkImage(light_image=Image.open(logo_path), dark_image=Image.open(logo_path), size=(180, 90))
            ctk.CTkLabel(hl, text="", image=logo_img).pack(anchor="w", pady=(0, 10))
            
        ctk.CTkLabel(hl, text="Your Gaming Journey", font=("Segoe UI", 32, "bold"), text_color=T["text"]).pack(anchor="w")
        ctk.CTkLabel(hl, text="A comprehensive breakdown of your library and habits.", font=FONTS["body"], text_color=T["text2"]).pack(anchor="w", pady=(5,0))
        
        hr = ctk.CTkFrame(hero, fg_color="transparent")
        hr.pack(side="right", padx=30, pady=30)
        ctk.CTkLabel(hr, text="Total Completion", font=FONTS["small"], text_color=T["text2"]).pack(anchor="e")
        
        total = stats["total"] or 1
        comp = stats["by_status"].get("Completed", 0)
        pct = int((comp / total) * 100) if stats["total"] else 0
        
        comp_frame = ctk.CTkFrame(hr, fg_color="transparent")
        comp_frame.pack(anchor="e")
        ctk.CTkLabel(comp_frame, text=f"{pct}%", font=("Segoe UI", 48, "bold"), text_color=T["accent"]).pack(side="left")
        
        # --- Big Stat Cards Grid ---
        cards_f = ctk.CTkFrame(self._scroll, fg_color="transparent")
        cards_f.pack(fill="x", padx=10)
        
        items = [
            ("🎮", str(stats["total"]), "Total Games", T["accent2"], "#083344"),
            ("▶", str(stats["by_status"].get("Playing", 0)), "Playing Now", T["green"], "#064e3b"),
            ("⏱", f"{total_playtime}h", "Playtime", T["purple"], "#4c1d95"),
            ("⭐", f"{stats['avg_rating']}/10", "Avg Rating", T["amber"], "#78350f"),
            ("🔥", fav_genre, "Top Genre", T["red"], "#7f1d1d")
        ]
        
        for i, (icon, val, lbl, fg_c, bg_c) in enumerate(items):
            cards_f.grid_columnconfigure(i, weight=1)
            c = ctk.CTkFrame(cards_f, fg_color=T["surface"], corner_radius=16)
            c.grid(row=0, column=i, padx=6, sticky="ew")
            
            icon_bg = ctk.CTkFrame(c, fg_color=bg_c, width=50, height=50, corner_radius=12)
            icon_bg.pack(anchor="w", padx=20, pady=(20, 10))
            icon_bg.pack_propagate(False)
            ctk.CTkLabel(icon_bg, text=icon, font=("Segoe UI", 24), text_color=fg_c).place(relx=0.5, rely=0.5, anchor="center")
            
            ctk.CTkLabel(c, text=val, font=("Segoe UI", 28, "bold"), text_color=T["text"]).pack(anchor="w", padx=20)
            ctk.CTkLabel(c, text=lbl, font=FONTS["body"], text_color=T["text2"]).pack(anchor="w", padx=20, pady=(0, 20))
            
        # --- Breakdowns (Bottom) ---
        bot_f = ctk.CTkFrame(self._scroll, fg_color="transparent")
        bot_f.pack(fill="both", expand=True, padx=10, pady=20)
        bot_f.grid_columnconfigure(0, weight=1)
        bot_f.grid_columnconfigure(1, weight=1)
        bot_f.grid_columnconfigure(2, weight=1)
        
        # Left: Status Breakdown
        stat_f = ctk.CTkFrame(bot_f, fg_color=T["surface"], corner_radius=16)
        stat_f.grid(row=0, column=0, padx=6, sticky="nsew")
        ctk.CTkLabel(stat_f, text="Library Status", font=("Segoe UI", 20, "bold"), text_color=T["text"]).pack(anchor="w", padx=24, pady=(24, 10))
        
        for st, config in STATUS.items():
            count = stats["by_status"].get(st, 0)
            p = count / total
            
            row = ctk.CTkFrame(stat_f, fg_color="transparent")
            row.pack(fill="x", padx=24, pady=10)
            
            top = ctk.CTkFrame(row, fg_color="transparent")
            top.pack(fill="x")
            
            lbl_f = ctk.CTkFrame(top, fg_color="transparent")
            lbl_f.pack(side="left")
            ctk.CTkLabel(lbl_f, text=config["i"], font=FONTS["bold"], text_color=config["c"], width=20).pack(side="left")
            ctk.CTkLabel(lbl_f, text=st, font=FONTS["bold"], text_color=T["text"]).pack(side="left", padx=5)
            
            ctk.CTkLabel(top, text=str(count), font=FONTS["bold"], text_color=T["text2"]).pack(side="right")
            
            bar_bg = ctk.CTkFrame(row, height=8, fg_color=T["border"], corner_radius=4)
            bar_bg.pack(fill="x", pady=(8,0))
            if p > 0:
                bar_fg = ctk.CTkFrame(bar_bg, height=8, fg_color=config["c"], corner_radius=4)
                bar_fg.place(relwidth=p, relheight=1)

        # Middle: Top Platforms
        plat_f = ctk.CTkFrame(bot_f, fg_color=T["surface"], corner_radius=16)
        plat_f.grid(row=0, column=1, padx=6, sticky="nsew")
        ctk.CTkLabel(plat_f, text="Top Platforms", font=("Segoe UI", 20, "bold"), text_color=T["text"]).pack(anchor="w", padx=24, pady=(24, 10))
        
        sorted_plats = sorted(plat_counts.items(), key=lambda x: x[1], reverse=True)
        for p_name, count in sorted_plats[:5]:
            p_pct = count / total
            if p_pct > 1.0: p_pct = 1.0
            
            row = ctk.CTkFrame(plat_f, fg_color="transparent")
            row.pack(fill="x", padx=24, pady=10)
            
            top = ctk.CTkFrame(row, fg_color="transparent")
            top.pack(fill="x")
            
            ctk.CTkLabel(top, text=p_name, font=FONTS["bold"], text_color=T["text"]).pack(side="left")
            ctk.CTkLabel(top, text=str(count), font=FONTS["bold"], text_color=T["text2"]).pack(side="right")
            
            bar_bg = ctk.CTkFrame(row, height=8, fg_color=T["border"], corner_radius=4)
            bar_bg.pack(fill="x", pady=(8,0))
            if p_pct > 0:
                bar_fg = ctk.CTkFrame(bar_bg, height=8, fg_color=T["accent2"], corner_radius=4)
                bar_fg.place(relwidth=p_pct, relheight=1)
                
        if not sorted_plats:
            ctk.CTkLabel(plat_f, text="No platforms found.", font=FONTS["body"], text_color=T["text2"]).pack(pady=40)

        # Right: Hall of Fame
        top_f = ctk.CTkFrame(bot_f, fg_color=T["surface"], corner_radius=16)
        top_f.grid(row=0, column=2, padx=6, sticky="nsew")
        ctk.CTkLabel(top_f, text="Hall of Fame", font=("Segoe UI", 20, "bold"), text_color=T["text"]).pack(anchor="w", padx=24, pady=(24, 10))
        
        rated_games = [g for g in games if g.get("rating", 0) > 0]
        rated_games.sort(key=lambda x: x["rating"], reverse=True)
        
        for i, g in enumerate(rated_games[:5]):
            row = ctk.CTkFrame(top_f, fg_color=T["surface2"], corner_radius=8)
            row.pack(fill="x", padx=24, pady=6)
            
            rank_bg = T["accent"] if i == 0 else (T["amber"] if i == 1 else (T["text3"] if i == 2 else T["border"]))
            rank_fg = "#ffffff" if i < 3 else T["text2"]
            rank_f = ctk.CTkFrame(row, fg_color=rank_bg, width=28, height=28, corner_radius=14)
            rank_f.pack(side="left", padx=10, pady=10)
            rank_f.pack_propagate(False)
            ctk.CTkLabel(rank_f, text=str(i+1), font=("Segoe UI", 12, "bold"), text_color=rank_fg).place(relx=0.5, rely=0.5, anchor="center")
            
            ctk.CTkLabel(row, text=g["name"], font=FONTS["bold"], text_color=T["text"]).pack(side="left", padx=5)
            
            r_frame = ctk.CTkFrame(row, fg_color="#451a03", corner_radius=10)
            r_frame.pack(side="right", padx=10)
            ctk.CTkLabel(r_frame, text=f"★ {g['rating']}", font=("Segoe UI", 12, "bold"), text_color=T["amber"]).pack(padx=8, pady=2)
            
        if not rated_games:
            ctk.CTkLabel(top_f, text="No rated games yet. Play and rate some games!", font=FONTS["body"], text_color=T["text2"]).pack(pady=40)

if __name__ == "__main__":
    ctk.set_appearance_mode("dark")
    app = MooDexApp()
    app.mainloop()
