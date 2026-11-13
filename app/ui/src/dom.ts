// dom.ts - Tiny DOM helpers, icons and formatting; no framework.
import type { Status } from "./api";

type Attrs = Record<string, string | number | boolean | null | undefined | EventListener>;
type Child = Node | string | null | undefined | false;

// Create an element with attributes, listeners (on*) and children.
export function h<K extends keyof HTMLElementTagNameMap>(tag: K, attrs: Attrs = {}, ...children: Child[]): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === null || v === undefined || v === false) continue;
    if (k.startsWith("on") && typeof v === "function") {
      el.addEventListener(k.slice(2).toLowerCase(), v as EventListener);
    } else if (k === "class") {
      el.className = String(v);
    } else if (v === true) {
      el.setAttribute(k, "");
    } else {
      el.setAttribute(k, String(v));
    }
  }
  for (const c of children) {
    if (c === null || c === undefined || c === false) continue;
    el.append(typeof c === "string" ? document.createTextNode(c) : c);
  }
  return el;
}

// Inline SVG icon from a path string.
export function icon(d: string, label?: string): SVGSVGElement {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "1.7");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  if (label) {
    svg.setAttribute("role", "img");
    svg.setAttribute("aria-label", label);
  } else {
    svg.setAttribute("aria-hidden", "true");
  }
  for (const part of d.split("|")) {
    const p = document.createElementNS(ns, "path");
    p.setAttribute("d", part);
    svg.append(p);
  }
  return svg;
}

// GitHub's mark, filled (not a stroke icon).
export function githubMark(): SVGSVGElement {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("aria-hidden", "true");
  const p = document.createElementNS(ns, "path");
  p.setAttribute("fill", "currentColor");
  p.setAttribute("d", "M12 .5C5.65.5.5 5.65.5 12c0 5.08 3.29 9.39 7.86 10.91.58.1.79-.25.79-.56v-2c-3.2.7-3.87-1.37-3.87-1.37-.52-1.33-1.28-1.68-1.28-1.68-1.04-.71.08-.7.08-.7 1.16.08 1.77 1.19 1.77 1.19 1.03 1.76 2.7 1.25 3.36.96.1-.75.4-1.25.73-1.54-2.55-.29-5.24-1.28-5.24-5.69 0-1.26.45-2.29 1.19-3.1-.12-.29-.52-1.46.11-3.05 0 0 .97-.31 3.17 1.18a11 11 0 0 1 5.77 0c2.2-1.49 3.17-1.18 3.17-1.18.63 1.59.23 2.76.11 3.05.74.81 1.19 1.84 1.19 3.1 0 4.42-2.69 5.39-5.25 5.68.41.36.78 1.06.78 2.14v3.17c0 .31.21.67.8.56A11.5 11.5 0 0 0 23.5 12C23.5 5.65 18.35.5 12 .5z");
  svg.append(p);
  return svg;
}

