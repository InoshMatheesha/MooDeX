// add.ts - Add a game: search RAWG, link a PC executable, or enter it manually.
import { open } from "@tauri-apps/plugin-dialog";
import { api, imgUrl, type DiscoverGame, type Status } from "../api";
import { debounce, h, icon, I, STATUS_LABEL } from "../dom";
import { modal, toast, toastError } from "../ui";
import { openGame } from "./game";

function statusSelect(def: Status): HTMLSelectElement {
  const s = h("select", { class: "select", "aria-label": "Status" });
  for (const v of ["backlog", "playing", "completed", "dropped"] as Status[]) s.append(h("option", { value: v, selected: v === def }, STATUS_LABEL[v]));
  return s;
}

export function openAddGame(): void {
  const m = modal("Add game");
  const tabs = h("div", { class: "seg", role: "tablist" });
  const pane = h("div", { class: "stack" });
  const defs: [string, () => void][] = [["Search RAWG", searchPane], ["PC game (exe)", () => { m.close(); void openAddPcGame(); }], ["Manual", manualPane]];
  for (const [label, fn] of defs) {
    const b = h("button", { role: "tab", "aria-pressed": "false" }, label);
    b.addEventListener("click", () => {
      tabs.querySelectorAll("button").forEach((x) => x.setAttribute("aria-pressed", String(x === b)));
      fn();
    });
    tabs.append(b);
  }
  m.body.append(tabs, pane);
  m.foot.append(h("button", { class: "btn", onclick: () => m.close() }, "Close"));
  (tabs.firstChild as HTMLButtonElement).click();

  function searchPane(): void {
    const q = h("input", { class: "input", type: "search", placeholder: "Game title", "aria-label": "Search RAWG" });
    const st = statusSelect("backlog");
    const list = h("div", { class: "list" });
    const note = h("p", { class: "note" }, "Type to search RAWG.");
    let seq = 0;
    q.addEventListener("input", debounce(async () => {
      const my = ++seq;
      const text = q.value.trim();
      if (!text) {
        list.replaceChildren();
        note.textContent = "Type to search RAWG.";
        return;
      }
      note.textContent = "Searching...";
      try {
        const p = await api.search(text, 1);
        if (my !== seq) return;
        note.textContent = p.results.length ? "" : "No results.";
        list.replaceChildren(...p.results.slice(0, 12).map((g) => resultRow(g)));
      } catch (e) {
        if (my === seq && (e as { kind: string }).kind !== "cancelled") note.textContent = (e as { message: string }).message;
      }
    }, 300));

    function resultRow(g: DiscoverGame): HTMLElement {
      const src = imgUrl(g.cover_key, "thumb");
      const btn = h("button", { class: "btn sm", disabled: !!g.library_id }, g.library_id ? "Added" : "Add");
      btn.addEventListener("click", async () => {
        btn.setAttribute("disabled", "");
        try {
          await api.addRawgGame(g, st.value as Status);
          btn.textContent = "Added";
          toast(`Added ${g.name}`, "ok");
        } catch (e) {
          btn.removeAttribute("disabled");
          toastError(e);
        }
      });
      return h("div", { class: "li" }, h("div", { class: "t" }, src ? h("img", { src, alt: "", loading: "lazy" }) : null), h("div", {}, h("b", {}, g.name), h("small", {}, g.released?.slice(0, 4) ?? "TBA")), btn);
    }

    pane.replaceChildren(h("div", { class: "inline" }, h("div", { class: "search", style: "flex:1" }, icon(I.search), q), st), note, list);
    q.focus();
  }

  function manualPane(): void {
    const title = h("input", { class: "input", placeholder: "Title", "aria-label": "Title" });
    const st = statusSelect("backlog");
    const err = h("p", { class: "err", hidden: true });
    const add = h("button", { class: "btn primary" }, "Add to library");
    add.addEventListener("click", async () => {
      const t = title.value.trim();
      if (!t) {
        err.textContent = "Enter a title.";
        err.hidden = false;
        return;
      }
      try {
        const id = await api.addGame({ title: t, status: st.value as Status });
        m.close();
        toast(`Added ${t}`, "ok");
        void openGame(id);
      } catch (e) {
        err.textContent = (e as { message: string }).message;
        err.hidden = false;
      }
    });
    pane.replaceChildren(h("label", { class: "field" }, h("span", {}, "Title"), title), h("label", { class: "field" }, h("span", {}, "Status"), st), err, h("div", {}, add));
    title.focus();
  }
}

// Pick an exe, guess the title, match a cover, then add with the install linked.
export async function openAddPcGame(): Promise<void> {
  const picked = await open({ title: "Choose the game's executable", multiple: false, directory: false, filters: [{ name: "Programs", extensions: ["exe"] }] });
  if (typeof picked !== "string") return;
  const path = picked;
  const m = modal("Add PC game");
  const title = h("input", { class: "input", "aria-label": "Title" });
  const list = h("div", { class: "list", role: "listbox", "aria-label": "Cover matches" });
  const note = h("p", { class: "note" }, "Matching on RAWG...");
  let chosen: DiscoverGame | null = null;
  const addBtn = h("button", { class: "btn primary" }, icon(I.plus), "Add game");
  m.body.append(h("p", { class: "path selectable" }, path), h("label", { class: "field" }, h("span", {}, "Title"), title), note, list);
  m.foot.append(h("button", { class: "btn ghost", onclick: () => m.close() }, "Cancel"), addBtn);

  function select(g: DiscoverGame | null, row?: HTMLElement): void {
    chosen = g;
    list.querySelectorAll(".li").forEach((x) => x.setAttribute("aria-selected", String(x === row)));
    if (g) title.value = g.name;
  }

  async function search(query: string | null): Promise<void> {
    try {
      const r = await api.matchTitle(query ? null : path, query);
      if (!title.value) title.value = r.title;
      if (r.error) {
        note.textContent = `${r.error.message} You can still add it with this title.`;
        return;
      }
      note.textContent = r.candidates.length ? "Pick the right game (best match first), or keep just the title:" : "No RAWG match - it will be added with this title.";
      const rows = r.candidates.map((g) => {
        const src = imgUrl(g.cover_key, "thumb");
        const row = h("div", { class: "li pick", role: "option", tabindex: 0 }, h("div", { class: "t" }, src ? h("img", { src, alt: "" }) : null), h("div", {}, h("b", {}, g.name), h("small", {}, g.released?.slice(0, 4) ?? "TBA")), g.library_id ? h("span", { class: "tag" }, "in library") : h("span", {}));
        row.addEventListener("click", () => select(g, row));
        row.addEventListener("keydown", (e) => {
          if (e.key === "Enter") select(g, row);
        });
        return row;
      });
      list.replaceChildren(...rows);
      if (r.candidates[0]) select(r.candidates[0], rows[0]);
    } catch (e) {
      note.textContent = (e as { message: string }).message;
    }
  }

  const research = h("button", { class: "btn sm", onclick: () => void search(title.value.trim() || null) }, icon(I.search), "Search again");
  m.body.insertBefore(h("div", {}, research), list);
  addBtn.addEventListener("click", async () => {
    const t = title.value.trim();
    if (!t) return;
    addBtn.setAttribute("disabled", "");
    try {
      const id = await api.addPcGame(path, t, chosen && chosen.name === t ? chosen : chosen && !chosen.library_id ? { ...chosen, name: t } : null);
      m.close();
      toast(`Added ${t} - press Play to launch and track it`, "ok");
      void openGame(id);
    } catch (e) {
      addBtn.removeAttribute("disabled");
      toastError(e);
    }
  });
  void search(null);
}
