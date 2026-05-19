import json
import os
import time
import requests
from PySide6.QtCore import QObject, QThread, Signal, QRunnable, QThreadPool

class RawgApi(QObject):
    def __init__(self, cache_file="api_cache.json"):
        super().__init__()
        self.cache_file = cache_file
        self.cache = self._load_cache()
        self.ttl = 6 * 3600  # 6 hours
        self.pool = QThreadPool.globalInstance()
        self._workers = []

    def _load_cache(self):
        if os.path.exists(self.cache_file):
            try:
                with open(self.cache_file, 'r', encoding='utf-8') as f:
                    return json.load(f)
            except:
                pass
        return {}

    def _save_cache(self):
        try:
            with open(self.cache_file, 'w', encoding='utf-8') as f:
                json.dump(self.cache, f)
        except:
            pass

    def _get_cached_or_fetch(self, url):
        now = time.time()
        if url in self.cache:
            entry = self.cache[url]
            if now - entry['time'] < self.ttl:
                return entry['data']
        
        try:
            response = requests.get(url, timeout=10)
            if response.status_code == 200:
                data = response.json()
                self.cache[url] = {'time': now, 'data': data}
                self._save_cache()
                return data
        except Exception as e:
            print("API Error:", e)
        return None

    def fetch_async(self, query_type, search_query="", callback=None):
        class Worker(QRunnable):
            def __init__(self, api, q_type, s_query):
                super().__init__()
                self.api = api
                self.q_type = q_type
                self.s_query = s_query
                self.signals = WorkerSignals()

            def run(self):
                API_KEY = "3e485ee2b0a54eae8eb1dcb3a93760ec"
                url = f"https://api.rawg.io/api/games?key={API_KEY}"
                
                if self.q_type == "search" and self.s_query:
                    url += f"&search={self.s_query}"
                elif self.q_type == "trending":
                    url += "&dates=2025-01-01,2026-05-05&ordering=-rating"
                elif self.q_type == "popular":
                    url += "&ordering=-added"
                elif self.q_type == "upcoming":
                    url += "&dates=2026-05-05,2028-12-31&ordering=released"
                
                data = self.api._get_cached_or_fetch(url)
                normalized = []
                if data and 'results' in data:
                    for item in data['results']:
                        normalized.append({
                            "name": item.get("name"),
                            "released": item.get("released", ""),
                            "rating": item.get("rating", 0),
                            "background_image": item.get("background_image", ""),
                            "genres": [g["name"] for g in item.get("genres", [])]
                        })
                self.signals.finished.emit(normalized)

        worker = Worker(self, query_type, search_query)
        if callback:
            worker.signals.finished.connect(callback)
        
        # Keep python reference alive until signal is processed
        self._workers.append(worker)
        worker.signals.finished.connect(lambda _: self._workers.remove(worker) if worker in self._workers else None)
        
        self.pool.start(worker)

class WorkerSignals(QObject):
    finished = Signal(list)
