// settings.ts - Preferences, folders, RAWG source, data tools, updates and About (spec 11, Appendix A).
import { open, save } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { api, type Settings, type SettingsView, type UpdateInfo, type UpdateProgress } from "../api";
import { githubMark, h, icon, I } from "../dom";
import { toast, toastError } from "../ui";
import { openImportV1 } from "../dialogs/importv1";

function toggleRow(title: string, help: string, checked: boolean, onChange: (v: boolean) => void, disabled = false): HTMLElement {
  const input = h("input", { type: "checkbox", role: "switch", "aria-label": title, checked, disabled });
  input.addEventListener("change", () => onChange(input.checked));
  return h("label", { class: "switch" }, h("div", {}, h("b", {}, title), h("small", {}, help)), h("span", { class: "toggle" }, input, h("i", {})));
}

function numberField(label: string, value: number, min: number, max: number, onCommit: (v: number) => void): HTMLElement {
  const input = h("input", { class: "input", type: "number", min, max, value: String(value), "aria-label": label });
  input.addEventListener("change", () => {
    const v = Number(input.value);
    if (Number.isInteger(v) && v >= min && v <= max) onCommit(v);
    else {
      toast(`${label} must be between ${min} and ${max}`, "error");
      input.value = String(value);
    }
  });
  return h("label", { class: "field" }, h("span", {}, label), input);
}

