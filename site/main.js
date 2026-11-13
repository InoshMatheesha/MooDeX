// main.js - Point download buttons at the latest GitHub release; quiet scroll touches.
(() => {
  const REPO = "InoshMatheesha/MooDeX";

  // Latest installer from GitHub; the static v2.0.4 links stay if this fails.
  fetch(`https://api.github.com/repos/${REPO}/releases/latest`, { headers: { Accept: "application/vnd.github+json" } })
    .then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
    .then((rel) => {
      const asset = (rel.assets || []).find((a) => /-setup\.exe$/i.test(a.name));
      if (!asset) return;
      const version = String(rel.tag_name || "").replace(/^v/i, "");
      const mb = (asset.size / 1048576).toFixed(1);
      document.querySelectorAll("[data-download]").forEach((a) => (a.href = asset.browser_download_url));
      document.querySelectorAll("[data-version]").forEach((el) => {
        el.textContent = el.closest(".cta-meta") ? `v${version}` : version;
      });
      document.querySelectorAll("[data-size]").forEach((el) => (el.textContent = `${mb} MB`));
    })
    .catch(() => {});

  // Hairline under the top bar once the page scrolls.
  const top = document.getElementById("top");
  const onScroll = () => top.classList.toggle("scrolled", window.scrollY > 8);
  window.addEventListener("scroll", onScroll, { passive: true });
  onScroll();

  // Gentle reveal for chapters as they enter the viewport.
  if (!window.matchMedia("(prefers-reduced-motion: reduce)").matches && "IntersectionObserver" in window) {
    const io = new IntersectionObserver(
      (entries) => entries.forEach((e) => {
        if (e.isIntersecting) {
          e.target.classList.add("in");
          io.unobserve(e.target);
        }
      }),
      { rootMargin: "0px 0px -8% 0px" },
    );
    document.querySelectorAll(".chapter .split, .chapter .spec, .chapter .log, .chapter .faq, .steps-row > div").forEach((el) => {
      el.classList.add("reveal");
      io.observe(el);
    });
  }
})();
