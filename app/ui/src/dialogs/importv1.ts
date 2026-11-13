// importv1.ts - Review the v1 import plan before anything is written (spec 15 steps 1-5).
import { open } from "@tauri-apps/plugin-dialog";
import { api, type ImportPlan } from "../api";
import { h, minutesLabel } from "../dom";
import { modal, toast } from "../ui";

export async function openImportV1(): Promise<void> {
  const m = modal("Import from MooDeX v1", { size: "wide" });
  const content = h("div", { class: "stack" }, h("p", { class: "note" }, "Reading your v1 library..."));
  m.body.append(content);
  const cancel = h("button", { class: "btn ghost", onclick: () => m.close() }, "Cancel");
  const apply = h("button", { class: "btn primary", disabled: true }, "Import");
  m.foot.append(cancel, apply);

  let plan: ImportPlan | null = null;
  const boxes: { lib: string; local: string; box: HTMLInputElement }[] = [];

  async function load(path: string | null): Promise<void> {
    try {
      plan = await api.v1Plan(path);
      render(plan);
    } catch (e) {
      const choose = h("button", { class: "btn", onclick: async () => {
        const p = await open({ title: "Choose games_data.json", multiple: false, filters: [{ name: "MooDeX v1 library", extensions: ["json"] }] });
        if (typeof p === "string") void load(p);
      } }, "Choose file...");
      content.replaceChildren(h("p", { class: "err" }, (e as { message: string }).message), h("div", {}, choose));
    }
  }

  function render(p: ImportPlan): void {
    boxes.length = 0;
    const total = p.games.reduce((a, g) => a + g.minutes, 0);
    const summary = h("div", { class: "tiles" }, tileOf("Games", String(p.games.length)), tileOf("Proposed merges", String(p.merges.length)), tileOf("Warnings", String(p.warnings.length)), tileOf("Playtime", minutesLabel(total)));
    const merges = h("div", { class: "list" });
    for (const mg of p.merges) {
      const box = h("input", { type: "checkbox", "aria-label": `Merge ${mg.title}` });
      boxes.push({ lib: mg.library_key, local: mg.local_key, box });
      merges.append(h("label", { class: "sess", style: "grid-template-columns:auto 1fr auto;cursor:pointer" }, box, h("span", {}, mg.title), h("span", { class: "src" }, mg.reason)));
    }
    const selectAll = h("button", { class: "btn sm", onclick: () => boxes.forEach((b) => (b.box.checked = true)) }, "Select all");
    const warnings = p.warnings.length ? h("div", { class: "code selectable" }, p.warnings.map((w) => `- ${w.message}`).join("\n")) : h("p", { class: "note" }, "No warnings.");
    const parts: (HTMLElement | null)[] = [
      h("p", { class: "path selectable" }, p.source_path),
      summary,
      h("p", {}, "Nothing is changed until you press Import. Your v1 file is copied to ", h("span", { class: "path" }, "games_data.v1.backup.json"), " first and is never modified."),
      p.merges.length
        ? h("div", { class: "stack" }, h("div", { class: "inline" }, h("h3", { style: "font-size:13px;margin-right:auto" }, "v1 stored PC games twice. Tick each pair to combine into one game:"), selectAll), merges, h("p", { class: "note" }, "Unticked pairs are imported as two separate games. Nothing is merged silently."))
        : null,
      h("h3", { style: "font-size:13px" }, "Warnings"),
      warnings,
      h("p", { class: "note" }, "v1 playtime becomes one session per game, labelled as an imported estimate."),
    ];
    content.replaceChildren(...parts.filter((x): x is HTMLElement => x !== null));
    apply.removeAttribute("disabled");
  }

  apply.addEventListener("click", async () => {
    if (!plan) return;
    apply.setAttribute("disabled", "");
    const accepted: [string, string][] = boxes.filter((b) => b.box.checked).map((b) => [b.lib, b.local]);
    try {
      const r = await api.v1Apply(plan, accepted);
      m.close();
      toast(`Imported ${r.imported} games (${r.merged} merged, ${r.skipped} skipped, ${r.sessions} playtime estimates)`, "ok", 8000);
    } catch (e) {
      apply.removeAttribute("disabled");
      content.prepend(h("p", { class: "err" }, (e as { message: string }).message));
    }
  });

  void load(null);
  await m.closed;
}

function tileOf(label: string, value: string): HTMLElement {
  return h("div", { class: "tile" }, h("span", {}, label), h("b", {}, value));
}
