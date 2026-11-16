// theme.ts - Colour themes: applied as <html data-theme>, cached locally so the first paint is right.
export type Theme = "classic" | "dark" | "black" | "grey" | "light" | "system";

export const THEMES: { id: Theme; name: string; blurb: string }[] = [
  { id: "classic", name: "Classic", blurb: "The original MooDeX violet-black." },
  { id: "dark", name: "Dark", blurb: "Layered slate surfaces with real depth." },
  { id: "black", name: "Black", blurb: "True black, made for OLED screens." },
  { id: "grey", name: "Grey", blurb: "Soft graphite, easy on the eyes." },
  { id: "light", name: "Light", blurb: "Bright and crisp for daytime." },
  { id: "system", name: "System", blurb: "Follows Windows light or dark mode." },
];

const KEY = "moodex.theme";
const lightQuery = window.matchMedia("(prefers-color-scheme: light)");
let chosen: Theme = "classic";

function isTheme(v: unknown): v is Theme {
  return THEMES.some((t) => t.id === v);
}

// "system" resolves to Light or Dark (slate) from the Windows setting.
function resolve(t: Theme): Exclude<Theme, "system"> {
  if (t !== "system") return t;
  return lightQuery.matches ? "light" : "dark";
}

function paint(): void {
  const root = document.documentElement;
  root.dataset.theme = resolve(chosen);
  root.dataset.themeChoice = chosen;
}

export function currentTheme(): Theme {
  return chosen;
}

// Apply a theme; animate=true cross-fades the whole window when the browser supports it.
export function applyTheme(t: unknown, animate = false): void {
  const next: Theme = isTheme(t) ? t : "classic";
  if (next === chosen && document.documentElement.dataset.theme) return;
  chosen = next;
  try {
    localStorage.setItem(KEY, next);
  } catch {
    // The cache only avoids a flash at start; settings hold the real value.
  }
  const doc = document as Document & { startViewTransition?: (cb: () => void) => { finished: Promise<void> } };
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (!animate || reduce || typeof doc.startViewTransition !== "function") {
    paint();
    return;
  }
  // theme-fade swaps the page-slide transition for a plain cross-fade (styles.css).
  const root = document.documentElement;
  root.classList.add("theme-fade");
  doc.startViewTransition(paint).finished.finally(() => root.classList.remove("theme-fade"));
}

// Called once before the shell renders.
export function initTheme(): void {
  let cached: unknown = null;
  try {
    cached = localStorage.getItem(KEY);
  } catch {
    cached = null;
  }
  chosen = isTheme(cached) ? cached : "classic";
  paint();
  lightQuery.addEventListener("change", () => {
    if (chosen === "system") paint();
  });
}
