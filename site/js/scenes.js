// scenes.js - Every scroll-driven scene. Each one only writes transform / opacity (and text when a value changes).
import { $, $$, clamp, mix, range, ease, easeOut, easeIn, env, follow, progress, scene, wake } from './engine.js';

const fx = (n, d = 1) => n.toFixed(d);

/* ---------- hero: particles, parallax fade, gradient words ---------- */
export function heroScene() {
  const el = $('#top');
  const inner = $('#hero-in');
  const cv = $('#particles');
  const ctx = cv.getContext('2d');
  const line = $('#grad-line');
  const words = $$('.w > span', line);
  let W = 0, H = 0, parts = [], light = false, rgb = '214,205,255', sprites = [];
  let lastP = -1, px = 0, tx = 0;

  if (env.fine && !env.reduce) {
    addEventListener('pointermove', (e) => { if (env.y < env.vh) { tx = e.clientX / env.vw - 0.5; wake(); } }, { passive: true });
  }

  // One gradient across the whole second line, even though each word animates on its own.
  function sizeGradient() {
    const lr = line.getBoundingClientRect();
    for (const w of words) {
      const r = w.parentElement.getBoundingClientRect();
      w.style.backgroundSize = `${fx(lr.width)}px 100%`;
      w.style.backgroundPosition = `${fx(lr.left - r.left)}px 0`;
    }
  }

  function sprite(color) {
    const c = document.createElement('canvas');
    c.width = c.height = 64;
    const g = c.getContext('2d');
    const rg = g.createRadialGradient(32, 32, 0, 32, 32, 32);
    rg.addColorStop(0, color);
    rg.addColorStop(1, 'rgba(255,255,255,0)');
    g.fillStyle = rg;
    g.fillRect(0, 0, 64, 64);
    return c;
  }

  function retheme() {
    light = document.documentElement.dataset.theme === 'light';
    rgb = getComputedStyle(document.documentElement).getPropertyValue('--particle').trim();
    sprites = light ? ['rgba(139,92,246,.55)', 'rgba(96,165,250,.5)', 'rgba(236,72,153,.35)'].map(sprite) : [];
    build();
    draw(env.time);
  }

  function build() {
    const dpr = Math.min(devicePixelRatio || 1, 2);
    W = cv.clientWidth;
    H = cv.clientHeight;
    cv.width = Math.round(W * dpr);
    cv.height = Math.round(H * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const phone = env.vw < 700;
    const n = light
      ? Math.round(clamp((W * H) / (phone ? 14000 : 24000), 14, 60))
      : Math.round(clamp((W * H) / (phone ? 6500 : 8500), 40, 200));
    parts = Array.from({ length: n }, () => ({ x: Math.random() * W, y: Math.random() * H, z: Math.random() * 0.9 + 0.1, t: Math.random() * 6.28, k: (Math.random() * 3) | 0 }));
  }

  function draw(t) {
    ctx.clearRect(0, 0, W, H);
    const drift = env.reduce ? 0 : t;
    if (!light) {
      ctx.fillStyle = `rgb(${rgb})`;
      for (const p of parts) {
        ctx.globalAlpha = 0.18 + 0.62 * p.z * (0.55 + 0.45 * Math.sin(drift / 800 + p.t));
        const s = 0.6 + 1.5 * p.z;
        let y = (p.y - drift * 0.006 * p.z) % H;
        if (y < 0) y += H;
        ctx.fillRect(p.x + px * 36 * p.z, y, s, s);
      }
    } else {
      for (const p of parts) {
        const s = 6 + 22 * p.z;
        let y = (p.y - drift * 0.011 * (0.4 + p.z)) % (H + 40);
        if (y < -20) y += H + 40;
        const x = p.x + Math.sin(drift / 2600 + p.t) * 22 * p.z + px * 24 * p.z;
        ctx.globalAlpha = 0.35 + 0.45 * p.z * (0.6 + 0.4 * Math.sin(drift / 1400 + p.t));
        ctx.drawImage(sprites[p.k], x - s / 2, y - s / 2, s, s);
      }
    }
    ctx.globalAlpha = 1;
  }

  const s = scene({
    el,
    margin: '0px',
    measure() { sizeGradient(); retheme(); },
    update(_, force) {
      const p = env.reduce ? 0 : clamp(env.y / env.vh);
      if (p !== lastP || force) {
        lastP = p;
        inner.style.transform = p ? `translate3d(0,${fx(-p * 90)}px,0) scale(${fx(1 - p * 0.06, 4)})` : '';
        inner.style.opacity = p ? fx(Math.max(0, 1 - p * 1.35), 3) : '';
      }
      if (env.reduce) return false;
      px = follow(px, tx, 0.05);
      draw(env.time);
      return true;
    },
  });
  s.retheme = retheme;
  return s;
}

/* ---------- 3D tour: scattered covers gather into a library, fly past, dashboard lands ---------- */
function seeded(seed) {
  return () => {
    seed = (seed * 16807) % 2147483647;
    return (seed - 1) / 2147483646;
  };
}

export function tourScene() {
  const el = $('#tour');
  const cardsEl = $('#tour-cards');
  const all = $$('.tcard', cardsEl);
  const shot = $('#tour-shot');
  const caps = $$('.cap', el).map((c) => ({ el: c, a: +c.dataset.in, b: +c.dataset.out, o: -1 }));
  const rnd = seeded(11);
  const seeds = all.map(() => Array.from({ length: 6 }, rnd));
  let cards = [], L = { y: 0, zs: 1 }, sm = 0, lastP = -1, merged = false;

  return scene({
    el,
    margin: '30% 0px',
    measure() {
      if (env.reduce) return;
      const phone = env.vw < 700, desk = env.vw >= 1000;
      const n = phone ? 6 : 12, cols = phone ? 2 : 4, rows = n / cols;
      const cw = all[0].offsetWidth, ch = (cw * 9) / 16, gap = phone ? 8 : Math.round(cw * 0.08);
      const gx = desk ? env.vw * 0.19 : 0, gy = phone ? -env.vh * 0.05 : 0;
      const zs = phone ? 0.7 : 1;
      cards = all.slice(0, n).map((c, i) => {
        const [a, b, d, e, f, g] = seeds[i];
        // keep scattered cards out of the dead centre so the first frame reads as "scattered"
        const sx = (a < 0.5 ? -1 : 1) * (0.12 + Math.abs(a - 0.5) * 1.5) * env.vw * 0.5;
        return {
          el: c, i, ph: a * 6.28, vis: '',
          gx: gx + ((i % cols) - (cols - 1) / 2) * (cw + gap),
          gy: gy + (Math.floor(i / cols) - (rows - 1) / 2) * (ch + gap),
          sx, sy: (b * 2 - 1) * env.vh * 0.38, sz: (-200 - d * 1500) * zs,
          rx: (e - 0.5) * 50, ry: (f - 0.5) * 70, rz: (g - 0.5) * 36,
        };
      });
      const ratio = 1800 / 1052, bar = 34;
      const w = Math.min(env.vw * (phone ? 0.94 : desk ? 0.84 : 0.9), (env.vh * (phone ? 0.52 : 0.68) - bar) * ratio);
      const h = w / ratio + bar;
      L = { y: env.vh * (phone ? 0.05 : 0.08), zs };
      shot.style.width = `${fx(w)}px`;
      shot.style.marginLeft = `${fx(-w / 2)}px`;
      shot.style.marginTop = `${fx(-h / 2)}px`;
    },
    update(s, force) {
      if (env.reduce) return false;
      const target = progress(s);
      sm = follow(sm, target, env.touch ? 0.2 : 0.12);
      if (sm === lastP && !force) return false;
      lastP = sm;
      const p = sm;
      const g = ease(range(p, 0.16, 0.42)); // gather into the grid
      const fly = range(p, 0.58, 0.8); // grid flies past the camera
      const flyE = easeIn(fly);
      // store chips flip to "In library" once, as a crisp CSS transition rather than a scrubbed cross-fade
      if ((p > 0.49) !== merged) { merged = p > 0.49; cardsEl.classList.toggle('merged', merged); }

      for (const c of cards) {
        const k = 1 - g;
        let x = mix(c.sx, c.gx, g);
        let y = mix(c.sy + Math.sin(c.ph + p * 9) * 18, c.gy, g);
        let z = mix(c.sz + p * 900 * L.zs, 0, g);
        if (flyE > 0) {
          const spread = 1 + flyE * 1.6;
          x *= spread;
          y *= spread;
          z += flyE * (820 + (c.i % 4) * 50) * L.zs;
        }
        const o = 1 - range(fly, 0.45, 0.95);
        const vis = o <= 0 ? 'hidden' : '';
        if (vis !== c.vis) { c.vis = vis; c.el.style.visibility = vis; }
        if (vis) continue;
        c.el.style.transform = `translate3d(${fx(x)}px,${fx(y)}px,${fx(z)}px) rotateX(${fx(c.rx * k, 2)}deg) rotateY(${fx(c.ry * k, 2)}deg) rotateZ(${fx(c.rz * k, 2)}deg)`;
        c.el.style.opacity = fx(o, 3);
      }

      const d = easeOut(range(p, 0.6, 0.9));
      shot.style.opacity = fx(range(p, 0.58, 0.66), 3);
      shot.style.transform = `translate3d(0,${fx(mix(env.vh * 0.42, L.y, d))}px,${fx(mix(-2600 * L.zs, 0, d))}px) rotateX(${fx(mix(62, 0, d), 2)}deg) rotateZ(${fx(mix(-12, 0, d), 2)}deg)`;

      for (const c of caps) {
        const fin = range(p, c.a, c.a + 0.05);
        const fout = range(p, c.b - 0.05, c.b);
        const o = Math.min(fin, 1 - fout);
        if (Math.abs(o - c.o) < 0.001 && !force) continue;
        c.o = o;
        c.el.style.opacity = fx(o, 3);
        c.el.style.transform = `translate3d(0,${fx((1 - fin) * 40 - fout * 40)}px,0)`;
      }
      return sm !== target;
    },
  });
}

/* ---------- playtime race ---------- */
export function raceScene() {
  const el = $('#playtime');
  const fill = $('#race-fill');
  const val = $('#race-val');
  const END = 252 / 270; // 4h 12m on a 4.5h axis
  let sm = 0, mins = -1, done = false;
  const setDone = (d) => { if (d !== done) { done = d; el.classList.toggle('done', d); } };

  return scene({
    el,
    update(s, force) {
      if (env.reduce) { val.textContent = '4h 12m'; setDone(true); return false; }
      const t = progress(s);
      sm = follow(sm, t, 0.14);
      const r = ease(range(sm, 0.1, 0.78));
      fill.style.transform = `translate3d(${fx((r * END - 1) * 100, 2)}%,0,0)`;
      const m = Math.round(r * 252);
      if (m !== mins || force) { mins = m; val.textContent = `${Math.floor(m / 60)}h ${String(m % 60).padStart(2, '0')}m`; }
      setDone(r > 0.995);
      return sm !== t;
    },
  });
}

/* ---------- horizontal gallery (pinned on desktop, swipe carousel on phones) ---------- */
export function galleryScene() {
  const el = $('#gallery');
  const track = $('#g-track');
  const panels = $$('.g-panel', track);
  const num = $('#g-num');
  const dots = $$('.g-dots i', el);
  let flat = false, centers = [], sm = 0, lastG = -1, idx = 0, step = 1;

  const setIdx = (i) => {
    if (i === idx) return;
    idx = i;
    num.textContent = `0${i + 1}`;
    dots.forEach((d, j) => d.classList.toggle('on', j === i));
  };
  track.addEventListener('scroll', () => { if (flat) setIdx(clamp(Math.round(track.scrollLeft / step), 0, panels.length - 1)); }, { passive: true });
  const go = (dir) => track.scrollTo({ left: clamp(idx + dir, 0, panels.length - 1) * step, behavior: env.reduce ? 'auto' : 'smooth' });
  $('#g-prev').addEventListener('click', () => go(-1));
  $('#g-next').addEventListener('click', () => go(1));

  return scene({
    el,
    measure() {
      flat = env.vw < 820 || env.reduce;
      if (flat) {
        track.style.transform = '';
        for (const p of panels) { p.style.transform = ''; p.style.opacity = ''; }
        step = panels[1].offsetLeft - panels[0].offsetLeft;
        return;
      }
      centers = panels.map((p) => p.offsetLeft + p.offsetWidth / 2);
      lastG = -1;
    },
    update(s, force) {
      if (flat) return false;
      const t = progress(s);
      sm = follow(sm, t, 0.12);
      const g = ease(range(sm, 0.05, 0.95));
      if (g === lastG && !force) return sm !== t;
      lastG = g;
      const half = env.vw / 2;
      const tx = mix(half - centers[0], half - centers[centers.length - 1], g);
      track.style.transform = `translate3d(${fx(tx)}px,0,0)`;
      let best = 0, bd = 1e9;
      panels.forEach((p, i) => {
        const d = (centers[i] + tx - half) / env.vw;
        const ad = Math.abs(d);
        if (ad < bd) { bd = ad; best = i; }
        p.style.transform = `rotateY(${fx(clamp(-d * 34, -40, 40), 2)}deg) scale(${fx(1 - Math.min(0.14, ad * 0.18), 4)})`;
        p.style.opacity = fx(1 - Math.min(0.6, ad * 0.75), 3);
      });
      setIdx(best);
      return sm !== t;
    },
  });
}

/* ---------- stores marquee: two rows, opposite directions, faster when you scroll faster ---------- */
export function marqueeScene() {
  const el = $('#stores');
  const rows = [$('#m1'), $('#m2')];
  const STORES = [['Steam', 'b-steam'], ['Epic Games', 'b-epic'], ['Ubisoft Connect', 'b-ubisoft'], ['GOG', 'b-gog'], ['EA app', 'b-ea'], ['Any .exe', 'i-terminal']];
  const PERKS = [['Real playtime', 'i-clock'], ['Auto-detect', 'i-folder'], ['Launch from the tray', 'i-inbox'], ['Cover art', 'i-sparkles'], ['Daily backups', 'i-database'], ['Signed updates', 'i-shield'], ['No telemetry', 'i-eye-off']];
  const set = (list) => `<div class="mset">${list.map(([n, ic]) => `<span><svg class="i"><use href="#${ic}"/></svg>${n}</span>`).join('')}</div>`;
  rows[0].innerHTML = set(STORES).repeat(4);
  rows[1].innerHTML = set(PERKS).repeat(4);
  let w0 = 1, w1 = 1, x = 0;

  return scene({
    el,
    measure() {
      w0 = rows[0].firstElementChild.offsetWidth || 1;
      w1 = rows[1].firstElementChild.offsetWidth || 1;
    },
    update() {
      if (env.reduce) return false;
      x += (0.5 + Math.min(14, Math.abs(env.vel) * 0.45)) * (env.dt / 16.7);
      rows[0].style.transform = `translate3d(${fx(-(x % w0))}px,0,0)`;
      rows[1].style.transform = `translate3d(${fx((x % w1) - w1)}px,0,0)`;
      return true;
    },
  });
}

/* ---------- how it works: the progress line draws itself ---------- */
export function stepsScene() {
  const el = $('#steps');
  const line = $('#steps-line');
  const steps = $$('.step', el);
  let sm = 0, vertical = false, lit = -1;

  return scene({
    el,
    measure() { vertical = env.vw < 760; },
    update(s, force) {
      const t = env.reduce ? 1 : clamp((env.y + env.vh * 0.72 - s.top) / (s.height * 0.9));
      sm = env.reduce ? 1 : follow(sm, t, 0.12);
      line.style.transform = vertical ? `scaleY(${fx(sm, 4)})` : `scaleX(${fx(sm, 4)})`;
      const n = sm >= 0.98 ? 3 : sm >= 0.5 ? 2 : sm > 0.02 ? 1 : 0;
      if (n !== lit || force) { lit = n; steps.forEach((st, i) => st.classList.toggle('on', i < n)); }
      return sm !== t;
    },
  });
}

/* ---------- Dynamic Island: compact while reading, full at the top or when held ---------- */
export function chromeScene() {
  const island = $('#nav');
  const ring = $('#ring');
  const marks = $$('section[data-label]');
  let lastP = -1, compact = null, hover = false, focus = false, leaveTimer = 0, tops = [], label = '';

  const sync = () => {
    const c = env.y > 120 && !hover && !focus && !island.classList.contains('open');
    if (c !== compact) { compact = c; island.classList.toggle('compact', c); }
  };
  if (env.fine) {
    island.addEventListener('pointerenter', () => { clearTimeout(leaveTimer); hover = true; sync(); });
    island.addEventListener('pointerleave', () => { leaveTimer = setTimeout(() => { hover = false; sync(); }, 260); });
  }
  // Keyboard focus keeps the island open; a tap that leaves focus on a link does not.
  island.addEventListener('focusin', () => { focus = !!island.querySelector(':focus-visible'); sync(); });
  island.addEventListener('focusout', (e) => { if (!island.contains(e.relatedTarget)) { focus = false; sync(); } });
  island.addEventListener('island:change', sync);

  return scene({
    measure() { tops = marks.map((m) => [m.getBoundingClientRect().top + env.y, m.dataset.label]); },
    update(_, force) {
      const p = clamp(env.y / Math.max(1, env.docH - env.vh));
      if (p !== lastP || force) {
        lastP = p;
        ring.style.strokeDashoffset = fx(100 - p * 100, 2);
        // live activity: the section crossing the upper part of the screen
        const line = env.y + env.vh * 0.45;
        let next = 'Overview';
        for (const [top, name] of tops) if (top <= line) next = name;
        if (next !== label) { label = next; island.dispatchEvent(new CustomEvent('island:label', { detail: next })); }
      }
      sync();
      return false;
    },
  });
}

/* ---------- desktop cursor glow ---------- */
export function cursorScene() {
  if (!env.fine || env.reduce) return;
  const glow = $('#cursor-glow');
  let x = 0, y = 0, tx = 0, ty = 0, shown = false;
  addEventListener('pointermove', (e) => {
    tx = e.clientX;
    ty = e.clientY;
    if (!shown) { shown = true; x = tx; y = ty; glow.classList.add('on'); }
    wake();
  }, { passive: true });
  document.documentElement.addEventListener('pointerleave', () => { shown = false; glow.classList.remove('on'); });
  scene({
    update() {
      if (!shown) return false;
      x = follow(x, tx, 0.16);
      y = follow(y, ty, 0.16);
      glow.style.transform = `translate3d(${fx(x)}px,${fx(y)}px,0)`;
      return x !== tx || y !== ty;
    },
  });
}
