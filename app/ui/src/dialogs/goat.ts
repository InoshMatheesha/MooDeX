// goat.ts - Choose the one G.O.A.T. game.
import { api, imgUrl } from "../api";
import { debounce, h } from "../dom";
import { games } from "../store";
import { modal, toast, toastError } from "../ui";

export async function openGoatPicker(): Promise<void> {
  const m = modal("Choose your G.O.A.T.");
  const q = h("input", { class: "input", type: "search", placeholder: "Search your library", "aria-label": "Search your library" });
  const list = h("div", { class: "list", role: "listbox" });
  m.body.append(q, list);
  m.foot.append(h("button", { class: "btn ghost", onclick: async () => {
    await api.setGoat(null).catch(toastError);
    m.close();
  } }, "Clear G.O.A.T."), h("span", { style: "flex:1" }), h("button", { class: "btn", onclick: () => m.close() }, "Cancel"));

  async function load(): Promise<void> {
    try {
      const ids = await api.listIds({ query: q.value.trim() || null }, "playtime");
      // Capped at 30 so the list stays small without virtualization (spec 1.10); search narrows it.
      const shown = ids.slice(0, 30);
      await games.ensure(shown);
      list.replaceChildren(
        ...shown.map((id) => {
          const g = games.get(id);
          if (!g) return h("div", {});
          const src = imgUrl(g.cover_key, "thumb");
          const row = h("div", { class: "li pick", role: "option", tabindex: 0, "aria-selected": String(g.is_goat) }, h("div", { class: "t" }, src ? h("img", { src, alt: "", loading: "lazy" }) : null), h("div", {}, h("b", {}, g.title), h("small", {}, `${g.playtime} played`)), g.is_goat ? h("span", { class: "tag" }, "current") : h("span", {}));
          const pick = async () => {
            try {
              await api.setGoat(id);
              toast(`${g.title} is your G.O.A.T.`, "ok");
              m.close();
            } catch (e) {
              toastError(e);
            }
          };
          row.addEventListener("click", () => void pick());
          row.addEventListener("keydown", (e) => {
            if (e.key === "Enter") void pick();
          });
          return row;
        }),
      );
      if (shown.length === 0) list.replaceChildren(h("p", { class: "note", style: "padding:10px" }, "No games found."));
    } catch (e) {
      toastError(e);
    }
  }
  q.addEventListener("input", debounce(() => void load(), 150));
  void load();
  q.focus();
  await m.closed;
}
