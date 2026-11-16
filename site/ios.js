// ios.js - iOS-style motion shared by MoOdY69's product sites: progress, nav highlight, tilt, magnet, reveal.
(function () {
  "use strict";

  var calm = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  var fine = window.matchMedia("(hover: hover) and (pointer: fine)").matches;

  // Light behind the glass and the progress hairline, added once.
  if (!document.querySelector(".ios-aurora")) {
    var aurora = document.createElement("div");
    aurora.className = "ios-aurora";
    aurora.setAttribute("aria-hidden", "true");
    aurora.innerHTML = "<i></i><i></i><i></i>";
    document.body.prepend(aurora);
  }
  var bar = document.createElement("div");
  bar.className = "ios-progress";
  bar.setAttribute("aria-hidden", "true");
  document.body.prepend(bar);

  var ticking = false;
  function onScroll() {
    ticking = false;
    var max = document.documentElement.scrollHeight - window.innerHeight;
    bar.style.transform = "scaleX(" + (max > 0 ? Math.min(1, window.scrollY / max) : 0) + ")";
  }
  window.addEventListener("scroll", function () {
    if (!ticking) { ticking = true; requestAnimationFrame(onScroll); }
  }, { passive: true });
  onScroll();

  // The logo (and anything marked) takes the page back to the top, smoothly.
  document.querySelectorAll("[data-scroll-top]").forEach(function (el) {
    el.addEventListener("click", function (e) {
      e.preventDefault();
      window.scrollTo({ top: 0, behavior: calm ? "auto" : "smooth" });
      if (history.replaceState) history.replaceState(null, "", location.pathname + location.search);
    });
  });

  // The nav's sliding highlight: the hovered link, else the section in view.
  document.querySelectorAll("[data-ios-nav]").forEach(function (nav) {
    nav.classList.add("ios-nav");
    var pill = document.createElement("span");
    pill.className = "ios-indicator";
    pill.setAttribute("aria-hidden", "true");
    nav.prepend(pill);
    var links = Array.prototype.slice.call(nav.querySelectorAll('a[href^="#"]'));
    var current = null;
    function moveTo(a) {
      if (!a) { pill.style.opacity = "0"; return; }
      pill.style.opacity = "1";
      pill.style.width = a.offsetWidth + "px";
      pill.style.transform = "translateX(" + a.offsetLeft + "px)";
    }
    links.forEach(function (a) {
      a.addEventListener("mouseenter", function () { moveTo(a); });
      a.addEventListener("focus", function () { moveTo(a); });
    });
    nav.addEventListener("mouseleave", function () { moveTo(current); });
    if ("IntersectionObserver" in window) {
      var spy = new IntersectionObserver(function (entries) {
        entries.forEach(function (e) {
          if (!e.isIntersecting) return;
          var id = "#" + e.target.id;
          current = links.filter(function (a) { return a.getAttribute("href") === id; })[0] || null;
          links.forEach(function (a) { a.toggleAttribute("aria-current", a === current); });
          if (!nav.matches(":hover")) moveTo(current);
        });
      }, { rootMargin: "-45% 0px -50% 0px" });
      links.forEach(function (a) {
        var target = document.querySelector(a.getAttribute("href"));
        if (target) spy.observe(target);
      });
    }
    window.addEventListener("resize", function () { moveTo(current); });
  });

  // Buttons give a little when pressed; marked ones lean toward the pointer.
  document.querySelectorAll("[data-magnetic]").forEach(function (el) {
    el.classList.add("ios-press");
    if (!fine || calm) return;
    el.addEventListener("pointermove", function (e) {
      var r = el.getBoundingClientRect();
      var x = e.clientX - (r.left + r.width / 2);
      var y = e.clientY - (r.top + r.height / 2);
      el.style.transform = "translate(" + (x * 0.18).toFixed(1) + "px," + (y * 0.28).toFixed(1) + "px)";
    });
    el.addEventListener("pointerleave", function () { el.style.transform = ""; });
  });

  // Cards turn toward the pointer in 3D, with a glare where it is.
  if (fine && !calm) {
    document.querySelectorAll("[data-tilt]").forEach(function (el) {
      var max = Number(el.getAttribute("data-tilt")) || 8;
      el.addEventListener("pointermove", function (e) {
        var r = el.getBoundingClientRect();
        var x = (e.clientX - r.left) / r.width;
        var y = (e.clientY - r.top) / r.height;
        el.style.transition = "transform 0.12s ease-out";
        el.style.transform = "perspective(1000px) rotateX(" + ((0.5 - y) * max).toFixed(2) + "deg) rotateY(" + ((x - 0.5) * max * 1.2).toFixed(2) + "deg) translateZ(6px)";
        el.style.setProperty("--mx", (x * 100).toFixed(1) + "%");
        el.style.setProperty("--my", (y * 100).toFixed(1) + "%");
      });
      el.addEventListener("pointerleave", function () {
        el.style.transition = "transform 0.7s var(--spring)";
        el.style.transform = "";
      });
    });
  }

  // Download counts: every release's program files of the repository, rolled in on the buttons.
  var counts = document.querySelectorAll("[data-dl-count][data-repo]");
  var byRepo = {};
  counts.forEach(function (el) { (byRepo[el.getAttribute("data-repo")] = byRepo[el.getAttribute("data-repo")] || []).push(el); });
  function render(el, total) {
    var odo = el.querySelector(".odo");
    var sr = el.querySelector(".sr-only");
    var text = total >= 100000 ? (total / 1000).toFixed(0) + "K" : total.toLocaleString("en-US");
    odo.textContent = "";
    text.split("").forEach(function (ch) {
      if (/\d/.test(ch)) {
        var digit = document.createElement("span");
        digit.className = "odo-digit";
        var strip = document.createElement("span");
        strip.className = "odo-strip";
        strip.style.setProperty("--d", ch);
        for (var n = 0; n < 10; n++) {
          var cell = document.createElement("span");
          cell.textContent = String(n);
          strip.appendChild(cell);
        }
        digit.appendChild(strip);
        odo.appendChild(digit);
      } else {
        var sep = document.createElement("span");
        sep.className = "odo-sep";
        sep.textContent = ch;
        odo.appendChild(sep);
      }
    });
    if (sr) sr.textContent = ", " + total.toLocaleString() + (total === 1 ? " download" : " downloads");
    el.setAttribute("title", total.toLocaleString() + (total === 1 ? " download" : " downloads") + " so far");
    el.hidden = false;
    if (calm || !("IntersectionObserver" in window)) { el.classList.add("odo-go"); return; }
    // Laid out at 0 first (a forced layout), then rolled to the count.
    var roll = function () {
      setTimeout(function () {
        void el.offsetWidth;
        el.classList.add("odo-go");
      }, 80);
    };
    var r = el.getBoundingClientRect();
    if (r.top < window.innerHeight && r.bottom > 0) { roll(); return; }
    var seen = new IntersectionObserver(function (entries) {
      if (entries[0].isIntersecting) { roll(); seen.disconnect(); }
    }, { threshold: 0.6 });
    seen.observe(el);
  }
  Object.keys(byRepo).forEach(function (repo) {
    fetch("https://api.github.com/repos/" + repo + "/releases?per_page=100", { headers: { Accept: "application/vnd.github+json" } })
      .then(function (r) { return r.ok ? r.json() : Promise.reject(r.status); })
      .then(function (rels) {
        var total = 0;
        rels.forEach(function (rel) {
          (rel.assets || []).forEach(function (a) {
            if (!/^SHA256SUMS|\.sig$|\.json$|\.txt$/i.test(a.name)) total += a.download_count || 0;
          });
        });
        if (total > 0) byRepo[repo].forEach(function (el) { render(el, total); });
      })
      .catch(function () {});
  });

  // Sections rise in as they arrive; items in a group follow each other.
  var items = [];
  document.querySelectorAll("[data-reveal-group]").forEach(function (g) {
    Array.prototype.forEach.call(g.children, function (c, i) {
      c.setAttribute("data-reveal", "");
      c.style.setProperty("--i", i);
    });
  });
  items = document.querySelectorAll("[data-reveal]");
  if (calm || !("IntersectionObserver" in window)) {
    items.forEach(function (el) { el.classList.add("ios-in"); });
  } else {
    var io = new IntersectionObserver(function (entries) {
      entries.forEach(function (e) {
        if (e.isIntersecting) { e.target.classList.add("ios-in"); io.unobserve(e.target); }
      });
    }, { rootMargin: "0px 0px -8% 0px" });
    items.forEach(function (el) { io.observe(el); });
  }
})();
