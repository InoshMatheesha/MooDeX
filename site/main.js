// main.js - Theme toggle, product tour, live timer, scroll effects, latest release lookup.
(() => {
  const REPO = "InoshMatheesha/MooDeX";
  const root = document.documentElement;

  // Theme: saved choice, else the system preference (already applied in <head>).
  const setTheme = (t) => {
    root.dataset.theme = t;
    try {
      localStorage.setItem("moodex-theme", t);
    } catch (e) {
      // Storage blocked: the choice simply lasts for this visit.
    }
    document.querySelectorAll("[data-theme-toggle]").forEach((b) => b.setAttribute("aria-label", t === "dark" ? "Switch to light theme" : "Switch to dark theme"));
  };
  document.querySelectorAll("[data-theme-toggle]").forEach((b) =>
    b.addEventListener("click", () => setTheme(root.dataset.theme === "dark" ? "light" : "dark")),
  );
  setTheme(root.dataset.theme || "dark");

  // One request keeps the whole page current: version, size, download link, pill and changelog.
  // Publishing a GitHub release is enough; the static HTML is only the offline fallback.
  const setup = (rel) => (rel.assets || []).find((a) => /-setup\.exe$/i.test(a.name));
  const isStable = (rel) => !rel.draft && !rel.prerelease && /^v\d+\.\d+\.\d+$/.test(rel.tag_name || "") && setup(rel);

  // First bullet of the notes: "- **Title.** Rest of the sentence" -> { title, text }.
  // Sections like "Download" or "Updating" describe files and steps, not what changed.
  const SKIP_SECTION = /download|updating|update steps|good to know|install|checksum/i;
  const summarize = (body) => {
    const lines = String(body || "").split(/\r?\n/);
    let skipping = false;
    for (const line of lines) {
      const head = line.match(/^\s*#{1,6}\s+(.*)$/);
      if (head) {
        skipping = SKIP_SECTION.test(head[1]);
        continue;
      }
      if (skipping) continue;
      const m = line.match(/^\s*[-*]\s+\*\*(.+?)\*\*\s*(.*)$/);
      if (m && !/`|\.exe|\.msi/i.test(m[1])) {
        return { title: m[1].replace(/[.:]\s*$/, ""), text: m[2].replace(/^[:\s]+/, "").replace(/[*_`]/g, "").trim() };
      }
    }
    const plain = lines.map((l) => l.trim()).find((l) => l && !l.startsWith("#") && !l.startsWith("<"));
    return { title: "Improvements and fixes", text: (plain || "").replace(/^[-*]\s+/, "").replace(/[*_`]/g, "") };
  };
  const clip = (s, n) => (s.length > n ? `${s.slice(0, n - 1).trimEnd()}…` : s);
  const shortDate = (iso) => new Date(iso).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });

  fetch(`https://api.github.com/repos/${REPO}/releases?per_page=12`, { headers: { Accept: "application/vnd.github+json" } })
    .then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
    .then((list) => {
      const releases = (Array.isArray(list) ? list : []).filter(isStable);
      const latest = releases[0];
      if (!latest) return;
      const asset = setup(latest);
      const v = latest.tag_name.replace(/^v/i, "");
      document.querySelectorAll("[data-download]").forEach((a) => (a.href = asset.browser_download_url));
      document.querySelectorAll("[data-version-text]").forEach((el) => (el.textContent = `v${v}`));
      document.querySelectorAll("[data-version-plain]").forEach((el) => (el.textContent = v));
      document.querySelectorAll("[data-size]").forEach((el) => (el.textContent = `${(asset.size / 1048576).toFixed(1)} MB`));

      // Hero pill names what the newest release brought.
      const pill = document.querySelector("[data-pill]");
      if (pill) pill.textContent = `v${v}: ${clip(summarize(latest.body).title, 44)}`;

      // Changelog cards, built with textContent only (release text is never parsed as HTML).
      const log = document.querySelector("[data-log]");
      if (log) {
        const cards = releases.slice(0, 4).map((rel) => {
          const s = summarize(rel.body);
          const a = document.createElement("a");
          a.className = "log-card";
          a.href = rel.html_url;
          const top = document.createElement("div");
          top.className = "log-top";
          const ver = document.createElement("span");
          ver.className = "mono ver";
          ver.textContent = rel.tag_name;
          const date = document.createElement("span");
          date.className = "log-date";
          date.textContent = shortDate(rel.published_at || rel.created_at);
          top.append(ver, date);
          const h = document.createElement("h3");
          h.textContent = clip(s.title, 48);
          const p = document.createElement("p");
          p.textContent = clip(s.text, 120);
          a.append(top, h, p);
          return a;
        });
        if (cards.length) log.replaceChildren(...cards);
      }
    })
    .catch(() => {});

  // Floating nav gains an edge once the page scrolls; the product flattens as you arrive.
  const nav = document.getElementById("nav");
  const stage = document.querySelector(".stage");
  const onScroll = () => {
    nav.classList.toggle("scrolled", window.scrollY > 10);
    if (stage) stage.classList.toggle("flat", window.scrollY > 120);
  };
  window.addEventListener("scroll", onScroll, { passive: true });
  onScroll();

  // "Playing now" timer ticks like the real app.
  const timer = document.querySelector("[data-timer]");
  if (timer) {
    let secs = 42 * 60 + 13;
    setInterval(() => {
      secs += 1;
      const h = Math.floor(secs / 3600);
      const m = String(Math.floor((secs % 3600) / 60)).padStart(2, "0");
      const s = String(secs % 60).padStart(2, "0");
      timer.textContent = `${h}:${m}:${s}`;
    }, 1000);
  }

  // Product tour tabs swap real screenshots.
  const img = document.getElementById("tour-img");
  const cap = document.getElementById("tour-caption");
  const tabs = document.querySelectorAll(".tabs [role=tab]");
  tabs.forEach((tab) =>
    tab.addEventListener("click", () => {
      tabs.forEach((t) => t.setAttribute("aria-selected", String(t === tab)));
      const name = tab.dataset.shot;
      img.classList.add("swap");
      const next = new Image();
      next.src = `/img/${name}.webp`;
      const show = () => {
        img.srcset = `/img/${name}-sm.webp 900w, /img/${name}.webp 1600w`;
        img.src = `/img/${name}.webp`;
        img.alt = `MooDeX ${tab.textContent} screen`;
        cap.textContent = tab.dataset.caption || "";
        requestAnimationFrame(() => img.classList.remove("swap"));
      };
      next.complete ? setTimeout(show, 150) : (next.onload = () => setTimeout(show, 60));
    }),
  );
  // Arrow keys move between tabs.
  document.querySelector(".tabs")?.addEventListener("keydown", (e) => {
    const list = [...tabs];
    const i = list.indexOf(document.activeElement);
    if (i < 0 || !["ArrowRight", "ArrowLeft"].includes(e.key)) return;
    const n = list[(i + (e.key === "ArrowRight" ? 1 : list.length - 1)) % list.length];
    n.focus();
    n.click();
  });

  // Reveal sections and start the little feature demos when they come into view.
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const targets = document.querySelectorAll(".head, .tour, .cell, .steps li, .dl, .log article, .faq");
  if (reduce || !("IntersectionObserver" in window)) {
    targets.forEach((el) => el.classList.add("in"));
    return;
  }
  const io = new IntersectionObserver(
    (entries) =>
      entries.forEach((e) => {
        if (!e.isIntersecting) return;
        e.target.classList.add("in");
        io.unobserve(e.target);
      }),
    { rootMargin: "0px 0px -10% 0px" },
  );
  targets.forEach((el, i) => {
    el.classList.add("reveal");
    el.style.transitionDelay = `${(i % 3) * 70}ms`;
    io.observe(el);
  });
})();
