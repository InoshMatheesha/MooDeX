from PySide6.QtCore import QObject, Signal, QUrl, QRunnable, QThreadPool, Qt
from PySide6.QtNetwork import QNetworkAccessManager, QNetworkRequest, QNetworkReply
from PySide6.QtGui import QPixmap, QImage
from pathlib import Path
import hashlib
import os

# =========================================
# CACHE DIRECTORY
# =========================================

CACHE_DIR = Path(os.getenv("LOCALAPPDATA")) / "MooDeX" / "image_cache"
CACHE_DIR.mkdir(parents=True, exist_ok=True)

# =========================================
# IMAGE DECODER SIGNALS
# =========================================

class ImageDecoderSignals(QObject):
    finished = Signal(str, QPixmap)

# =========================================
# IMAGE DECODER THREAD
# =========================================

class ImageDecoder(QRunnable):

    def __init__(self, data, url):
        super().__init__()

        self.data = data
        self.url = url

        self.signals = ImageDecoderSignals()

    def run(self):

        image = QImage()

        image.loadFromData(self.data)

        # HUGE PERFORMANCE BOOST
        image = image.scaled(
            420,
            240,
            Qt.KeepAspectRatioByExpanding,
            Qt.FastTransformation
        )

        pixmap = QPixmap.fromImage(image)

        self.signals.finished.emit(self.url, pixmap)

# =========================================
# IMAGE LOADER
# =========================================

class ImageLoader(QObject):

    _instance = None

    @classmethod
    def get_instance(cls):

        if cls._instance is None:
            cls._instance = ImageLoader()

        return cls._instance

    def __init__(self):

        super().__init__()

        self.manager = QNetworkAccessManager(self)

        # MEMORY CACHE
        self.cache = {}

        # PENDING CALLBACKS
        self.pending = {}

        # LIMIT THREADS
        QThreadPool.globalInstance().setMaxThreadCount(4)

    # =====================================
    # CACHE PATH
    # =====================================

    def _cache_path(self, url):

        filename = hashlib.md5(url.encode()).hexdigest() + ".img"

        return CACHE_DIR / filename

    # =====================================
    # LOAD IMAGE
    # =====================================

    def load_image(self, url, callback):

        if not url:
            return

        # MEMORY CACHE
        if url in self.cache:
            callback(self.cache[url])
            return

        # DISK CACHE
        cache_file = self._cache_path(url)

        if cache_file.exists():
            if url in self.pending:
                self.pending[url].append(callback)
                return
            
            self.pending[url] = [callback]
            try:
                data = cache_file.read_bytes()
                decoder = ImageDecoder(data, url)
                decoder.signals.finished.connect(self._on_decoded)
                QThreadPool.globalInstance().start(decoder)
                return
            except Exception:
                if url in self.pending:
                    del self.pending[url]

        # ALREADY DOWNLOADING
        if url in self.pending:

            self.pending[url].append(callback)

            return

        self.pending[url] = [callback]

        request = QNetworkRequest(QUrl(url))

        reply = self.manager.get(request)

        reply.finished.connect(
            lambda r=reply, u=url: self._on_finished(r, u)
        )

    # =====================================
    # DOWNLOAD FINISHED
    # =====================================

    def _on_finished(self, reply, url):

        if reply.error() == QNetworkReply.NoError:

            data = reply.readAll()

            # SAVE RAW FILE
            cache_file = self._cache_path(url)

            try:
                with open(cache_file, "wb") as f:
                    f.write(bytes(data))
            except:
                pass

            decoder = ImageDecoder(data, url)

            decoder.signals.finished.connect(self._on_decoded)

            QThreadPool.globalInstance().start(decoder)

        else:

            if url in self.pending:
                del self.pending[url]

        reply.deleteLater()

    # =====================================
    # DECODE FINISHED
    # =====================================

    def _on_decoded(self, url, pixmap):

        # LIMIT RAM CACHE SIZE
        if len(self.cache) > 300:

            first_key = next(iter(self.cache))

            del self.cache[first_key]

        self.cache[url] = pixmap

        if url in self.pending:

            for callback in self.pending[url]:

                callback(pixmap)

            del self.pending[url]