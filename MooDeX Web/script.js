/* ═══════════════════════════════════════════════════════════
   MooDeX — site behaviour
   ═══════════════════════════════════════════════════════════ */
(function () {
  'use strict';

  var reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  /* ── Theme ─────────────────────────────────────────────── */
  var root = document.documentElement;
  var stored = localStorage.getItem('moodex-theme');

  if (stored === 'light') root.setAttribute('data-theme', 'light');

  document.getElementById('themeToggle').addEventListener('click', function () {
    var light = root.getAttribute('data-theme') === 'light';
    if (light) {
      root.removeAttribute('data-theme');
      localStorage.setItem('moodex-theme', 'dark');
    } else {
      root.setAttribute('data-theme', 'light');
      localStorage.setItem('moodex-theme', 'light');
    }
  });

  /* ── Sticky header hairline ────────────────────────────── */
  var header = document.getElementById('siteHeader');
  var onScroll = function () {
    header.classList.toggle('is-stuck', window.scrollY > 8);
  };
  window.addEventListener('scroll', onScroll, { passive: true });
  onScroll();

  /* ── Mobile nav ────────────────────────────────────────── */
  var navToggle = document.getElementById('navToggle');
  var navLinks = document.getElementById('navLinks');

  var closeNav = function () {
    navLinks.classList.remove('is-open');
    navToggle.setAttribute('aria-expanded', 'false');
  };

  navToggle.addEventListener('click', function () {
    var open = navLinks.classList.toggle('is-open');
    navToggle.setAttribute('aria-expanded', String(open));
  });

  navLinks.addEventListener('click', function (e) {
    if (e.target.tagName === 'A') closeNav();
  });

  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') closeNav();
  });

  /* ── Segmented tour control ────────────────────────────── */
  var segs = Array.prototype.slice.call(document.querySelectorAll('.seg'));
  var panels = Array.prototype.slice.call(document.querySelectorAll('.tour-panel'));
  var thumb = document.querySelector('.seg-thumb');

  function moveThumb(btn) {
    if (!thumb || !btn) return;
    thumb.style.width = btn.offsetWidth + 'px';
    thumb.style.transform = 'translateX(' + (btn.offsetLeft - 4) + 'px)';
  }

  function select(btn, focus) {
    segs.forEach(function (s) {
      var on = s === btn;
      s.setAttribute('aria-selected', String(on));
      s.tabIndex = on ? 0 : -1;
    });
    panels.forEach(function (p) {
      p.hidden = p.dataset.panel !== btn.dataset.panel;
    });
    moveThumb(btn);
    if (focus) btn.focus();
  }

  segs.forEach(function (btn, i) {
    btn.addEventListener('click', function () { select(btn); });

    btn.addEventListener('keydown', function (e) {
      var next = null;
      if (e.key === 'ArrowRight') next = segs[(i + 1) % segs.length];
      else if (e.key === 'ArrowLeft') next = segs[(i - 1 + segs.length) % segs.length];
      else if (e.key === 'Home') next = segs[0];
      else if (e.key === 'End') next = segs[segs.length - 1];
      if (next) { e.preventDefault(); select(next, true); }
    });
  });

  if (segs.length) {
    // Fonts can shift button widths — position once they've settled.
    var initThumb = function () { moveThumb(segs[0]); };
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(initThumb);
    else initThumb();
    initThumb();

    window.addEventListener('resize', function () {
      var active = document.querySelector('.seg[aria-selected="true"]');
      moveThumb(active);
    });
  }

  /* ── Reveal on scroll ──────────────────────────────────── */
  var revealables = document.querySelectorAll('.reveal');

  if (reduceMotion || !('IntersectionObserver' in window)) {
    revealables.forEach(function (el) { el.classList.add('in'); });
  } else {
    var io = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (!entry.isIntersecting) return;
        entry.target.classList.add('in');
        io.unobserve(entry.target);
      });
    }, { threshold: 0.12, rootMargin: '0px 0px -40px 0px' });

    revealables.forEach(function (el) { io.observe(el); });
  }
})();
