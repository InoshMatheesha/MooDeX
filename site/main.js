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

  // Latest installer from GitHub Releases; static fallbacks stay if this fails.
  fetch(`https://api.github.com/repos/${REPO}/releases/latest`, { headers: { Accept: "application/vnd.github+json" } })
    .then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
    .then((rel) => {
      const asset = (rel.assets || []).find((a) => /-setup\.exe$/i.test(a.name));
      if (!asset) return;
      const v = String(rel.tag_name || "").replace(/^v/i, "");
      document.querySelectorAll("[data-download]").forEach((a) => (a.href = asset.browser_download_url));
      document.querySelectorAll("[data-version-text]").forEach((el) => (el.textContent = `v${v}`));
      document.querySelectorAll("[data-version-plain]").forEach((el) => (el.textContent = v));
      document.querySelectorAll("[data-size]").forEach((el) => (el.textContent = `${(asset.size / 1048576).toFixed(1)} MB`));
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
