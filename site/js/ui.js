// ui.js - Event-driven pieces: theme, menu, FAQ, reveals, scan demo, pointer effects, release info, easter egg.
import { $, $$, env } from './engine.js';

const REPO = 'InoshMatheesha/MooDeX';
const later = (fn, ms) => setTimeout(fn, env.reduce ? 0 : ms);

/* ---------- theme: saved choice > system; circular reveal from the toggle ---------- */
export function initTheme(onChange) {
  const root = document.documentElement;
  const btn = $('#theme-btn');
  const metas = $$('meta[name="theme-color"]');
  const label = () => btn.setAttribute('aria-label', root.dataset.theme === 'light' ? 'Switch to dark theme' : 'Switch to light theme');
  const apply = (t, save) => {
    root.dataset.theme = t;
    if (save) {
      metas.forEach((m) => (m.content = t === 'light' ? '#F7F7FB' : '#0B0C10'));
      try { localStorage.setItem('moodex-theme', t); } catch {}
    }
    label();
    onChange?.(t);
  };
  label();

  btn.addEventListener('click', () => {
    const next = root.dataset.theme === 'light' ? 'dark' : 'light';
    if (!document.startViewTransition || env.reduce) {
      root.classList.add('theme-fade');
      apply(next, true);
      setTimeout(() => root.classList.remove('theme-fade'), 400);
      return;
    }
    const r = btn.getBoundingClientRect();
    const x = r.left + r.width / 2, y = r.top + r.height / 2;
    const end = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y));
    document.startViewTransition(() => apply(next, true)).ready.then(() => {
      root.animate(
        { clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${end}px at ${x}px ${y}px)`] },
        { duration: 700, easing: 'cubic-bezier(.2,.8,.2,1)', pseudoElement: '::view-transition-new(root)' },
      );
    }).catch(() => {});
  });

  matchMedia('(prefers-color-scheme: light)').addEventListener('change', (e) => {
    let saved = null;
    try { saved = localStorage.getItem('moodex-theme'); } catch {}
    if (!saved) apply(e.matches ? 'light' : 'dark', false);
  });
}

/* ---------- mobile menu ---------- */
export function initMenu() {
  const nav = $('#nav'), btn = $('#menu-btn'), menu = $('#menu');
  const isOpen = () => btn.getAttribute('aria-expanded') === 'true';
  const open = () => {
    menu.hidden = false;
    menu.classList.remove('closing');
    btn.setAttribute('aria-expanded', 'true');
    btn.setAttribute('aria-label', 'Close menu');
    nav.classList.add('menu-open');
    menu.querySelector('a').focus({ preventScroll: true });
  };
  const close = (refocus) => {
    if (!isOpen()) return;
    btn.setAttribute('aria-expanded', 'false');
    btn.setAttribute('aria-label', 'Open menu');
    nav.classList.remove('menu-open');
    menu.classList.add('closing');
    later(() => { if (!isOpen()) { menu.hidden = true; menu.classList.remove('closing'); } }, 220);
    if (refocus) btn.focus();
  };
  btn.addEventListener('click', () => (isOpen() ? close() : open()));
  menu.addEventListener('click', (e) => { if (e.target.closest('a')) close(); });
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape') close(true); });
  document.addEventListener('pointerdown', (e) => { if (isOpen() && !nav.contains(e.target)) close(); });
  matchMedia('(min-width: 900px)').addEventListener('change', (e) => e.matches && close());
}

/* ---------- FAQ accordion (button + region; height animates via grid rows) ---------- */
export function initFaq() {
  for (const b of $$('.faq-q')) {
    b.addEventListener('click', () => {
      const item = b.closest('.faq-item');
      const open = !item.classList.contains('open');
      item.classList.toggle('open', open);
      b.setAttribute('aria-expanded', String(open));
    });
  }
}

/* ---------- reveal on view (one-shot) ---------- */
export function initReveal() {
  const io = new IntersectionObserver((es) => es.forEach((e) => {
    if (!e.isIntersecting) return;
    e.target.classList.add('in');
    io.unobserve(e.target);
  }), { rootMargin: '0px 0px -8% 0px' });
  $$('.rv, .wipe').forEach((el) => io.observe(el));

  // The download card's orb only breathes while the card is on screen.
  const cta = $('#download');
  new IntersectionObserver(([e]) => cta.classList.toggle('is-on', e.isIntersecting)).observe(cta);
}

/* ---------- "Point at a folder" demo ---------- */
export function initScan() {
  const win = $('#scan-win');
  const path = $('#scan-path'), bar = $('#scan-bar'), status = $('#scan-status'), file = $('#scan-file'), toast = $('#scan-toast');
  const covers = $$('.sc', win);
  const PATH = 'D:\\SteamLibrary';
  const FILES = [
    'steamapps\\common\\Among Us\\Among Us.exe  game',
    'steamapps\\common\\Among Us\\UnityCrashHandler64.exe  skipped, crash reporter',
    'steamapps\\common\\Apex Legends\\r5apex.exe  game',
    'steamapps\\common\\Steamworks Shared\\_CommonRedist\\vcredist_x64.exe  skipped, redistributable',
    'steamapps\\common\\Aliens Fireteam Elite  game',
  ];
  let timers = [];
  const at = (ms, fn) => timers.push(setTimeout(fn, ms));

  function finalState() {
    path.textContent = PATH;
    bar.style.transition = 'none';
    bar.style.transform = 'none';
    status.textContent = 'Scan complete: 6 games, 3 new';
    status.classList.add('ok');
    file.textContent = '';
    toast.classList.add('on');
    covers.forEach((c) => c.classList.add('on'));
  }

  function run() {
    timers.forEach(clearTimeout);
    timers = [];
    if (env.reduce) return finalState();
    path.textContent = '';
    file.textContent = '';
    bar.style.transition = 'none';
    bar.style.transform = 'scaleX(0)';
    status.textContent = 'Waiting for a folder';
    status.classList.remove('ok');
    toast.classList.remove('on');
    covers.forEach((c) => c.classList.remove('on'));

    [...PATH].forEach((ch, i) => at(300 + i * 65, () => (path.textContent += ch)));
    const t1 = 300 + PATH.length * 65 + 250;
    at(t1, () => {
      status.textContent = 'Scanning for games...';
      bar.style.transition = 'transform 1.7s cubic-bezier(.6,0,.3,1)';
      bar.style.transform = 'scaleX(1)';
    });
    FILES.forEach((f, i) => at(t1 + 80 + i * 320, () => (file.textContent = f)));
    at(t1 + 1800, () => {
      status.textContent = 'Scan complete: 6 games, 3 new';
      status.classList.add('ok');
      file.textContent = '';
      toast.classList.add('on');
    });
    covers.forEach((c, i) => at(t1 + 2050 + i * 110, () => c.classList.add('on')));
  }

  const io = new IntersectionObserver(([e]) => { if (e.isIntersecting) { run(); io.disconnect(); } }, { threshold: 0.45 });
  io.observe(win);
  $('#replay').addEventListener('click', run);
}

/* ---------- desktop pointer effects: card tilt + spotlight, magnetic buttons ---------- */
export function initPointerFx() {
  if (!env.fine || env.reduce) return;
  let epoch = 0; // bumps on scroll so cached rects are re-read
  addEventListener('scroll', () => epoch++, { passive: true });

  const track = (el, onMove, onLeave) => {
    let rect = null, seen = -1, raf = 0, ex = 0, ey = 0;
    el.addEventListener('pointermove', (e) => {
      ex = e.clientX;
      ey = e.clientY;
      if (raf) return;
      raf = requestAnimationFrame(() => {
        raf = 0;
        if (!rect || seen !== epoch) { rect = el.getBoundingClientRect(); seen = epoch; }
        onMove((ex - rect.left) / rect.width, (ey - rect.top) / rect.height, rect);
      });
    });
    el.addEventListener('pointerleave', () => { cancelAnimationFrame(raf); raf = 0; rect = null; onLeave(); });
  };

  for (const c of $$('.fc')) {
    track(c, (x, y) => {
      c.style.setProperty('--mx', `${(x * 100).toFixed(1)}%`);
      c.style.setProperty('--my', `${(y * 100).toFixed(1)}%`);
      c.style.transform = `perspective(900px) rotateX(${((0.5 - y) * 7).toFixed(2)}deg) rotateY(${((x - 0.5) * 9).toFixed(2)}deg) translateY(-4px)`;
    }, () => (c.style.transform = ''));
  }
  for (const b of $$('.mag')) {
    track(b, (x, y, r) => {
      b.style.transform = `translate(${((x - 0.5) * r.width * 0.22).toFixed(1)}px, ${((y - 0.5) * r.height * 0.34).toFixed(1)}px)`;
    }, () => (b.style.transform = ''));
  }
}

/* ---------- live release info: version, size and a direct installer link ---------- */
export function initRelease() {
  const run = () => fetch(`https://api.github.com/repos/${REPO}/releases/latest`, { headers: { Accept: 'application/vnd.github+json' } })
    .then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
    .then((rel) => {
      const asset = (rel.assets || []).find((a) => /-setup\.exe$/i.test(a.name));
      if (!asset) return;
      const v = String(rel.tag_name || '').replace(/^v/i, '');
      const mb = (asset.size / 1048576).toFixed(1);
      $$('[data-download]').forEach((a) => (a.href = asset.browser_download_url));
      $$('[data-version]').forEach((el) => (el.textContent = `v${v}`));
      $$('[data-size]').forEach((el) => (el.textContent = `${mb} MB`));
    })
    .catch(() => {}); // static links and text stay as they are
  if ('requestIdleCallback' in window) requestIdleCallback(run, { timeout: 3000 });
  else setTimeout(run, 1500);
}

/* ---------- easter egg: type "moodex" for a burst of tiny controllers ---------- */
export function initEasterEgg() {
  let buf = '';
  addEventListener('keydown', (e) => {
    if (e.ctrlKey || e.metaKey || e.altKey || e.key.length !== 1) return;
    const t = e.target;
    if (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName)) return;
    buf = (buf + e.key.toLowerCase()).slice(-6);
    if (buf === 'moodex') { buf = ''; confetti(); }
  });
}

function confetti() {
  if (env.reduce) return;
  const cv = document.createElement('canvas');
  cv.className = 'confetti';
  cv.setAttribute('aria-hidden', 'true');
  document.body.append(cv);
  const dpr = Math.min(devicePixelRatio || 1, 2), W = innerWidth, H = innerHeight;
  cv.width = W * dpr;
  cv.height = H * dpr;
  const ctx = cv.getContext('2d');
  ctx.scale(dpr, dpr);
  const cols = ['#8B5CF6', '#A78BFA', '#60A5FA', '#4ADE80', '#FACC15', '#F87171', '#C084FC'];
  const ps = Array.from({ length: W < 700 ? 60 : 120 }, () => ({
    x: W / 2 + (Math.random() - 0.5) * 160, y: H * 0.62,
    vx: (Math.random() - 0.5) * 18, vy: -(9 + Math.random() * 13),
    r: Math.random() * 6.28, vr: (Math.random() - 0.5) * 0.3, s: 0.7 + Math.random() * 0.7,
    c: cols[(Math.random() * cols.length) | 0],
  }));
  const start = performance.now();
  (function frame(t) {
    const age = t - start;
    ctx.clearRect(0, 0, W, H);
    for (const p of ps) {
      p.vy += 0.42; p.vx *= 0.99; p.x += p.vx; p.y += p.vy; p.r += p.vr;
      ctx.save();
      ctx.translate(p.x, p.y);
      ctx.rotate(p.r);
      ctx.scale(p.s, p.s);
      ctx.globalAlpha = Math.max(0, 1 - age / 2600);
      ctx.fillStyle = p.c;
      ctx.beginPath(); // a tiny gamepad: body + two grips
      ctx.roundRect(-11, -6, 22, 11, 5.5);
      ctx.arc(-7, 4, 4.2, 0, 6.29);
      ctx.moveTo(11.2, 4);
      ctx.arc(7, 4, 4.2, 0, 6.29);
      ctx.fill();
      ctx.fillStyle = 'rgba(11,12,16,.55)';
      ctx.fillRect(-8.5, -1.6, 5, 1.6); ctx.fillRect(-6.8, -3.3, 1.6, 5);
      ctx.beginPath(); ctx.arc(6, -2.3, 1.2, 0, 6.29); ctx.arc(8.6, 0.2, 1.2, 0, 6.29); ctx.fill();
      ctx.restore();
    }
    if (age < 2600) requestAnimationFrame(frame);
    else cv.remove();
  })(start);
}