export function settingsView(): { el: HTMLElement; dispose: () => void } {
  const body = h("div", { class: "settings" });
  const el = h("section", { class: "view", "aria-label": "Settings" }, h("header", { class: "view-head" }, h("h1", {}, "Settings")), h("div", { class: "scroll" }, h("div", { class: "pad" }, body)));

  async function set(key: keyof Settings, value: unknown): Promise<void> {
    try {
      render(await api.setSetting(key, value));
    } catch (e) {
      toastError(e);
      void load();
    }
  }

  function render(v: SettingsView): void {
    const s = v.settings;
    const general = h(
      "section",
      {},
      h("h2", {}, "General"),
      toggleRow("Minimize to tray", "Closing the window keeps MooDeX running in the tray so playtime is still recorded.", s.minimize_to_tray, (x) => void set("minimize_to_tray", x)),
      toggleRow("Launch at startup", "Start MooDeX when you sign in to Windows.", s.launch_at_startup, (x) => void set("launch_at_startup", x)),
      toggleRow("Start minimized", "When launched at startup, stay in the tray.", s.start_minimized, (x) => void set("start_minimized", x), !s.launch_at_startup),
    );

    const tracking = h(
      "section",
      {},
      h("h2", {}, "Play tracking"),
      h(
        "div",
        { class: "line" },
        numberField("Ignore sessions shorter than (seconds)", s.min_session_seconds, 0, 3600, (x) => void set("min_session_seconds", x)),
        numberField("Launcher stub threshold (seconds)", s.stub_threshold_seconds, 1, 600, (x) => void set("stub_threshold_seconds", x)),
      ),
      h("p", { class: "note", style: "padding-bottom:8px" }, "If a launched game exits faster than the stub threshold and has a separate detection executable, MooDeX keeps the session open and watches for the real game (Steam, Epic, Ubisoft)."),
    );

    // RAWG source: proxy (default), personal key in Credential Manager, or none (spec 9.1).
    const source = h("select", { class: "select", "aria-label": "RAWG source" });
    for (const [val, label] of [["proxy", "MooDeX proxy (no key needed)"], ["user", "My own RAWG API key"], ["none", "Off - local library only"]] as const) {
      source.append(h("option", { value: val, selected: s.rawg_key_source === val }, label));
    }
    source.addEventListener("change", () => void set("rawg_key_source", source.value));
    const proxy = h("input", { class: "input", type: "url", placeholder: "https://your-proxy.vercel.app/api/rawg", value: s.rawg_proxy_url, "aria-label": "Proxy URL" });
    const saveProxy = h("button", { class: "btn", onclick: () => void set("rawg_proxy_url", proxy.value.trim()) }, "Save");
    const key = h("input", { class: "input", type: "password", autocomplete: "off", placeholder: v.rawg_key_set ? "Key stored in Windows Credential Manager" : "Paste your RAWG API key", "aria-label": "RAWG API key" });
    const saveKey = h("button", { class: "btn", onclick: async () => {
      try {
        render(await api.setRawgKey(key.value.trim() || null));
        toast("Key saved to Windows Credential Manager", "ok");
      } catch (e) {
        toastError(e);
      }
    } }, "Save key");
    const clearKey = h("button", { class: "btn ghost", disabled: !v.rawg_key_set, onclick: async () => {
      try {
        render(await api.setRawgKey(null));
        toast("Key removed", "ok");
      } catch (e) {
        toastError(e);
      }
    } }, "Remove");
    const test = h("button", { class: "btn", onclick: async () => {
      test.setAttribute("disabled", "");
      try {
        const n = await api.testRawg();
        toast(`RAWG is reachable (${n} results)`, "ok");
      } catch (e) {
        toastError(e, "RAWG test failed");
      } finally {
        test.removeAttribute("disabled");
      }
    } }, "Test connection");
    const discover = h(
      "section",
      {},
      h("h2", {}, "Discover (RAWG)"),
      h("div", { class: "line" }, h("label", { class: "field" }, h("span", {}, "Source"), source), test),
      s.rawg_key_source === "proxy" ? h("div", { class: "line" }, h("label", { class: "field" }, h("span", {}, "Proxy URL"), proxy), saveProxy) : null,
      s.rawg_key_source === "user" ? h("div", { class: "line" }, h("label", { class: "field" }, h("span", {}, "Personal API key"), key), saveKey, clearKey) : null,
      h("p", { class: "note", style: "padding-bottom:8px" }, `Active route: ${v.rawg_active}. No key is ever stored in the app or its database; a personal key lives only in Windows Credential Manager.`),
    );

    const data = h(
      "section",
      {},
      h("h2", {}, "Your data"),
      h(
        "div",
        { class: "line" },
        h("button", { class: "btn", onclick: () => void doExport() }, icon(I.download), "Export library"),
        h("button", { class: "btn", onclick: () => void doImport() }, icon(I.upload), "Import export file"),
        h("button", { class: "btn ghost", onclick: () => void api.openFolder("backups").catch(toastError) }, icon(I.folder), "Backups"),
      ),
      h("div", { class: "line" }, numberField("Cover cache budget (MB)", s.image_cache_budget_mb, 50, 20000, (x) => void set("image_cache_budget_mb", x)), numberField("Daily backups kept", s.backup_retention_days, 1, 90, (x) => void set("backup_retention_days", x))),
      h("p", { class: "note", style: "padding-bottom:8px" }, "The library is a crash-safe SQLite database with a daily rolling backup. Cache budget changes apply on next start."),
    );

    const about = aboutSection();
    // Auto-detect: folders scanned now and watched for new installs.
    const folderList = h("div", { class: "list", style: "margin:6px 0 10px" });
    if (s.scan_folders.length === 0) {
      folderList.append(h("p", { class: "note", style: "padding:10px 12px" }, "No folders yet. Add the folder your games install into (for example D:\\Games or a SteamLibrary)."));
    }
    for (const f of s.scan_folders) {
      const rm = h("button", { class: "btn sm ghost danger", "aria-label": `Stop watching ${f}` }, "Remove");
      rm.addEventListener("click", () => void set("scan_folders", s.scan_folders.filter((x) => x !== f)));
      folderList.append(h("div", { class: "sess", style: "grid-template-columns:auto 1fr auto;align-items:center" }, icon(I.folder), h("span", { class: "path selectable" }, f), rm));
    }
    const addFolder = h("button", { class: "btn primary", onclick: async () => {
      const dir = await open({ title: "Choose a folder that contains your games", directory: true, multiple: false });
      if (typeof dir !== "string") return;
      if (s.scan_folders.some((x) => x.toLowerCase() === dir.toLowerCase())) return;
      toast("Scanning for games...", "info", 2500);
      await set("scan_folders", [...s.scan_folders, dir]);
    } }, icon(I.plus), "Add folder");
    const scanBtn = h("button", { class: "btn", disabled: s.scan_folders.length === 0 }, icon(I.refresh), "Scan now");
    scanBtn.addEventListener("click", async () => {
      scanBtn.setAttribute("disabled", "");
      try {
        const r = await api.scanNow();
        // Found games are announced by the global scan listener (main.ts).
        if (r.added.length + r.linked.length === 0) toast(`No new games in ${r.scanned} folder${r.scanned === 1 ? "" : "s"}`, "info");
      } catch (e) {
        toastError(e, "Scan failed");
      } finally {
        scanBtn.removeAttribute("disabled");
      }
    });
    const resetIgnored = s.scan_ignored.length
      ? h("button", { class: "btn sm ghost", onclick: () => void set("scan_ignored", []) }, `Allow ${s.scan_ignored.length} removed game${s.scan_ignored.length === 1 ? "" : "s"} again`)
      : null;
    const folders = h(
      "section",
      {},
      h("h2", {}, "Auto-detect games"),
      h("p", { class: "note", style: "padding-top:6px" }, "Pick the folders your games are installed in. MooDeX finds each game's executable, adds it with a matching cover, and watches the folders - new installs appear automatically."),
      folderList,
      h("div", { class: "line", style: "padding-top:0" }, addFolder, scanBtn, resetIgnored),
      h("p", { class: "note", style: "padding-bottom:8px" }, "Games you delete from MooDeX are remembered and not re-added. Steam libraries are detected via steamapps\\common."),
    );

    // One-time import of the old library; you review it before anything is written.
    const v1 = h("section", {}, h("h2", {}, "Import from MooDeX v1"), h("p", { class: "note", style: "padding:6px 0 10px" }, "Checking for a v1 library..."));
    void api.v1Status().then((st) => {
      const text = st.already_imported
        ? "Your v1 library has been imported. You can run it again - games already present are skipped."
        : st.path
          ? `Found a v1 library at ${st.path}. Import your games and playtime; you'll review everything first.`
          : "No v1 library found in the default location. You can choose the file manually.";
      v1.replaceChildren(
        h("h2", {}, "Import from MooDeX v1"),
        h("p", { class: "note", style: "padding:6px 0 4px" }, text),
        h("div", { class: "line" }, h("button", { class: `btn ${st.path && !st.already_imported ? "primary" : ""}`, onclick: () => void openImportV1() }, icon(I.upload), st.already_imported ? "Import again" : "Review import")),
      );
    }).catch(() => {});

    body.replaceChildren(updatesSection(s), general, folders, tracking, discover, data, v1, about);

  }

  // Updates: check GitHub Releases, then download + verify + install in one click.
  function updatesSection(s: Settings): HTMLElement {
    const status = h("p", { class: "note", style: "padding:6px 0 0" }, "");
    const notes = h("div", { class: "code selectable", hidden: true });
    const bar = h("div", { class: "upd-bar", hidden: true }, h("i", {}));
    const checkBtn = h("button", { class: "btn" }, icon(I.refresh), "Check for updates");
    const installBtn = h("button", { class: "btn primary", hidden: true }, icon(I.download), "Download & install");
    const ver = h("b", {}, "");
    void api.diagnostics().then((d) => (ver.textContent = d.version)).catch(() => {});

    function show(info: UpdateInfo): void {
      if (info.available) {
        status.replaceChildren(h("span", { class: "upd-new" }, `MooDeX ${info.version} is available`), ` - you have ${info.current}.`);
        notes.textContent = info.notes?.trim() || "No release notes.";
        notes.hidden = false;
        installBtn.hidden = false;
      } else {
        status.textContent = `You're up to date (MooDeX ${info.current}).`;
        notes.hidden = true;
        installBtn.hidden = true;
      }
    }
    checkBtn.addEventListener("click", async () => {
      checkBtn.setAttribute("disabled", "");
      status.textContent = "Checking GitHub for a new release...";
      try {
        show(await api.checkUpdate());
      } catch (e) {
        status.textContent = (e as { message: string }).message;
      } finally {
        checkBtn.removeAttribute("disabled");
      }
    });
    installBtn.addEventListener("click", async () => {
      installBtn.setAttribute("disabled", "");
      checkBtn.setAttribute("disabled", "");
      bar.hidden = false;
      status.textContent = "Downloading the update...";
      const fill = bar.firstElementChild as HTMLElement;
      const off = await listen<UpdateProgress>("update-progress", (e) => {
        const { downloaded, total } = e.payload;
        const mb = (n: number) => (n / 1048576).toFixed(1);
        if (total) {
          fill.style.width = `${Math.min(100, (downloaded / total) * 100)}%`;
          status.textContent = `Downloading ${mb(downloaded)} of ${mb(total)} MB...`;
        } else status.textContent = `Downloading ${mb(downloaded)} MB...`;
      });
      try {
        await api.installUpdate();
        status.textContent = "Installing - MooDeX will restart.";
      } catch (e) {
        status.textContent = (e as { message: string }).message;
        installBtn.removeAttribute("disabled");
        checkBtn.removeAttribute("disabled");
        bar.hidden = true;
      } finally {
        off();
      }
    });
    // A startup check may already have found something.
    const pending = (window as unknown as { __moodexUpdate?: UpdateInfo }).__moodexUpdate;
    if (pending?.available) show(pending);
    return h(
      "section",
      {},
      h("h2", {}, "Updates"),
      h("div", { class: "line" }, h("div", { style: "flex:1;min-width:200px" }, h("p", {}, "Installed version ", ver), status), checkBtn, installBtn),
      bar,
      notes,
      toggleRow("Check for updates at startup", "MooDeX asks GitHub Releases for a newer version when it starts. Updates are signature-checked before installing.", s.auto_update_check, (x) => void set("auto_update_check", x)),
    );
  }

  // About: identity, maker and project links in one card.
  function aboutSection(): HTMLElement {
    const ver = h("span", { class: "about-chip mono" }, "v-");
    const build = h("span", {}, "");
    void api.diagnostics().then((d) => {
      ver.textContent = `v${d.version}`;
      build.textContent = `build ${d.build_hash}`;
    }).catch(() => {});
    const link = (which: string, ico: SVGElement, title: string, sub: string): HTMLElement => {
      const b = h("button", { class: "about-link", "aria-label": `${title}: ${sub}` },
        h("span", { class: "about-link-ico" }, ico),
        h("span", { class: "about-link-text" }, h("b", {}, title), h("small", {}, sub)),
        h("span", { class: "about-link-go", "aria-hidden": "true" }, icon(I.arrowUpRight)),
      );
      b.addEventListener("click", () => void api.openLink(which).catch(toastError));
      return b;
    };
    const sign = h("button", { class: "about-sign-name", title: "MoOdY69 on GitHub" }, "MoOdY69");
    sign.addEventListener("click", () => void api.openLink("profile").catch(toastError));
    const logs = h("button", { class: "about-logs" }, "Open logs");
    logs.addEventListener("click", () => void api.openFolder("logs").catch(toastError));
    return h(
      "section",
      { class: "about" },
      h("div", { class: "about-mark", "aria-hidden": "true" }),
      h(
        "div",
        { class: "about-head" },
        h("img", { class: "about-icon", src: "/icon.png", alt: "" }),
        h(
          "div",
          { class: "about-id" },
          h("img", { class: "about-wordmark", src: "/logo-wordmark.png", alt: "MooDeX" }),
          h("p", { class: "about-tag" }, "Every game you own. Every hour you actually played."),
          h("div", { class: "about-meta mono" }, ver, build, h("span", {}, "Rust · Tauri 2")),
        ),
        h("div", { class: "about-sign" }, h("span", { class: "about-sign-label mono" }, "Crafted by"), sign),
      ),
      h(
        "div",
        { class: "about-links" },
        link("repo", githubMark(), "Source code", "github.com/InoshMatheesha/MooDeX"),
        link("releases", icon(I.tag), "Release notes", "What changed in each version"),
        link("site", icon(I.globe), "Website", "moodex-web.vercel.app"),
        link("issues", icon(I.flag), "Report a problem", "Opens a new GitHub issue"),
      ),
      h(
        "div",
        { class: "about-foot" },
        h("span", {}, "Free and open source · No telemetry · Your library never leaves this PC"),
        logs,
      ),
    );
  }

  async function doExport(): Promise<void> {
    try {
      const path = await save({ title: "Export MooDeX library", defaultPath: `moodex-library-${new Date().toISOString().slice(0, 10)}.json`, filters: [{ name: "MooDeX export", extensions: ["json"] }] });
      if (!path) return;
      const n = await api.exportLibrary(path);
      toast(`Exported ${n} games`, "ok");
    } catch (e) {
      toastError(e, "Export failed");
    }
  }

  async function doImport(): Promise<void> {
    try {
      const path = await open({ title: "Import MooDeX export", multiple: false, directory: false, filters: [{ name: "MooDeX export", extensions: ["json"] }] });
      if (!path || Array.isArray(path)) return;
      const r = await api.importLibrary(path);
      toast(`Imported ${r.imported} games (${r.skipped} already present)`, "ok");
    } catch (e) {
      toastError(e, "Import failed");
    }
  }

  async function load(): Promise<void> {
    try {
      render(await api.getSettings());
    } catch (e) {
      toastError(e, "Could not load settings");
    }
  }
  void load();
  return { el, dispose: () => {} };
}
