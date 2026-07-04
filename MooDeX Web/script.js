// ── Tab Gallery ───────────────────────────────────────────────
document.querySelectorAll('.tab-btn').forEach(function(btn) {
  btn.addEventListener('click', function() {
    document.querySelectorAll('.tab-btn').forEach(function(b) { b.classList.remove('active'); });
    document.querySelectorAll('.gallery-panel').forEach(function(p) { p.classList.remove('active'); });
    btn.classList.add('active');
    document.querySelector('.gallery-panel[data-panel="' + btn.dataset.tab + '"]').classList.add('active');
  });
});

// ── Dark Mode Toggle ──────────────────────────────────────────
var themeToggle = document.getElementById('themeToggle');
var saved = localStorage.getItem('moodex-theme');
if (saved === 'dark') document.documentElement.setAttribute('data-theme', 'dark');

themeToggle.addEventListener('click', function() {
  var isDark = document.documentElement.getAttribute('data-theme') === 'dark';
  if (isDark) {
    document.documentElement.removeAttribute('data-theme');
    localStorage.setItem('moodex-theme', 'light');
  } else {
    document.documentElement.setAttribute('data-theme', 'dark');
    localStorage.setItem('moodex-theme', 'dark');
  }
});

// ── Mobile Nav Toggle ─────────────────────────────────────────
var navToggle = document.getElementById('navToggle');
var navLinks  = document.querySelector('.nav-links');

navToggle.addEventListener('click', function() {
  var isOpen = navLinks.classList.contains('mobile-open');
  if (isOpen) {
    navLinks.classList.remove('mobile-open');
    navLinks.style.cssText = '';
  } else {
    navLinks.classList.add('mobile-open');
    navLinks.style.cssText = 'display:flex;flex-direction:column;position:fixed;top:68px;left:0;right:0;background:rgba(235,245,253,0.96);backdrop-filter:blur(20px);-webkit-backdrop-filter:blur(20px);border-bottom:1px solid rgba(200,230,250,0.55);padding:20px 28px;gap:18px;z-index:199;';
  }
});

// ── Stat Count-Up Animation ───────────────────────────────────
var countEls = document.querySelectorAll('.stat-value[data-target]');
var obs = new IntersectionObserver(function(entries) {
  entries.forEach(function(entry) {
    if (!entry.isIntersecting) return;
    var el = entry.target;
    var target = parseFloat(el.dataset.target);
    var suffix = el.dataset.suffix || '';
    var dur = 1400;
    var start = performance.now();
    function tick(now) {
      var p = Math.min((now - start) / dur, 1);
      el.textContent = Math.round(target * (1 - Math.pow(1 - p, 3))) + suffix;
      if (p < 1) requestAnimationFrame(tick);
    }
    requestAnimationFrame(tick);
    obs.unobserve(el);
  });
}, { threshold: 0.6 });
countEls.forEach(function(el) { obs.observe(el); });

// ── Hero Window 3D Tilt ───────────────────────────────────────
var win = document.getElementById('heroWindow');
if (win && window.matchMedia('(prefers-reduced-motion: no-preference)').matches) {
  win.parentElement.addEventListener('mousemove', function(e) {
    var r = win.getBoundingClientRect();
    var x = (e.clientX - r.left) / r.width - 0.5;
    var y = (e.clientY - r.top) / r.height - 0.5;
    win.style.transform = 'perspective(1200px) rotateY(' + (-7 + x * 9) + 'deg) rotateX(' + (3 - y * 9) + 'deg) translateZ(0)';
  });
  win.parentElement.addEventListener('mouseleave', function() {
    win.style.transform = 'perspective(1200px) rotateY(-7deg) rotateX(3deg) translateZ(0)';
  });
}
