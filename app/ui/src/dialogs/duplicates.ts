// duplicates.ts - Merge suggestions; nothing is merged without the user choosing (spec 1.3).
import { api, imgUrl, type DuplicateCandidate, type GameView } from "../api";
import { h, icon, I } from "../dom";
import { games } from "../store";
import { confirmBox, modal, toast, toastError } from "../ui";

const DISMISS_KEY = "moodex.dismissedDuplicates";

function dismissed(): Set<string> {
  try {
    return new Set(JSON.parse(localStorage.getItem(DISMISS_KEY) ?? "[]") as string[]);
  } catch {
    return new Set();
  }
}

function pairKey(c: DuplicateCandidate): string {
  return [c.a, c.b].sort().join("+");
}

function side(g: GameView): HTMLElement {
  const src = imgUrl(g.cover_key, "thumb");
  return h("div", { class: "li" }, h("div", { class: "t" }, src ? h("img", { src, alt: "" }) : null), h("div", {}, h("b", {}, g.title), h("small", {}, `${g.playtime} · ${g.play_count} sessions${g.installed ? " · installed" : ""}${g.rawg_id ? " · RAWG linked" : ""}`)), h("span", {}));
}

export async function openDuplicates(): Promise<void> {
  const m = modal("Possible duplicates", { size: "wide" });
  const list = h("div", { class: "stack" });
  m.body.append(h("p", { class: "note" }, "These look like the same game. Merging keeps the left game's details, fills gaps from the right one, and moves all play sessions. Nothing changes unless you choose."), list);
  m.foot.append(h("button", { class: "btn", onclick: () => m.close() }, "Close"));

  async function load(): Promise<void> {
    try {
      const skip = dismissed();
      // Show at most 30 pairs at a time; resolving some reveals the rest (spec 1.10).
      const all = (await api.duplicates()).filter((c) => !skip.has(pairKey(c))).slice(0, 30);
      await games.ensure(all.flatMap((c) => [c.a, c.b]));
      if (all.length === 0) {
        list.replaceChildren(h("p", { class: "note" }, "No duplicate suggestions."));
        return;
      }
      list.replaceChildren(
        ...all.map((c) => {
          const a = games.get(c.a);
          const b = games.get(c.b);
          if (!a || !b) return h("div", {});
          const merge = async (keep: GameView, absorb: GameView) => {
            const ok = await confirmBox("Merge games?", `Merge "${absorb.title}" into "${keep.title}"? Its sessions move over and it is removed.`, "Merge");
            if (!ok) return;
            try {
              await api.mergeGames(keep.id, absorb.id);
              toast("Merged", "ok");
              void load();
            } catch (e) {
              toastError(e);
            }
          };
          const notDup = () => {
            const s = dismissed();
            s.add(pairKey(c));
            try {
              localStorage.setItem(DISMISS_KEY, JSON.stringify([...s]));
            } catch {
              // Not persisted; the pair may be suggested again later.
            }
            void load();
          };
          const why = c.reason.kind === "same_title" ? "Same title" : `Similar title (${Math.round(c.reason.score * 100)}%)`;
          return h(
            "div",
            { class: "panel" },
            h("h3", {}, why),
            h("div", { class: "cols" }, side(a), side(b)),
            h("div", { class: "inline", style: "margin-top:10px" }, h("button", { class: "btn sm", onclick: () => void merge(a, b) }, icon(I.merge), "Keep left"), h("button", { class: "btn sm", onclick: () => void merge(b, a) }, icon(I.merge), "Keep right"), h("button", { class: "btn sm ghost", onclick: notDup }, "Not duplicates")),
          );
        }),
      );
    } catch (e) {
      toastError(e);
    }
  }
  void load();
  await m.closed;
}
