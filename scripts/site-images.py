"""Build the website's images from the source art in assets/website/.

Run from the repo root:  python scripts/site-images.py
Writes AVIF + WebP files into site/img/. Re-run it after swapping any source image.

Sources (assets/website/):
  dashboard.webp, library.webp, discover.webp, pc-games.webp   app screenshots (dark theme)
  cover0..cover11.webp                                         game covers for the 3D tour and scan demo
  logo.png                                                     icon + white "MooDeX" wordmark
"""

from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "assets" / "website"
OUT = ROOT / "site" / "img"

SHOTS = ["dashboard", "library", "discover", "pc-games"]
SHOT_WIDTHS = [640, 1080, 1800]
SHOT_ASPECT = 1800 / 1052  # every screenshot is normalised to this so panels line up
INK = (18, 19, 26)  # light-theme text colour, used to recolour the white wordmark


def save_pair(im: Image.Image, stem: str, avif_q: int = 52, webp_q: int = 78) -> None:
    im.save(OUT / f"{stem}.avif", quality=avif_q, speed=4)
    im.save(OUT / f"{stem}.webp", quality=webp_q, method=6)


def fit_aspect(im: Image.Image) -> Image.Image:
    """Pad short (wide) screenshots by repeating the bottom row, so the app's
    sidebar and background simply extend; crop tall ones from the bottom."""
    w, h = im.size
    want_h = round(w / SHOT_ASPECT)
    if want_h == h:
        return im
    if want_h < h:
        return im.crop((0, 0, w, want_h))
    out = Image.new("RGB", (w, want_h))
    out.paste(im, (0, 0))
    last = im.crop((0, h - 1, w, h))
    for y in range(h, want_h):
        out.paste(last, (0, y))
    return out


def shots() -> None:
    for name in SHOTS:
        im = fit_aspect(Image.open(SRC / f"{name}.webp").convert("RGB"))
        for w in SHOT_WIDTHS:
            if w > im.width:
                w = im.width
            h = round(w / SHOT_ASPECT)
            save_pair(im.resize((w, h), Image.LANCZOS), f"{name}-{w}")
        print(name, im.size)


def covers() -> None:
    for i in range(12):
        im = Image.open(SRC / f"cover{i}.webp").convert("RGB").resize((320, 180), Image.LANCZOS)
        save_pair(im, f"cover{i}", avif_q=50, webp_q=76)
    print("covers: 12")


def logos() -> None:
    src = Image.open(SRC / "logo.png").convert("RGBA")
    # Light variant: low-saturation (white/grey) pixels are the wordmark -> recolour to ink.
    light = src.copy()
    px = light.load()
    for y in range(light.height):
        for x in range(light.width):
            r, g, b, a = px[x, y]
            if a and max(r, g, b) - min(r, g, b) < 40:
                px[x, y] = (*INK, a)
    for variant, im in (("dark", src), ("light", light)):
        for w in (360, 840):
            h = round(im.height * w / im.width)
            im.resize((w, h), Image.LANCZOS).save(OUT / f"logo-{variant}-{w}.webp", quality=92, method=6)
    # Icon-only mark (the "M"), cropped from the logo, for the playtime lane and the nav island.
    mark = src.crop((0, 0, src.height, src.height))
    mark.resize((96, 96), Image.LANCZOS).save(OUT / "mark-96.webp", quality=92, method=6)
    # White wordmark only (no "M"), for the always-dark nav island.
    word = src.crop((round(src.height * 1.02), 0, src.width, src.height))
    word = word.crop(word.getbbox())
    h = 44
    word.resize((round(word.width * h / word.height), h), Image.LANCZOS).save(OUT / "wordmark-white.webp", quality=92, method=6)
    print("logos:", src.size)


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    shots()
    covers()
    logos()
