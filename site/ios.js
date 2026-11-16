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
