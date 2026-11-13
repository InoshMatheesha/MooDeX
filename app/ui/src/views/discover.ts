// discover.ts - Trending / Upcoming / Popular / Search, virtualized with infinite scroll (spec 9, 12.3).
import { listen } from "@tauri-apps/api/event";
import { api, type ApiError, type DiscoverGame, type DiscoverPage } from "../api";
import { discoverCard } from "../cards";
import { debounce, h, icon, I } from "../dom";
import { VirtualGrid } from "../grid";
import { toast, toastError } from "../ui";
import { openDiscoverDetail } from "../dialogs/detail";
import { navigate } from "../router";

type Feed = "trending" | "upcoming" | "popular";

export function discoverView(): { el: HTMLElement; dispose: () => void } {
  let feed: Feed = "trending";
  let query = "";
  let page = 0;
  let hasNext = true;
  let loading = false;
  let generation = 0;
  const items = new Map<string, DiscoverGame>();
  let keys: string[] = [];

  const grid = new VirtualGrid({
    label: "Discover results",
    minCardWidth: 220,
    gap: 16,
    infoHeight: 86,
    render: (key, existing) => {
      const g = items.get(key);
      if (!g) return existing ?? h("article", { class: "card" });
      return discoverCard(g, (img) => grid.observeImage(img), add, existing);
    },
    onActivate: (key) => {
      const g = items.get(key);
      if (g) openDiscoverDetail(g, (id) => markOwned(g.id, id));
    },
    onEndReached: () => void more(),
  });

  const state = h("div", { class: "empty", hidden: true });

  function markOwned(rawgId: number, libId: string): void {
    const k = String(rawgId);
    const g = items.get(k);
    if (!g) return;
    items.set(k, { ...g, library_id: libId });
    grid.patch([k]);
  }

  async function add(g: DiscoverGame, btn: HTMLButtonElement): Promise<void> {
    btn.disabled = true;
    try {
      const id = await api.addRawgGame(g, "backlog");
      markOwned(g.id, id);
      toast(`Added ${g.name} to your library`, "ok");
    } catch (e) {
      btn.disabled = false;
      toastError(e, `Could not add ${g.name}`);
    }
  }

  function showError(e: ApiError): void {
    if (e.kind === "cancelled") return;
    if (keys.length > 0) {
      toastError(e);
      return;
    }
    state.hidden = false;
    grid.host.hidden = true;
    const settingsBtn = h("button", { class: "btn primary", onclick: () => navigate("settings") }, "Open Settings");
    const retry = h("button", { class: "btn", onclick: () => void reset() }, icon(I.refresh), "Retry");
    const title = e.kind === "no_key" ? "Discover needs a RAWG source" : e.kind === "offline" ? "You're offline" : "Discover is unavailable";
    state.replaceChildren(h("h2", {}, title), h("p", {}, e.message), h("div", { class: "inline" }, e.kind === "no_key" ? settingsBtn : retry));
  }

  function accept(gen: number, p: DiscoverPage): void {
    if (gen !== generation) return;
    for (const g of p.results) {
      const k = String(g.id);
      if (!items.has(k)) keys.push(k);
      items.set(k, g);
    }
    hasNext = p.has_next;
    grid.setKeys([...keys]);
    const none = keys.length === 0;
    state.hidden = !none;
    grid.host.hidden = none;
    if (none) state.replaceChildren(h("h2", {}, "No results"), h("p", {}, query ? `Nothing on RAWG matches "${query}".` : "Nothing here right now."));
  }

  async function more(): Promise<void> {
    if (loading || !hasNext) return;
    loading = true;
    const gen = generation;
    try {
      page += 1;
      const p = query ? await api.search(query, page) : await api.feed(feed, page);
      accept(gen, p);
    } catch (e) {
      page -= 1;
      if (gen === generation) showError(e as ApiError);
    } finally {
      loading = false;
    }
  }

  async function reset(): Promise<void> {
    generation += 1;
    page = 0;
    hasNext = true;
    loading = false;
    items.clear();
    keys = [];
    grid.setKeys([]);
    grid.host.scrollTop = 0;
    await more();
  }

  const tabs = h("div", { class: "chips", role: "tablist", "aria-label": "Feeds" });
  const tabDefs: [Feed, string][] = [["trending", "Trending"], ["upcoming", "Upcoming"], ["popular", "Popular"]];
  for (const [f, label] of tabDefs) {
    const t = h("button", { class: "chip", role: "tab", "aria-pressed": String(f === feed) }, label);
    t.addEventListener("click", () => {
      feed = f;
      query = "";
      search.value = "";
      tabs.querySelectorAll(".chip").forEach((x) => x.setAttribute("aria-pressed", String(x === t)));
      void reset();
    });
    tabs.append(t);
  }

  // Debounced at 300 ms; the backend cancels the in-flight search (spec 9.3).
  const search = h("input", { class: "input", type: "search", placeholder: "Search 800k+ games on RAWG", "aria-label": "Search RAWG" });
  search.addEventListener("input", debounce(() => {
    query = search.value.trim();
    tabs.querySelectorAll(".chip").forEach((x) => x.setAttribute("aria-pressed", String(!query && x.textContent?.toLowerCase() === feed)));
    void reset();
  }, 300));

  const el = h(
    "section",
    { class: "view", "aria-label": "Discover" },
    h("header", { class: "view-head" }, h("h1", {}, "Discover"), h("div", { class: "search" }, icon(I.search), search)),
    h("div", { class: "view-head", style: "padding-top:10px;padding-bottom:10px" }, tabs),
    h("div", { class: "scroll" }, grid.host, state),
  );

  // Background revalidation found new content: refresh quietly if we are on page 1.
  let unlisten: (() => void) | null = null;
  void listen("discover-updated", () => {
    if (!query && page <= 1) void reset();
  }).then((u) => (unlisten = u));

  void reset();
  return {
    el,
    dispose: () => {
      unlisten?.();
      grid.destroy();
    },
  };
}
