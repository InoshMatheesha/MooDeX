# MooDeX website

The landing page at <https://getmoodex.vercel.app/>. Plain HTML, CSS and JavaScript modules: no framework, no build step, no dependencies.

```
site/
  index.html        all page content and copy
  css/site.css      theme tokens (top of file), then one block per section
  js/main.js        entry point: registers scenes, wires up the UI
  js/engine.js      the single requestAnimationFrame loop + scroll helpers
  js/scenes.js      scroll-driven scenes: hero, 3D tour, playtime race, gallery, marquee, steps
  js/ui.js          theme switch, menu, FAQ, scan demo, hover effects, live release info, easter egg
  img/              generated images (do not edit by hand, see below)
  vercel.json       headers and clean URLs
```

## Run it locally

Any static file server works. From the repo root:

```sh
python -m http.server 8000 --directory site
```

Then open <http://localhost:8000>. Opening `index.html` straight from disk will not work, because browsers block ES modules on `file://`.

## Deploy to Vercel

The Vercel project (`getmoodex`) uses `site/` as its root and has no build command.

- **Git integration:** push to the branch Vercel watches and it deploys.
- **CLI:** `cd site && vercel` for a preview, `vercel --prod` for production.

## Change text

All copy lives in `index.html`, in the order it appears on the page. A few things are filled in at runtime:

- **Version, installer size and download link** come from the GitHub API (`initRelease` in `js/ui.js`). The values written in the HTML (`v2.0.10`, `5.2 MB`, `releases/latest`) are only fallbacks, so you only need to update them occasionally.
- **The stores strip** has two lists in `marqueeScene` (`js/scenes.js`): `STORES` (logos) and `PERKS` (highlight pills).
- **The nav island's section names** come from each `<section data-label="...">` in `index.html`.

## Icons

All icons live in the SVG sprite at the top of `index.html` and are used as `<svg class="i"><use href="#i-name"/></svg>`.

- **Line icons** (`i-*`) follow Lucide's geometry (ISC licence). The `.i` class gives them one stroke weight (1.75) so they all match.
- **Brand marks** (`b-*`: GitHub, Steam, Epic Games, Ubisoft, GOG, EA) come from Simple Icons (CC0). They are trademarks of their owners and appear only to name the supported stores.

To add an icon, copy its 24×24 path data into a new `<symbol>`.
- **The scan demo** folder path and file names are `PATH` and `FILES` in `initScan` (`js/ui.js`).
- **The changelog** is static. Add the newest release at the top of the `<ol class="log">` and drop the oldest entry.

## Swap screenshots or covers

Source art lives in `assets/website/` at the repo root:

| File | Used for |
| --- | --- |
| `dashboard.webp`, `library.webp`, `discover.webp`, `pc-games.webp` | the 3D tour and the gallery (dark-theme app screenshots, about 1800 px wide) |
| `cover0.webp` to `cover11.webp` | flying covers in the tour and cards in the scan demo |
| `logo.png` | icon + white wordmark; the light-theme logo is generated from it |

Replace a file, keeping its name, then rebuild the images from the repo root:

```sh
python scripts/site-images.py
```

The script needs Pillow 11.2 or later (for AVIF). It writes AVIF and WebP at 640 / 1080 / 1800 px, pads every screenshot to the same aspect ratio, and makes both logo variants. `img/og.jpg` (1200 x 630 social card), `favicon.png` and `apple-touch-icon.png` are kept as they are.

## How the motion works

- **One loop.** `engine.js` runs a single `requestAnimationFrame` loop. It reads `scrollY` once per frame, uses section positions cached on resize, and goes to sleep when nothing is moving. Each scene only runs while its section is near the viewport (IntersectionObserver).
- **Only compositor properties.** Scenes write `transform` and `opacity`. Progress bars use `translate`/`scale`, never `width`.
- **Native scroll.** Scroll is never hijacked. Scroll values are smoothed with a frame-rate-independent lerp, so it also feels right on phones.
- **The nav is a Dynamic Island.** At the top of the page it shows the full links. Once you scroll, it shrinks to the logo, the current section's name and a page-progress ring. Hover or keyboard focus expands it again, and on phones the menu grows out of it. The state is set in `chromeScene`, and the folding is CSS grid columns and rows animating between `0fr` and `1fr`.
- **Pinned sections** are `position: sticky` inside tall sections sized in `svh`. On phones (under 820 px wide) the gallery switches to a native swipe carousel.
- **Reduced motion.** With `prefers-reduced-motion`, nothing is pinned or animated and every scene shows its final state as a still layout (see the end of `site.css`).
- **Themes.** Every colour is a CSS custom property, set under `:root[data-theme="dark"]` and `:root[data-theme="light"]`. An inline script in `<head>` applies the saved theme, or the system theme, before first paint. The toggle uses the View Transitions API for a circular reveal, with a crossfade as the fallback.

Easter egg: type `moodex` anywhere on the page.
