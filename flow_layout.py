from PySide6.QtWidgets import QLayout, QWidgetItem, QSizePolicy
from PySide6.QtCore import Qt, QRect, QSize, QPoint

class FlowLayout(QLayout):
    def __init__(self, parent=None, margin=0, spacing=-1):
        super().__init__(parent)
        if parent is not None:
            self.setContentsMargins(margin, margin, margin, margin)
        self.setSpacing(spacing)
        self.itemList = []

    def __del__(self):
        item = self.takeAt(0)
        while item:
            item = self.takeAt(0)

    def addItem(self, item):
        self.itemList.append(item)

    def count(self):
        return len(self.itemList)

    def itemAt(self, index):
        if 0 <= index < len(self.itemList):
            return self.itemList[index]
        return None

    def takeAt(self, index):
        if 0 <= index < len(self.itemList):
            return self.itemList.pop(index)
        return None

    def expandingDirections(self):
        return Qt.Orientations(Qt.Orientation(0))

    def hasHeightForWidth(self):
        return True

    def heightForWidth(self, width):
        height = self.doLayout(QRect(0, 0, width, 0), True)
        return height

    def setGeometry(self, rect):
        super().setGeometry(rect)
        self.doLayout(rect, False)

    def sizeHint(self):
        return self.minimumSize()

    def minimumSize(self):
        size = QSize()
        for item in self.itemList:
            size = size.expandedTo(item.minimumSize())
        size += QSize(2 * self.contentsMargins().top(), 2 * self.contentsMargins().top())
        return size

    def doLayout(self, rect, testOnly):
        # Layout calculation specifically designed to auto-fill space like modern responsive grids
        x = rect.x()
        y = rect.y()
        
        spaceX = self.spacing()
        spaceY = self.spacing()
        
        # Determine total grid bounds and group elements by rows based on their actual responsive widths
        lines = []
        current_line = []
        current_line_width = 0
        lineHeight = 0
        
        for item in self.itemList:
            if item.widget() and item.widget().isHidden():
                continue
            itemWidth = item.sizeHint().width()
            itemHeight = item.sizeHint().height()

            # Time to wrap to next line?
            if current_line and (current_line_width + spaceX + itemWidth > rect.width()):
                lines.append((current_line, current_line_width, lineHeight))
                current_line = [item]
                current_line_width = itemWidth
                lineHeight = itemHeight
            else:
                current_line.append(item)
                if current_line_width == 0:
                    current_line_width = itemWidth
                else:
                    current_line_width += spaceX + itemWidth
                lineHeight = max(lineHeight, itemHeight)

        if current_line:
            lines.append((current_line, current_line_width, lineHeight))

        # Find the maximum width of the longest row to center the grid block as a whole
        # This keeps the last row strictly left-aligned under the cards above it!
        max_line_width = max([lw for _, lw, _ in lines]) if lines else 0
        grid_offset = max(0, (rect.width() - max_line_width) // 2)

        # Repaint lines centered as a solid grid block 
        for items, line_width, lh in lines:
            x_offset = rect.x() + grid_offset
            for item in items:
                if not testOnly:
                    item.setGeometry(QRect(QPoint(x_offset, y), item.sizeHint()))
                x_offset += item.sizeHint().width() + spaceX
            y += lh + spaceY

        return y - spaceY - rect.y() if lines else 0
