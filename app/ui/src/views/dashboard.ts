// dashboard.ts - Status cards, playtime stats, G.O.A.T. art card, ranked most-played; all numbers come from the actor.
import { api, imgUrl, type DashboardView, type GameView, type Status } from "../api";
import { clock, h, icon, I, initials, relativeLabel, STATUS_LABEL } from "../dom";
import { games } from "../store";
import { toastError } from "../ui";
import { navigate } from "../router";
import { openGame } from "../dialogs/game";
import { openGoatPicker } from "../dialogs/goat";

const STATUS_ICON: Record<Status, string> = {
  playing: I.playCircle,
  completed: I.trophy,
  backlog: I.heart,
  dropped: I.stopCircle,
};

function greeting(): string {
  const hr = new Date().getHours();
  if (hr < 5) return "Late-night session?";
  if (hr >= 22) return "Gaming late tonight?";
  if (hr < 12) return "Good morning!";
  if (hr < 18) return "Good afternoon!";
  return "Good evening!";
}

// Jump to My Library pre-filtered by a status.
function openStatus(s: Status): void {
  try {
    const key = "moodex.view.library";
    const prev = JSON.parse(localStorage.getItem(key) ?? "{}") as Record<string, unknown>;
    localStorage.setItem(key, JSON.stringify({ ...prev, status: s }));
  } catch {
    // Filter memory is a convenience; the library still opens.
  }
  navigate("library");
}

function art(g: GameView, variant: "hero" | "thumb"): HTMLElement {
  const src = imgUrl(g.cover_key, variant);
  return src ? h("img", { src, alt: "", decoding: "async" }) : h("div", { class: "ph" }, initials(g.title));
}

function statusCard(s: Status, count: number, total: number): HTMLElement {
  const pct = total ? Math.round((count / total) * 100) : 0;
  const b = h(
    "button",
    { class: `status-card ${s}`, "aria-label": `${STATUS_LABEL[s]}: ${count} games, show them` },
    h("div", {}, h("span", {}, STATUS_LABEL[s]), h("b", {}, String(count)), h("small", {}, `${pct}% of library`)),
    icon(STATUS_ICON[s]),
  );
  b.addEventListener("click", () => openStatus(s));
  return b;
}

// Library mix as one segmented bar.
function mixBar(st: DashboardView["stats"]): HTMLElement {
  const parts: [Status, number][] = [["playing", st.playing], ["completed", st.completed], ["backlog", st.backlog], ["dropped", st.dropped]];
  const bar = h("div", { class: "mix", role: "img", "aria-label": parts.map(([s, n]) => `${STATUS_LABEL[s]} ${n}`).join(", ") });
  for (const [s, n] of parts) if (n > 0) bar.append(h("i", { style: `flex-grow:${n};background:var(--st-${s})`, title: `${STATUS_LABEL[s]}: ${n}` }));
  const legend = h("div", { class: "mix-legend" });
  for (const [s, n] of parts) legend.append(h("em", { style: `--dot:var(--st-${s})` }, `${STATUS_LABEL[s]} ${n}`));
  return h("div", {}, bar, legend);
}

function statCard(label: string, value: string, ico: string, color: string, foot: (HTMLElement | string)[]): HTMLElement {
  return h(
    "div",
    { class: "stat-card" },
    h("div", { class: "top" }, h("div", { class: "chip-ico", style: `--c:${color}` }, icon(ico)), label),
    h("div", { class: "big" }, value),
    h("div", { class: "foot" }, ...foot),
  );
}

function goatCard(g: GameView | null): HTMLElement {
  if (!g) {
    const b = h(
      "button",
      { class: "goat-card empty-goat", "aria-label": "Choose your G.O.A.T. game" },
      h("span", { class: "kick" }, icon(I.starCircle), "My G.O.A.T. game"),
      h("h3", {}, "Crown your all-time favourite"),
      h("span", { class: "note" }, "One game above the rest. Click to choose."),
    );
    b.addEventListener("click", () => void openGoatPicker());
    return b;
  }
  const edit = h("button", { class: "edit", "aria-label": "Change G.O.A.T.", title: "Change G.O.A.T." }, icon(I.pencil));
  edit.addEventListener("click", (e) => {
    e.stopPropagation();
    void openGoatPicker();
  });
  const card = h(
    "div",
    { class: "goat-card", role: "button", tabindex: 0, "aria-label": `G.O.A.T.: ${g.title}` },
    h("div", { class: "art" }, art(g, "hero")),
    h("span", { class: "kick" }, icon(I.starCircle), "My G.O.A.T. game"),
    h("h3", {}, g.title),
    h("span", { class: "played" }, icon(I.clock), `${g.playtime} played`),
    edit,
  );
  card.addEventListener("click", () => void openGame(g.id));
  card.addEventListener("keydown", (e) => {
    if (e.key === "Enter") void openGame(g.id);
  });
  return card;
}

// Gold, silver, bronze - with each game's share of all playtime.
function podium(list: GameView[], totalMinutes: number): HTMLElement {
  const names = ["1st PLACE", "2nd PLACE", "3rd PLACE"];
  const wrap = h("div", { class: "podium" });
  list.slice(0, 3).forEach((g, i) => {
    const share = totalMinutes > 0 ? Math.max(1, Math.round((g.minutes / totalMinutes) * 100)) : 0;
    const card = h(
      "button",
      { class: "pod", "aria-label": `${names[i]}: ${g.title}, ${g.playtime}` },
      h("div", { class: "art" }, art(g, "hero")),
      h("span", { class: `rank r${i + 1}` }, names[i]),
      g.running_since ? h("span", { class: "live-dot", title: "Playing now" }) : null,
      h(
        "div",
        { class: "body" },
        h("h3", { title: g.title }, g.title),
        h("span", { class: "played" }, icon(I.clock), g.playtime),
        h("div", { class: "share", "aria-hidden": "true" }, h("i", { style: `width:${share}%` })),
        h("span", { class: "share-label" }, `${share}% of all your playtime`),
      ),
    );
    card.addEventListener("click", () => void openGame(g.id));
    wrap.append(card);
  });
  return wrap;
}

