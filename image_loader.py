from PySide6.QtCore import QObject, Signal, QByteArray, QUrl, QRunnable, QThreadPool
from PySide6.QtNetwork import QNetworkAccessManager, QNetworkRequest, QNetworkReply
from PySide6.QtGui import QPixmap, QImage

class ImageDecoderSignals(QObject):
    finished = Signal(str, QImage)

class ImageDecoder(QRunnable):
    def __init__(self, data, url):
        super().__init__()
        self.data = data
        self.url = url
        self.signals = ImageDecoderSignals()

    def run(self):
        image = QImage()
        image.loadFromData(self.data)
        self.signals.finished.emit(self.url, image)

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
        self.cache = {}
        self.pending = {}

    def load_image(self, url, callback):
        if not url:
            return
            
        if url in self.cache:
            callback(self.cache[url])
            return

        if url in self.pending:
            self.pending[url].append(callback)
            return
            
        self.pending[url] = [callback]

        request = QNetworkRequest(QUrl(url))
        reply = self.manager.get(request)
        reply.finished.connect(lambda r=reply, u=url: self._on_finished(r, u))

    def _on_finished(self, reply, url):
        if reply.error() == QNetworkReply.NoError:
            data = reply.readAll()
            decoder = ImageDecoder(data, url)
            decoder.signals.finished.connect(self._on_decoded)
            QThreadPool.globalInstance().start(decoder)
        else:
            if url in self.pending:
                del self.pending[url]
        reply.deleteLater()

    def _on_decoded(self, url, image):
        pixmap = QPixmap.fromImage(image)
        self.cache[url] = pixmap
        if url in self.pending:
            for cb in self.pending[url]:
                cb(pixmap)
            del self.pending[url]
