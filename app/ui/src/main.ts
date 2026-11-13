// main.ts - Shell: sidebar, routing, and the live "Playing now" timer (spec 12.3).
import "./styles.css";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { api, imgUrl, type ScanEvent, type UpdateInfo } from "./api";
import { clock, h, icon, I, relativeLabel } from "./dom";
import { setMount, type Route } from "./router";
import { games, startStore } from "./store";
import { toast, toastError } from "./ui";
import { dashboardView } from "./views/dashboard";
import { libraryView } from "./views/library";
import { discoverView } from "./views/discover";
import { settingsView } from "./views/settings";
import { openGame } from "./dialogs/game";

const NAV: [Route, string, string][] = [
  ["dashboard", "Dashboard", I.dash],
  ["library", "My Library", I.lib],
  ["pc", "My PC Games", I.pc],
  ["discover", "Discover", I.discover],
  ["settings", "Settings", I.settings],
];

function boot(): void {
  const main = h("main", { class: "main", id: "main" });
  const nav = h("nav", { class: "nav", "aria-label": "Primary" });
  const nowList = h("div", { class: "stack", style: "gap:6px" });
  const nowTitle = h("h2", {}, "Playing now");
  const foot = h("div", { class: "side-foot" }, "MooDeX");
  const side = h(
    "aside",
    { class: "side" },
    h("div", { class: "brand" }, h("img", { src: "/logo-wordmark.png", alt: "MooDeX", class: "wordmark" })),
    nav,
    h("section", { class: "now", "aria-label": "Playing now" }, nowTitle, nowList),
    foot,
  );
  document.body.replaceChildren(h("div", { class: "app" }, side, main));

  let current: { el: HTMLElement; dispose: () => void } | null = null;
  const buttons = new Map<Route, HTMLButtonElement>();
  for (const [route, label, path] of NAV) {
    const b = h("button", { "aria-current": "false" }, icon(path), h("span", {}, label)) as HTMLButtonElement;
    b.addEventListener("click", () => mount(route));
    buttons.set(route, b);
    nav.append(b);
  }

  // Sliding highlight behind the active sidebar item.
  const indicator = h("span", { class: "nav-ind", "aria-hidden": "true" });
  nav.prepend(indicator);
  function moveIndicator(route: Route, instant = false): void {
    const b = buttons.get(route);
    if (!b) return;
    indicator.classList.toggle("instant", instant);
    indicator.style.transform = `translateY(${b.offsetTop}px)`;
    indicator.style.height = `${b.offsetHeight}px`;
    indicator.classList.add("ready");
  }

  const order: Route[] = NAV.map(([r]) => r);
  let currentRoute: Route | null = null;
  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

  function swap(route: Route): void {
    current?.dispose();
    const v =
      route === "dashboard" ? dashboardView() :
      route === "library" ? libraryView("library") :
      route === "pc" ? libraryView("pc") :
      route === "discover" ? discoverView() :
      settingsView();
    current = v;
    main.replaceChildren(v.el);
  }

  // Page changes travel in the direction of the sidebar: down the list rises, up the list falls.
  function mount(route: Route): void {
    if (route === currentRoute) return;
    const from = currentRoute === null ? -1 : order.indexOf(currentRoute);
    const to = order.indexOf(route);
    const first = currentRoute === null;
    currentRoute = route;
    buttons.forEach((b, r) => b.setAttribute("aria-current", r === route ? "page" : "false"));
    moveIndicator(route, first);
    document.documentElement.dataset.navDir = to >= from ? "down" : "up";
    const doc = document as Document & { startViewTransition?: (cb: () => void) => unknown };
    if (!first && !reduceMotion.matches && typeof doc.startViewTransition === "function") {
      doc.startViewTransition(() => swap(route));
    } else {
      swap(route);
      if (!first && !reduceMotion.matches) {
        main.classList.remove("page-enter");
        void main.offsetWidth;
        main.classList.add("page-enter");
      }
    }
    try {
      sessionStorage.setItem("moodex.route", route);
    } catch {
      // Route memory is a convenience only.
    }
  }
  window.addEventListener("resize", () => currentRoute && moveIndicator(currentRoute, true));
  setMount(mount);

  // Live timer ticks client-side from SessionStarted; totals are reconciled by PlaytimeTick.
  // Running games with a live clock; otherwise the most recently played game.
  let lastPlayed: string | null = null;
  function nowItem(id: string, live: boolean): HTMLElement {
    const g = games.get(id);
    if (!g) return h("div", {});
    const src = imgUrl(g.cover_key, "thumb");
    const sub = live
      ? h("span", { "data-since": g.running_since ?? "" }, "0:00:00")
      : h("span", { class: "last" }, `${relativeLabel(g.last_played)} \u00b7 ${g.playtime}`);
    const item = h(
      "button",
      { class: `now-item ${live ? "" : "is-last"}`, "aria-label": live ? `${g.title}, playing now` : `${g.title}, last played ${relativeLabel(g.last_played)}` },
      h("div", { class: "thumb" }, src ? h("img", { src, alt: "" }) : null),
      h("div", {}, h("b", { title: g.title }, g.title), sub),
    );
    item.addEventListener("click", () => void openGame(id));
    return item;
  }
  function renderNow(): void {
    const ids = games.running();
    if (ids.length > 0) {
      nowTitle.textContent = "Playing now";
      nowList.replaceChildren(...ids.map((id) => nowItem(id, true)));
      tick();
      return;
    }
    nowTitle.textContent = "Last played";
    nowList.replaceChildren(lastPlayed && games.get(lastPlayed) ? nowItem(lastPlayed, false) : h("p", { class: "now-empty" }, "Nothing played yet"));
  }
  async function loadLastPlayed(): Promise<void> {
    try {
      const ids = await api.listIds({}, "last_played");
      const first = ids[0];
      if (first) await games.ensure([first], true);
      const g = first ? games.get(first) : undefined;
      lastPlayed = g?.last_played ? g.id : null;
    } catch {
      lastPlayed = null;
    }
    renderNow();
  }
  function tick(): void {
    nowList.querySelectorAll<HTMLElement>("[data-since]").forEach((el) => {
      const since = Date.parse(el.dataset.since ?? "");
      if (!isNaN(since)) el.textContent = clock((Date.now() - since) / 1000);
    });
  }
  games.onRunning(() => void loadLastPlayed());
  window.setInterval(tick, 1000);

  let start: Route = "dashboard";
  try {
    start = (sessionStorage.getItem("moodex.route") as Route | null) ?? "dashboard";
  } catch {
    start = "dashboard";
  }
  mount(start);
  void loadLastPlayed();
  void getVersion().then((v) => (foot.textContent = `MooDeX ${v}`)).catch(() => {});
}

// Block the webview's default context menu outside text fields.
document.addEventListener("contextmenu", (e) => {
  const t = e.target as HTMLElement;
  if (!t.closest("input, textarea, .selectable")) e.preventDefault();
});

// Startup update check found a release: badge the Settings tab and say so once.
void listen<UpdateInfo>("update-available", (e) => {
  (window as unknown as { __moodexUpdate?: UpdateInfo }).__moodexUpdate = e.payload;
  document.querySelector(".nav button:last-child")?.classList.add("has-update");
  toast(`MooDeX ${e.payload.version} is available - open Settings > Updates to install it.`, "info", 10000);
});

// New installs found in watched folders (or by Scan now).
void listen<ScanEvent>("scan-event", (e) => {
  if (e.payload.type !== "finished") return;
  const r = e.payload.report;
  const names = [...r.added, ...r.linked];
  if (names.length === 0) return;
  toast(`Auto-detected ${names.length} game${names.length === 1 ? "" : "s"}: ${names.slice(0, 4).join(", ")}${names.length > 4 ? "..." : ""}`, "ok", 7000);
});

startStore()
  .catch((e) => toastError(e, "Could not connect to the library"))
  .finally(boot);
