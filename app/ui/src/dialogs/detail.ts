// detail.ts - RAWG game detail from Discover, with add-to-library.
import { api, imgUrl, type DiscoverGame, type Status } from "../api";
import { h, initials, STATUS_LABEL } from "../dom";
import { modal, toast, toastError } from "../ui";
import { openGame } from "./game";

export async function openDiscoverDetail(g: DiscoverGame, onAdded: (libId: string) => void): Promise<void> {
  const src = imgUrl(g.cover_key, "hero");
  // Same uncropped banner as the game dialog: full art over a blurred copy.
  const hero = h(
    "div",
    { class: "hero g-hero" },
    src ? h("img", { src, alt: "", class: "g-bg", "aria-hidden": "true" }) : null,
    src ? h("img", { src, alt: `${g.name} artwork`, class: "g-art" }) : h("div", { class: "g-art g-ph" }, initials(g.name)),
  );
  const m = modal(g.name, { hero });
  m.el.classList.add("game-dlg");
  const desc = h("p", { class: "note" }, "Loading details...");
  const kv = h("dl", { class: "kv" });
  m.body.append(kv, desc);

  const status = h("select", { class: "select", "aria-label": "Status" });
  for (const s of ["backlog", "playing", "completed", "dropped"] as Status[]) status.append(h("option", { value: s }, STATUS_LABEL[s]));
  const add = h("button", { class: "btn primary" }, "Add to library");
  add.addEventListener("click", async () => {
    add.setAttribute("disabled", "");
    try {
      const id = await api.addRawgGame(g, status.value as Status);
      onAdded(id);
      toast(`Added ${g.name}`, "ok");
      m.close();
    } catch (e) {
      add.removeAttribute("disabled");
      toastError(e);
    }
  });
  if (g.library_id) {
    const libId = g.library_id;
    m.foot.append(h("button", { class: "btn", onclick: () => { m.close(); void openGame(libId); } }, "Open in library"));
  } else {
    m.foot.append(status, add);
  }

  try {
    const d = await api.rawgDetails(g.id);
    kv.replaceChildren(
      h("dt", {}, "Released"), h("dd", {}, d.released ?? "TBA"),
      h("dt", {}, "Genres"), h("dd", {}, d.genres.join(", ") || "-"),
      h("dt", {}, "Developer"), h("dd", {}, d.developers.join(", ") || "-"),
      h("dt", {}, "Platforms"), h("dd", {}, d.platforms.join(", ") || "-"),
      h("dt", {}, "Metacritic"), h("dd", {}, d.metacritic ? String(d.metacritic) : "-"),
    );
    desc.textContent = d.description ? d.description.slice(0, 1600) : "No description.";
    desc.className = "selectable";
  } catch (e) {
    desc.textContent = (e as { message: string }).message;
  }
  await m.closed;
}
