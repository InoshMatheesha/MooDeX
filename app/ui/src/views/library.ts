// library.ts - My Library and My PC Games: one view, two filters (spec 6.1: "PC Games" is a filter).
import { api, type GameFilter, type GameId, type SortOrder, type Status } from "../api";
import { gameCard, type PlayFn } from "../cards";
import { debounce, h, icon, I, STATUS_LABEL } from "../dom";
import { VirtualGrid } from "../grid";
import { games } from "../store";
import { toast, toastError } from "../ui";
import { openGame } from "../dialogs/game";
import { openAddGame, openAddPcGame } from "../dialogs/add";
import { openDuplicates } from "../dialogs/duplicates";

const SORTS: [SortOrder, string][] = [
  ["title", "A-Z"],
  ["recently_added", "Recently added"],
  ["playtime", "Most played"],
  ["last_played", "Last played"],
  ["rating", "Rating"],
  ["release_date", "Release date"],
];

export function libraryView(mode: "library" | "pc"): { el: HTMLElement; dispose: () => void } {
  const pc = mode === "pc";
  const pref = `moodex.view.${mode}`;
  let saved: { status: Status | null; sort: SortOrder } = { status: null, sort: pc ? "last_played" : "title" };
  try {
    const raw = localStorage.getItem(pref);
    if (raw) saved = { ...saved, ...JSON.parse(raw) };
  } catch {
    // Per-viewer convenience only; defaults are fine.
  }
  let status: Status | null = saved.status;
  let sort: SortOrder = saved.sort;
  let query = "";
  let ids: GameId[] = [];

  // Direct Play from the card; errors show inline as a toast.
  const play: PlayFn = async (g, btn) => {
    btn.disabled = true;
    try {
      const out = await api.play(g.id);
      toast(out.note ?? `Launching ${g.title} - playtime is being tracked`, out.note ? "info" : "ok");
    } catch (e) {
      btn.disabled = false;
      toastError(e, `Could not start ${g.title}`);
      if ((e as { kind?: string }).kind === "executable_missing") void openGame(g.id);
    }
  };

  const grid = new VirtualGrid({
    label: pc ? "PC games" : "Library",
    minCardWidth: 210,
    gap: 16,
    infoHeight: 62,
    render: (key, existing) => {
      const g = games.get(key);
      if (!g) return existing ?? h("article", { class: "card" });
      return gameCard(g, (img) => grid.observeImage(img), existing, play);
    },
    onActivate: (key) => openGame(key),
  });

  const count = h("span", { class: "sub" });
  const empty = h("div", { class: "empty", hidden: true });
  const dupBtn = h("button", { class: "btn sm ghost", hidden: true, onclick: () => openDuplicates() }, icon(I.merge), "");
  const savePrefs = () => {
    try {
      localStorage.setItem(pref, JSON.stringify({ status, sort }));
    } catch {
      // Storage may be unavailable; preferences simply reset.
    }
  };

  async function reload(): Promise<void> {
    const filter: GameFilter = { status, installed_only: pc, query: query || null };
    try {
      const next = await api.listIds(filter, sort);
      await games.ensure(next);
      ids = next;
      grid.setKeys(ids);
      count.textContent = `${ids.length} ${ids.length === 1 ? "game" : "games"}`;
      renderEmpty();
    } catch (e) {
      toastError(e, "Could not load the library");
    }
  }

  function renderEmpty(): void {
    const show = ids.length === 0;
    empty.hidden = !show;
    grid.host.hidden = show;
    if (!show) return;
    const filtered = !!(status || query);
    empty.replaceChildren(
      h("h2", {}, filtered ? "No matches" : pc ? "No PC games yet" : "Your library is empty"),
      h("p", {}, filtered ? "Try a different filter or search." : pc ? "Link a game's executable and MooDeX will track your real playtime." : "Add games from Discover, search RAWG, or add a PC game."),
    );
    if (!filtered) empty.append(h("button", { class: "btn primary", onclick: () => (pc ? openAddPcGame() : openAddGame()) }, icon(I.plus), pc ? "Add PC game" : "Add game"));
  }

  async function checkDuplicates(): Promise<void> {
    if (pc) return;
    try {
      const d = await api.duplicates();
      dupBtn.hidden = d.length === 0;
      dupBtn.lastChild!.textContent = ` ${d.length} possible duplicate${d.length === 1 ? "" : "s"}`;
    } catch {
      dupBtn.hidden = true;
    }
  }

  const chips = h("div", { class: "chips", role: "group", "aria-label": "Filter by status" });
  const chipDefs: [Status | null, string][] = [[null, "All"], ["playing", "Playing"], ["completed", "Completed"], ["backlog", STATUS_LABEL.backlog], ["dropped", STATUS_LABEL.dropped]];
  for (const [s, label] of chipDefs) {
    const c = h("button", { class: "chip", "aria-pressed": String(status === s) }, label);
    c.addEventListener("click", () => {
      status = s;
      chips.querySelectorAll(".chip").forEach((x) => x.setAttribute("aria-pressed", String(x === c)));
      savePrefs();
      void reload();
    });
    chips.append(c);
  }

  const sortSel = h("select", { class: "select", "aria-label": "Sort" });
  for (const [v, label] of SORTS) sortSel.append(h("option", { value: v, selected: v === sort }, label));
  sortSel.addEventListener("change", () => {
    sort = sortSel.value as SortOrder;
    savePrefs();
    void reload();
  });

  const search = h("input", { class: "input", type: "search", placeholder: pc ? "Search PC games" : "Search title or genre", "aria-label": "Search library" });
  search.addEventListener("input", debounce(() => {
    query = search.value.trim();
    void reload();
  }, 150));

  const addBtn = h("button", { class: "btn primary", onclick: () => (pc ? openAddPcGame() : openAddGame()) }, icon(I.plus), pc ? "Add PC game" : "Add game");
  const head = h(
    "header",
    { class: "view-head" },
    h("h1", {}, pc ? "My PC Games" : "My Library", count),
    dupBtn,
    h("div", { class: "search" }, icon(I.search), search),
    sortSel,
    addBtn,
  );
  const toolbar = h("div", { class: "view-head", style: "padding-top:10px;padding-bottom:10px" }, chips);
  const scroll = h("div", { class: "scroll" }, grid.host, empty);
  const el = h("section", { class: "view", "aria-label": pc ? "My PC Games" : "My Library" }, head, toolbar, scroll);

  // Keyed updates: re-query order (cheap ids), then patch only the changed cards.
  const resync = debounce(() => void reload(), 60);
  const offChange = games.onChange((changed) => {
    grid.patch(changed);
    resync();
    void checkDuplicates();
  });

  void reload();
  void checkDuplicates();
  return {
    el,
    dispose: () => {
      offChange();
      grid.destroy();
    },
  };
}
