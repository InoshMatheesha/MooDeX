"""
rawg_api.py — RAWG Video Games Database API Client
Provides search, discovery (popular / upcoming / trending), and caching.
"""

import json, os, time, requests
from datetime import datetime, timedelta

API_KEY    = "3e485ee2b0a54eae8eb1dcb3a93760ec"
BASE_URL   = "https://api.rawg.io/api/games"
CACHE_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "api_cache.json")
CACHE_TTL  = 6 * 3600  # 6 hours in seconds


class RAWGApiClient:
    """Thin wrapper around the RAWG API with local JSON cache."""

    def __init__(self):
        self._cache = self._load_cache()

    # ── cache helpers ────────────────────────────────────────────────
    def _load_cache(self) -> dict:
        try:
            if os.path.exists(CACHE_FILE):
                with open(CACHE_FILE, "r", encoding="utf-8") as f:
                    return json.load(f)
        except (json.JSONDecodeError, IOError):
            pass
        return {}

    def _save_cache(self):
        try:
            with open(CACHE_FILE, "w", encoding="utf-8") as f:
                json.dump(self._cache, f, ensure_ascii=False, indent=2)
        except IOError:
            pass

    def _get_cached(self, key: str):
        entry = self._cache.get(key)
        if entry and time.time() - entry.get("ts", 0) < CACHE_TTL:
            return entry.get("data")
        return None

    def _set_cached(self, key: str, data):
        self._cache[key] = {"ts": time.time(), "data": data}
        self._save_cache()

    # ── normalise a single API result into our standard dict ────────
    @staticmethod
    def _normalise(raw: dict) -> dict:
        genres = []
        for g in (raw.get("genres") or []):
            genres.append(g.get("name", ""))

        platforms = []
        for p in (raw.get("platforms") or [])[:4]:
            plat = p.get("platform") or {}
            platforms.append(plat.get("name", ""))

        released = raw.get("released") or "Unknown"
        if released in ("", None):
            released = "TBA"

        return {
            "name":             raw.get("name", "Unknown"),
            "released":         released,
            "rating":           raw.get("rating", 0.0),
            "background_image": raw.get("background_image") or "",
            "metacritic":       raw.get("metacritic") or 0,
            "genres":           genres,
            "platforms":        platforms,
            "slug":             raw.get("slug", ""),
            "playtime":         raw.get("playtime") or 0,
        }

    # ── API fetch ────────────────────────────────────────────────────
    def _fetch(self, params: dict) -> list[dict]:
        params["key"] = API_KEY
        try:
            resp = requests.get(BASE_URL, params=params, timeout=10)
            resp.raise_for_status()
            results = resp.json().get("results", [])
            return [self._normalise(r) for r in results]
        except (requests.RequestException, ValueError, KeyError):
            return []

    # ── public methods ───────────────────────────────────────────────
    def search_game(self, name: str) -> dict | None:
        """Return a single game result, cached by lowercase name."""
        key = name.strip().lower()
        if not key:
            return None
        cached = self._get_cached(key)
        if cached:
            return cached
        results = self._fetch({"search": name, "page_size": 1})
        if results:
            self._set_cached(key, results[0])
            return results[0]
        return None

    def search_games(self, name: str, count: int = 8) -> list[dict]:
        """Multi-result search — never cached (live search)."""
        if not name.strip():
            return []
        return self._fetch({"search": name, "page_size": count})

    def get_popular_games(self, count: int = 12) -> list[dict]:
        cached = self._get_cached("_popular")
        if cached:
            return cached
        data = self._fetch({
            "ordering":  "-metacritic",
            "metacritic": "80,100",
            "page_size":  count,
        })
        if data:
            self._set_cached("_popular", data)
        return data

    def get_upcoming_games(self, count: int = 12) -> list[dict]:
        cached = self._get_cached("_upcoming")
        if cached:
            return cached
        today = datetime.now().strftime("%Y-%m-%d")
        future = (datetime.now() + timedelta(days=365)).strftime("%Y-%m-%d")
        data = self._fetch({
            "dates":     f"{today},{future}",
            "ordering":  "-added",
            "page_size": count,
        })
        if data:
            self._set_cached("_upcoming", data)
        return data

    def get_trending_games(self, count: int = 12) -> list[dict]:
        cached = self._get_cached("_trending")
        if cached:
            return cached
        end = datetime.now().strftime("%Y-%m-%d")
        start = (datetime.now() - timedelta(days=30)).strftime("%Y-%m-%d")
        data = self._fetch({
            "dates":     f"{start},{end}",
            "ordering":  "-rating",
            "page_size": count,
        })
        if data:
            self._set_cached("_trending", data)
        return data

    @staticmethod
    def convert_rating_to_10(rawg_rating: float) -> int:
        """Convert RAWG 0-5 scale → 1-10 integer."""
        if rawg_rating <= 0:
            return 0
        return max(1, min(10, round(rawg_rating * 2)))
