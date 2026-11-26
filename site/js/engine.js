// engine.js - One requestAnimationFrame loop for every scroll-driven effect.
// Scroll is read once per frame; section sizes are cached and re-measured only on resize.
// The loop sleeps when nothing is moving and wakes on scroll, resize or wake().

export const $ = (s, el = document) => el.querySelector(s);
export const $$ = (s, el = document) => [...el.querySelectorAll(s)];
export const clamp = (x, a = 0, b = 1) => (x < a ? a : x > b ? b : x);
export const mix = (a, b, t) => a + (b - a) * t;
export const range = (p, a, b) => clamp((p - a) / (b - a));
export const ease = (x) => (x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2);
export const easeOut = (x) => 1 - Math.pow(1 - x, 3);
export const easeIn = (x) => x * x * x;

const mq = (q) => matchMedia(q);
export const env = {
  reduce: mq('(prefers-reduced-motion: reduce)').matches,
  fine: mq('(hover: hover) and (pointer: fine)').matches,
  touch: mq('(pointer: coarse)').matches,
  y: 0, vw: 0, vh: 0, vel: 0, dy: 0, dt: 16.7, time: 0, docH: 1,
};

const scenes = [];
let raf = 0;
let last = 0;

/** Frame-rate independent smoothing toward a target. Snaps when close. */
export function follow(cur, target, k = 0.14) {
  const f = 1 - Math.pow(1 - k, env.dt / 16.7);
  const next = cur + (target - cur) * f;
  return Math.abs(target - next) < 1e-4 ? target : next;
}

/** Progress (0..1) through a pinned section: 0 when its top reaches the viewport top, 1 when its bottom reaches the viewport bottom. */
export const progress = (s) => clamp((env.y - s.top) / Math.max(1, s.height - env.vh));

/**
 * Register a scene. def = { el?, margin?, measure?(s), update(s) -> bool, onActive?(on, s) }.
 * update runs only while el is near the viewport; returning true asks for another frame.
 */
export function scene(def) {
  const s = { active: !def.el, top: 0, height: 0, ...def };
  scenes.push(s);
  if (def.el) {
    new IntersectionObserver(([e]) => {
      s.active = e.isIntersecting;
      def.el.classList.toggle('is-on', s.active);
      s.onActive?.(s.active, s);
      wake();
    }, { rootMargin: def.margin || '15% 0px' }).observe(def.el);
  }
  return s;
}

export function measure() {
  env.vw = document.documentElement.clientWidth;
  env.vh = innerHeight;
  env.y = scrollY;
  for (const s of scenes) {
    if (s.el) {
      const r = s.el.getBoundingClientRect();
      s.top = r.top + env.y;
      s.height = r.height;
    }
  }
  for (const s of scenes) s.measure?.(s);
  env.docH = document.documentElement.scrollHeight;
  wake(true);
}

function tick(t) {
  raf = 0;
  env.dt = last ? Math.min(64, t - last) : 16.7;
  last = t;
  env.time = t;
  const y = scrollY;
  env.dy = y - env.y;
  env.vel = mix(env.vel, env.dy * (16.7 / env.dt), 0.25);
  env.y = y;
  let busy = Math.abs(env.vel) > 0.05;
  for (const s of scenes) if (s.active) busy = s.update(s, forced) || busy;
  forced = false;
  if (busy) raf = requestAnimationFrame(tick);
  else last = 0;
}

let forced = false;
/** Ask for a frame. force = scenes should redraw even if their values did not change (after a resize). */
export function wake(force = false) {
  if (force) forced = true;
  if (!raf) raf = requestAnimationFrame(tick);
}

export function start() {
  addEventListener('scroll', () => wake(), { passive: true });
  let pending = 0;
  const remeasure = () => { if (!pending) pending = requestAnimationFrame(() => { pending = 0; measure(); }); };
  addEventListener('resize', remeasure);
  new ResizeObserver(remeasure).observe(document.body);
  document.fonts?.ready.then(remeasure);
  measure();
}
