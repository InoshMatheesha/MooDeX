// game.ts - Game detail: play, status, rating, notes, G.O.A.T., install/detection, sessions, cover.
import { open } from "@tauri-apps/plugin-dialog";
import { api, imgUrl, type GameId, type GameView, type Install, type LaunchTarget, type Status } from "../api";
import { dateLabel, dateTimeLabel, debounce, h, icon, I, initials, minutesLabel, STATUS_LABEL } from "../dom";
import { games } from "../store";
import { confirmBox, modal, toast, toastError } from "../ui";
import { pickMatch } from "./match";

const SOURCE_LABEL = { job_object: "launched", wmi: "detected", manual: "imported estimate" } as const;

export async function openGame(id: GameId): Promise<void> {
  await games.ensure([id]);
  const g0 = games.get(id);
  if (!g0) return;
  const heroImg = imgUrl(g0.cover_key, "hero");
  // Short artwork banner with the editable title on top: the whole dialog fits without scrolling.
  const heroTitle = h("div", { class: "g-hero-title" });
  const closeX = h("button", { class: "g-close", "aria-label": "Close" }, icon(I.close));
  const hero = h(
    "div",
    { class: "hero g-hero" },
    // Blurred copy fills the banner; the real artwork is shown whole, never cropped.
    heroImg ? h("img", { src: heroImg, alt: "", class: "g-bg", "aria-hidden": "true" }) : null,
    heroImg ? h("img", { src: heroImg, alt: `${g0.title} artwork`, class: "g-art" }) : h("div", { class: "g-art g-ph" }, initials(g0.title)),
    heroTitle,
    closeX,
  );
  const m = modal(g0.title, { size: "wide", hero, bare: true });
  m.el.classList.add("game-dlg");
  closeX.addEventListener("click", () => m.close());

  const title = h("input", { class: "input title-input", value: g0.title, "aria-label": "Title" });
  const saveTitle = async () => {
    const t = title.value.trim();
    const g = games.get(id);
    if (!g || !t || t === g.title) return;
    try {
      await api.updateGame(id, { title: t });
    } catch (e) {
      toastError(e);
    }
  };
  title.addEventListener("change", () => void saveTitle());

  // Status segmented control.
  const seg = h("div", { class: "seg", role: "group", "aria-label": "Status" });
  const statuses: Status[] = ["playing", "completed", "backlog", "dropped"];
  for (const s of statuses) {
    const b = h("button", { "aria-pressed": "false", "data-s": s }, STATUS_LABEL[s]);
    b.addEventListener("click", () => void api.updateGame(id, { status: s }).catch(toastError));
    seg.append(b);
  }

  // 1-5 stars; clicking the current value clears it.
  const rating = h("div", { class: "rating", role: "group", "aria-label": "Rating" });
  for (let i = 1; i <= 5; i++) {
    const b = h("button", { "aria-label": `${i} star${i === 1 ? "" : "s"}`, "data-v": i }, "\u2605");
    b.addEventListener("click", () => {
      const cur = games.get(id)?.rating ?? null;
      void api.updateGame(id, { rating: cur === i ? null : i }).catch(toastError);
    });
    rating.append(b);
  }

  const goat = h("button", { class: "btn sm" }, icon(I.crown), "G.O.A.T.");
  goat.addEventListener("click", () => {
    const g = games.get(id);
    void api.setGoat(g?.is_goat ? null : id).catch(toastError);
  });

  const notes = h("textarea", { class: "textarea", rows: 3, placeholder: "Notes, thoughts, where you left off...", "aria-label": "Notes" });
  notes.value = g0.notes;
  const saveNotes = debounce(() => {
    const g = games.get(id);
    if (g && notes.value !== g.notes) void api.updateGame(id, { notes: notes.value }).catch(toastError);
  }, 600);
  notes.addEventListener("input", saveNotes);

  const playBtn = h("button", { class: "btn primary" }, icon(I.play), "Play");
  const playErr = h("p", { class: "err", role: "alert", hidden: true });
  playBtn.addEventListener("click", async () => {
    playErr.hidden = true;
    playBtn.setAttribute("disabled", "");
    try {
      const out = await api.play(id);
      toast(out.note ?? `Launching ${games.get(id)?.title ?? "game"} - playtime is being tracked`, out.note ? "info" : "ok");
    } catch (e) {
      playErr.textContent = (e as { message: string }).message;
      playErr.hidden = false;
    } finally {
      playBtn.removeAttribute("disabled");
    }
  });

  const stats = h("dl", { class: "kv" });
  const installBox = h("div", { class: "panel" });
  const sessionsBox = h("div", { class: "list" });
  const genres = h("p", { class: "note" });

  const changeCover = h("button", { class: "btn sm" }, icon(I.image), "Change cover / RAWG link");
  changeCover.addEventListener("click", async () => {
    const g = games.get(id);
    if (!g) return;
    const pick = await pickMatch(g.title, null);
    if (!pick) return;
    try {
      await api.updateGame(id, {
        cover_url: pick.background_image,
        rawg_id: pick.id,
        release_date: pick.released,
        genres: pick.genres.length ? pick.genres : undefined,
      });
      toast("Cover updated", "ok");
    } catch (e) {
      toastError(e);
    }
  });

  const del = h("button", { class: "btn danger" }, icon(I.trash), "Delete");
  del.addEventListener("click", async () => {
    const g = games.get(id);
    if (!g) return;
    const ok = await confirmBox("Delete game?", `Remove "${g.title}" and its ${g.play_count} play session(s) from your library? This cannot be undone.`, "Delete", true);
    if (!ok) return;
    try {
      await api.deleteGame(id);
      m.close();
      toast(`Deleted ${g.title}`, "ok");
    } catch (e) {
      toastError(e);
    }
  });

  heroTitle.append(title, genres);
  const allSessions = h("button", { class: "btn sm ghost", hidden: true }, "Show all");
  m.body.append(
    h(
      "div",
      { class: "g-cols" },
      h(
        "div",
        { class: "stack" },
        h("div", { class: "inline" }, playBtn, goat, changeCover),
        playErr,
        h("div", { class: "inline" }, seg, rating),
        h("label", { class: "field" }, h("span", {}, "Notes"), notes),
        h("div", { class: "stack", style: "gap:6px" }, h("div", { class: "inline" }, h("h3", { style: "font-size:13px;margin-right:auto" }, "Recent sessions"), allSessions), sessionsBox),
      ),
      h("div", { class: "stack" }, h("div", { class: "panel" }, h("h3", {}, "Playtime"), stats), installBox),
    ),
  );
  let showAll = false;
  allSessions.addEventListener("click", () => {
    showAll = !showAll;
    void loadSessions();
  });
  m.foot.append(del, h("span", { style: "flex:1" }), h("button", { class: "btn", onclick: () => m.close() }, "Done"));

  function renderInstall(g: GameView): void {
    const parts: (HTMLElement | null)[] = [h("h3", {}, "Install & detection")];
    if (g.install) {
      parts.push(h("p", { class: "path selectable" }, launchLabel(g.install.launch)));
      if (g.install.detect_exe) parts.push(h("p", { class: "note" }, "Detects: ", h("span", { class: "path" }, g.install.detect_exe)));
      if (g.install_missing) parts.push(h("p", { class: "err" }, "The linked file is missing - re-link it below."));
      parts.push(
        h(
          "div",
          { class: "inline", style: "margin-top:10px" },
          h("button", { class: "btn sm", onclick: () => void linkExe() }, icon(I.link), "Re-link exe"),
          h("button", { class: "btn sm", onclick: () => void setDetect() }, "Detection exe..."),
          h("button", { class: "btn sm ghost", onclick: () => void editLaunch(g) }, "Advanced"),
          h("button", { class: "btn sm ghost danger", onclick: () => void api.setInstall(id, null).catch(toastError) }, "Unlink"),
        ),
      );
    } else {
      parts.push(
        h("p", { class: "note" }, "Not installed. Link an executable and MooDeX will launch the game and record exact playtime."),
        h(
          "div",
          { class: "inline", style: "margin-top:10px" },
          h("button", { class: "btn sm primary", onclick: () => void linkExe() }, icon(I.link), "Link executable"),
          h("button", { class: "btn sm", onclick: () => void editLaunch(null) }, "Steam / Epic URI or shortcut"),
        ),
      );
    }
    installBox.replaceChildren(...parts.filter((x): x is HTMLElement => x !== null));
  }

  async function pickFile(title: string, ext: string[]): Promise<string | null> {
    const p = await open({ title, multiple: false, directory: false, filters: [{ name: ext.join(", "), extensions: ext }] });
    return typeof p === "string" ? p : null;
  }

  async function linkExe(): Promise<void> {
    const p = await pickFile("Choose the game's executable", ["exe"]);
    if (!p) return;
    try {
      await api.setInstall(id, { launch: { kind: "Exe", path: p, args: [], working_dir: null }, detect_exe: null });
      toast("Executable linked", "ok");
    } catch (e) {
      toastError(e);
    }
  }

  async function setDetect(): Promise<void> {
    const g = games.get(id);
    if (!g?.install) return;
    const p = await pickFile("Choose the exe that runs while you play (the real game, not the launcher)", ["exe"]);
    if (!p) return;
    try {
      await api.setInstall(id, { ...g.install, detect_exe: p });
      toast("Detection executable set", "ok");
    } catch (e) {
      toastError(e);
    }
  }

  // URI / shortcut / args editor for launcher-mediated games (spec 8.3).
  async function editLaunch(g: GameView | null): Promise<void> {
    const cur = g?.install ?? null;
    const sub = modal("Launch settings", { size: "narrow" });
    const kind = h("select", { class: "select", "aria-label": "Launch type" });
    for (const [v, l] of [["Exe", "Executable"], ["Uri", "URI (steam://, com.epicgames.launcher://)"], ["Shortcut", "Shortcut (.lnk / .url)"]]) {
      kind.append(h("option", { value: v, selected: (cur?.launch.kind ?? "Uri") === v }, l));
    }
    const target = h("input", { class: "input", "aria-label": "Target", value: cur ? targetOf(cur.launch) : "" });
    const args = h("input", { class: "input", "aria-label": "Arguments", placeholder: "-fullscreen", value: cur?.launch.kind === "Exe" ? cur.launch.args.join(" ") : "" });
    const detect = h("input", { class: "input", "aria-label": "Detection executable", placeholder: "C:\\Games\\...\\game.exe", value: cur?.detect_exe ?? "" });
    const browse = h("button", { class: "btn sm", onclick: async () => {
      const p = kind.value === "Shortcut" ? await pickFile("Choose a shortcut", ["lnk", "url"]) : await pickFile("Choose an executable", ["exe"]);
      if (p) target.value = p;
    } }, "Browse");
    const browseDetect = h("button", { class: "btn sm", onclick: async () => {
      const p = await pickFile("Choose the real game executable", ["exe"]);
      if (p) detect.value = p;
    } }, "Browse");
    const err = h("p", { class: "err", hidden: true });
    sub.body.append(
      h("label", { class: "field" }, h("span", {}, "Launch with"), kind),
      h("label", { class: "field" }, h("span", {}, "Target"), h("div", { class: "inline" }, target, browse)),
      h("label", { class: "field" }, h("span", {}, "Arguments (executables only)"), args),
      h("label", { class: "field" }, h("span", {}, "Detection executable"), h("div", { class: "inline" }, detect, browseDetect)),
      h("p", { class: "note" }, "For Steam, Epic or Ubisoft games, launch with the store URI and set the detection exe to the game's real executable. MooDeX matches the full path only - never a partial name."),
      err,
    );
    const saveBtn = h("button", { class: "btn primary" }, "Save");
    saveBtn.addEventListener("click", async () => {
      const t = target.value.trim();
      let launch: LaunchTarget;
      if (kind.value === "Uri") launch = { kind: "Uri", uri: t };
      else if (kind.value === "Shortcut") launch = { kind: "Shortcut", path: t };
      else launch = { kind: "Exe", path: t, args: splitArgs(args.value), working_dir: null };
      const install: Install = { launch, detect_exe: detect.value.trim() || null };
      try {
        await api.setInstall(id, install);
        sub.close();
        toast("Launch settings saved", "ok");
      } catch (e) {
        err.textContent = (e as { message: string }).message;
        err.hidden = false;
      }
    });
    sub.foot.append(h("button", { class: "btn ghost", onclick: () => sub.close() }, "Cancel"), saveBtn);
  }

  async function loadSessions(): Promise<void> {
    try {
      const list = await api.sessions(id);
      if (list.length === 0) {
        sessionsBox.replaceChildren(h("p", { class: "note", style: "padding:10px" }, "No sessions yet."));
        return;
      }
      allSessions.hidden = list.length <= 3;
      allSessions.textContent = showAll ? "Show fewer" : `Show all ${Math.min(list.length, 50)}`;
      sessionsBox.classList.toggle("expanded", showAll);
      sessionsBox.replaceChildren(
        ...list.slice(0, showAll ? 50 : 3).map((s) =>
          h("div", { class: "sess" }, h("span", {}, dateTimeLabel(s.started_at)), h("span", {}, s.ended_at ? minutesLabel(s.minutes) : "in progress"), h("span", { class: "src" }, SOURCE_LABEL[s.source])),
        ),
      );
    } catch (e) {
      sessionsBox.replaceChildren(h("p", { class: "err", style: "padding:10px" }, (e as { message: string }).message));
    }
  }

  function render(): void {
    const g = games.get(id);
    if (!g) {
      m.close();
      return;
    }
    if (document.activeElement !== title) title.value = g.title;
    seg.querySelectorAll("button").forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.s === g.status)));
    rating.querySelectorAll("button").forEach((b) => b.classList.toggle("on", Number(b.dataset.v) <= (g.rating ?? 0)));
    goat.classList.toggle("primary", g.is_goat);
    goat.setAttribute("aria-pressed", String(g.is_goat));
    playBtn.hidden = !g.installed;
    playBtn.toggleAttribute("disabled", !!g.running_since);
    playBtn.lastChild!.textContent = g.running_since ? "Running" : "Play";
    genres.textContent = [g.genres.join(", "), g.release_date ? `Released ${dateLabel(g.release_date)}` : ""].filter(Boolean).join("  \u00b7  ");
    stats.replaceChildren(
      h("dt", {}, "Total"), h("dd", {}, g.playtime, g.has_estimate ? h("span", { class: "tag warn", style: "margin-left:8px", title: "Includes playtime imported from v1 as an estimate" }, "includes estimate") : null),
      h("dt", {}, "Sessions"), h("dd", {}, String(g.play_count)),
      h("dt", {}, "Last played"), h("dd", {}, g.running_since ? "Now" : dateLabel(g.last_played)),
      h("dt", {}, "Added"), h("dd", {}, dateLabel(g.added_at)),
    );
    renderInstall(g);
  }

  render();
  void loadSessions();
  const off = games.onChange((ids) => {
    if (!ids.includes(id)) return;
    render();
    void loadSessions();
  });
  await m.closed;
  off();
  saveNotes();
  const g = games.get(id);
  if (g && notes.value !== g.notes) void api.updateGame(id, { notes: notes.value }).catch(toastError);
  if (title.value.trim() && g && title.value.trim() !== g.title) void saveTitle();
}

function launchLabel(l: LaunchTarget): string {
  switch (l.kind) {
    case "Exe":
      return l.args.length ? `${l.path} ${l.args.join(" ")}` : l.path;
    case "Uri":
      return l.uri;
    case "Shortcut":
      return l.path;
  }
}

function targetOf(l: LaunchTarget): string {
  return l.kind === "Uri" ? l.uri : l.path;
}

// Split on spaces, honouring double quotes.
function splitArgs(s: string): string[] {
  const out: string[] = [];
  const re = /"([^"]*)"|(\S+)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(s)) !== null) out.push(m[1] ?? m[2] ?? "");
  return out;
}