export const I = {
  dash: "M4 13h6V4H4z|M14 20h6v-9h-6z|M14 4h6v4h-6z|M4 20h6v-3H4z",
  lib: "M4 5h4v15H4z|M10 5h4v15h-4z|M16.5 5.5l3.8 1-3.8 14.4-3.8-1z",
  pc: "M3 5h18v11H3z|M8 20h8|M12 16v4",
  discover: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z|M15.5 8.5l-2 5-5 2 2-5z",
  settings: "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6z|M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z",
  search: "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14z|M20 20l-4-4",
  plus: "M12 5v14|M5 12h14",
  play: "M7 4.5v15l12-7.5z",
  close: "M6 6l12 12|M18 6L6 18",
  trash: "M4 7h16|M10 11v6|M14 11v6|M6 7l1 13h10l1-13|M9 7V4h6v3",
  folder: "M3 6.5A1.5 1.5 0 0 1 4.5 5H9l2 2h8.5A1.5 1.5 0 0 1 21 8.5v9a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 17.5z",
  crown: "M3 8l4 4 5-7 5 7 4-4-2 11H5z",
  image: "M4 5h16v14H4z|M4 15l4-4 5 5|M14 13l2-2 4 4|M15.5 8.5h.01",
  link: "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1|M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
  merge: "M7 4v6a5 5 0 0 0 5 5h5|M17 4v16|M14 12l3 3-3 3",
  download: "M12 4v11|M7.5 10.5 12 15l4.5-4.5|M5 20h14",
  upload: "M12 20V9|M7.5 13.5 12 9l4.5 4.5|M5 4h14",
  refresh: "M20 11a8 8 0 1 0-2.3 5.7|M20 4v7h-7",
  playCircle: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z|M10 8.5v7l5.5-3.5z",
  trophy: "M8 4h8v5a4 4 0 0 1-8 0z|M8 6H5v1a3 3 0 0 0 3 3|M16 6h3v1a3 3 0 0 1-3 3|M12 13v4|M8.5 20h7|M10 17h4",
  heart: "M12 20s-7-4.4-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.6-7 10-7 10z",
  stopCircle: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z|M9.5 9.5h5v5h-5z",
  gamepad: "M7 8h10a4 4 0 0 1 4 4v1a3 3 0 0 1-5.4 1.8L14.5 14h-5l-1.1 1.8A3 3 0 0 1 3 13v-1a4 4 0 0 1 4-4z|M8 10.5v3|M6.5 12h3|M15.5 11.5h.01|M17.5 12.5h.01",
  clock: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z|M12 7.5V12l3 2",
  pencil: "M4 20h4L19 9l-4-4L4 16z|M13.5 6.5l4 4",
  arrowUpRight: "M7 17L17 7|M8 7h9v9",
  tag: "M3 12V4h8l10 10-8 8z|M7.5 7.5h.01",
  globe: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z|M3 12h18|M12 3c2.5 2.7 3.6 5.8 3.6 9s-1.1 6.3-3.6 9c-2.5-2.7-3.6-5.8-3.6-9S9.5 5.7 12 3z",
  flag: "M5 21V4|M5 4h11l-2 4 2 4H5",
  starCircle: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z|M12 7.5l1.3 2.7 3 .4-2.2 2.1.5 3-2.6-1.4-2.6 1.4.5-3-2.2-2.1 3-.4z",
};

export const STATUS_LABEL: Record<Status, string> = {
  playing: "Playing",
  completed: "Completed",
  backlog: "Want to Play",
  dropped: "Stopped",
};

export function stars(n: number | null): string {
  if (!n) return "";
  return "★".repeat(n) + "☆".repeat(5 - n);
}

// Elapsed seconds as h:mm:ss.
export function clock(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  const hh = Math.floor(s / 3600);
  const mm = Math.floor((s % 3600) / 60);
  const ss = s % 60;
  return `${hh}:${String(mm).padStart(2, "0")}:${String(ss).padStart(2, "0")}`;
}

// Minutes to "12h 35m" - display only; the source number came from the actor.
export function minutesLabel(m: number): string {
  if (!isFinite(m) || m <= 0) return "0m";
  const t = Math.round(m);
  const hh = Math.floor(t / 60);
  const mm = t % 60;
  if (hh === 0) return `${mm}m`;
  return mm === 0 ? `${hh}h` : `${hh}h ${mm}m`;
}

export function dateLabel(iso: string | null): string {
  if (!iso) return "Never";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso;
  return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

// "Today", "Yesterday", "3 days ago", "2 months ago".
export function relativeLabel(iso: string | null): string {
  if (!iso) return "Never";
  const t = Date.parse(iso);
  if (isNaN(t)) return iso;
  const days = Math.floor((Date.now() - t) / 86_400_000);
  if (days <= 0) return "Today";
  if (days === 1) return "Yesterday";
  if (days < 30) return `${days} days ago`;
  const months = Math.floor(days / 30);
  if (months < 12) return `${months} month${months === 1 ? "" : "s"} ago`;
  const years = Math.floor(months / 12);
  return `${years} year${years === 1 ? "" : "s"} ago`;
}

export function dateTimeLabel(iso: string): string {
  const d = new Date(iso);
  return isNaN(d.getTime()) ? iso : d.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

// Trailing-edge debounce.
export function debounce<A extends unknown[]>(fn: (...a: A) => void, ms: number): (...a: A) => void {
  let t: number | undefined;
  return (...a: A) => {
    window.clearTimeout(t);
    t = window.setTimeout(() => fn(...a), ms);
  };
}

// Initials for coverless placeholders.
export function initials(title: string): string {
  const words = title.split(/\s+/).filter(Boolean);
  return (words.slice(0, 2).map((w) => w[0]).join("") || "?").toUpperCase();
}