function nowStrip(list: GameView[]): HTMLElement {
  const strip = h("div", { class: "now-strip" });
  for (const g of list) {
    const t = h("span", { "data-since": g.running_since ?? "" }, "0:00:00");
    const c = h(
      "button",
      { class: "now-card", "aria-label": `${g.title}, playing now` },
      h("div", { class: "t" }, art(g, "thumb")),
      h("div", {}, h("b", {}, g.title), h("span", { class: "clock" }, h("span", { class: "live-dot" }), t, h("span", { class: "note" }, `· ${g.playtime} total`))),
    );
    c.addEventListener("click", () => void openGame(g.id));
    strip.append(c);
  }
  return strip;
}

function recentRow(list: GameView[]): HTMLElement {
  const row = h("div", { class: "recent" });
  for (const g of list) {
    const m = h(
      "button",
      { class: "mini", "aria-label": `${g.title}, last played ${relativeLabel(g.last_played)}` },
      h("div", { class: "t" }, art(g, "thumb")),
      h("div", { class: "i" }, h("b", {}, g.title), h("small", {}, `${relativeLabel(g.last_played)} · ${g.playtime}`)),
    );
    m.addEventListener("click", () => void openGame(g.id));
    row.append(m);
  }
  return row;
}

export function dashboardView(): { el: HTMLElement; dispose: () => void } {
  const body = h("div", { class: "dash" });
  const el = h("section", { class: "view", "aria-label": "Dashboard" }, h("div", { class: "scroll" }, body));

  function render(d: DashboardView): void {
    const s = d.stats;
    const days = s.total_minutes / 1440;
    const avg = s.average_rating === null ? "-" : `${s.average_rating.toFixed(1)} ★`;
    const perGame = s.total ? Math.round(s.total_minutes / Math.max(1, s.total)) : 0;
    const parts: (HTMLElement | null)[] = [
      h(
        "header",
        { class: "dash-head" },
        h("div", { class: "grow" }, h("h1", {}, "Dashboard"), h("p", {}, `${greeting()} Here's your gaming overview.`)),
        h("button", { class: "btn", onclick: () => navigate("discover") }, icon(I.discover), "Discover"),
      ),
      h("div", { class: "status-grid" }, ...(["playing", "completed", "backlog", "dropped"] as Status[]).map((st) => statusCard(st, s[st], s.total))),
      s.total > 0 ? mixBar(s) : null,
      h(
        "div",
        { class: "stat-grid" },
        statCard("Total Games", String(s.total), I.gamepad, "#a78bfa", [h("span", {}, h("b", {}, String(s.installed)), " installed"), h("span", {}, "Avg rating ", h("b", {}, avg))]),
        statCard("Playtime", s.total_playtime, I.clock, "#60a5fa", [
          h("span", {}, "≈ ", h("b", {}, days >= 1 ? `${days.toFixed(1)} days` : "under a day")),
          h("span", {}, h("b", {}, String(s.total_sessions)), " sessions"),
          perGame > 0 ? h("span", {}, h("b", {}, `${Math.floor(perGame / 60)}h ${perGame % 60}m`), " per game") : "",
        ]),
        goatCard(d.goat),
      ),
    ];
    if (d.playing_now.length) parts.push(h("h2", { class: "sec" }, h("span", { class: "live-dot" }), "Playing now"), nowStrip(d.playing_now));
    parts.push(
      h("h2", { class: "sec" }, "Most Played Games", h("small", {}, "by recorded playtime")),
      d.most_played.length ? podium(d.most_played, s.total_minutes) : h("p", { class: "note" }, "Play something - MooDeX records sessions automatically."),
    );
    if (d.recently_played.length) parts.push(h("h2", { class: "sec" }, "Recently Played"), recentRow(d.recently_played));
    if (s.total === 0) {
      parts.push(h("div", { class: "empty" }, h("h2", {}, "Welcome to MooDeX"), h("p", {}, "Add games from Discover, or link a PC game to start tracking real playtime.")));
    }
    body.replaceChildren(...parts.filter((x): x is HTMLElement => x !== null));
    tick();
  }

  // Live session clocks tick client-side from running_since.
  function tick(): void {
    body.querySelectorAll<HTMLElement>("[data-since]").forEach((e) => {
      const since = Date.parse(e.dataset.since ?? "");
      if (!isNaN(since)) e.textContent = clock((Date.now() - since) / 1000);
    });
  }
  const timer = window.setInterval(tick, 1000);

  async function load(): Promise<void> {
    try {
      render(await api.dashboard());
    } catch (e) {
      toastError(e, "Could not load the dashboard");
    }
  }

  let t: number | undefined;
  const off = games.onChange(() => {
    window.clearTimeout(t);
    t = window.setTimeout(() => {
      void load();
    }, 250);
  });
  void load();
  return {
    el,
    dispose: () => {
      off();
      window.clearInterval(timer);
    },
  };
}
