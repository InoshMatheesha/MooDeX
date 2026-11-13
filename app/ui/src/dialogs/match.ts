// match.ts - Pick the right RAWG entry for a title or exe; ranked by moodex-core (spec 6.3).
import { api, imgUrl, type DiscoverGame, type ExeMatch } from "../api";
import { debounce, h } from "../dom";
import { modal } from "../ui";

function rowFor(g: DiscoverGame, onPick: () => void): HTMLElement {
  const src = imgUrl(g.cover_key, "thumb");
  const el = h(
    "div",
    { class: "li pick", role: "option", tabindex: 0, "aria-selected": "false" },
    h("div", { class: "t" }, src ? h("img", { src, alt: "", loading: "lazy" }) : null),
    h("div", {}, h("b", {}, g.name), h("small", {}, [g.released?.slice(0, 4) ?? "TBA", g.genres.slice(0, 3).join(", ")].filter(Boolean).join("  ·  "))),
    g.library_id ? h("span", { class: "tag" }, "in library") : h("span", {}),
  );
  el.addEventListener("click", onPick);
  el.addEventListener("keydown", (e) => {
    if (e.key === "Enter") onPick();
  });
  return el;
}

// Resolves to the chosen RAWG game, or null if cancelled.
export async function pickMatch(initialQuery: string, exePath: string | null): Promise<DiscoverGame | null> {
  const m = modal("Match game & cover");
  const q = h("input", { class: "input", type: "search", value: initialQuery, "aria-label": "Search RAWG" });
  const list = h("div", { class: "list", role: "listbox", "aria-label": "Matches" });
  const status = h("p", { class: "note" }, "Searching...");
  m.body.append(h("label", { class: "field" }, h("span", {}, "Search"), q), status, list);
  m.foot.append(h("button", { class: "btn ghost", onclick: () => m.close(null) }, "Cancel"));

  let seq = 0;
  async function run(query: string | null, path: string | null): Promise<void> {
    const my = ++seq;
    status.textContent = "Searching...";
    let res: ExeMatch;
    try {
      res = await api.matchTitle(path, query);
    } catch (e) {
      if (my === seq) status.textContent = (e as { message: string }).message;
      return;
    }
    if (my !== seq) return;
    if (path && !q.value) q.value = res.title;
    if (res.error) {
      status.textContent = res.error.message;
      list.replaceChildren();
      return;
    }
    status.textContent = res.candidates.length ? "Best match first:" : "No matches - try another search.";
    list.replaceChildren(...res.candidates.map((g) => rowFor(g, () => m.close(g))));
  }
  q.addEventListener("input", debounce(() => void run(q.value.trim() || null, null), 300));
  void run(initialQuery || null, initialQuery ? null : exePath);
  q.focus();
  return ((await m.closed) as DiscoverGame | null | undefined) ?? null;
}
