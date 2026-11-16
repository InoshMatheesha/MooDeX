// grid.ts - Virtualized, keyed card grid: only visible rows exist, cards are patched in place (spec 12.3).
import { Virtualizer, elementScroll, observeElementOffset, observeElementRect } from "@tanstack/virtual-core";
import { h } from "./dom";

export interface GridOptions {
  label: string;
  minCardWidth: number;
  gap: number;
  infoHeight: number;
  // Create a card for a key, or update `existing` in place.
  render: (key: string, existing?: HTMLElement) => HTMLElement;
  onActivate?: (key: string) => void;
  onEndReached?: () => void;
}

export class VirtualGrid {
  readonly host: HTMLElement;
  private space: HTMLElement;
  private keys: string[] = [];
  private mounted = new Map<string, HTMLElement>();
  private cols = 1;
  private cardW = 200;
  private rowH = 200;
  private virt: Virtualizer<HTMLElement, Element>;
  private cleanup: () => void;
  private ro: ResizeObserver;
  private io: IntersectionObserver;
  private pendingFocus: number | null = null;
  // First screen of cards fades in with a short stagger; scrolling never animates.
  private introUntil = 0;
  private introCount = 0;

  constructor(private opts: GridOptions) {
    this.space = h("div", { class: "grid-space", role: "list", "aria-label": opts.label });
    this.host = h("div", { class: "grid-host", tabindex: -1 }, this.space);
    // Covers load only when their card is actually on screen, in scroll order.
    this.io = new IntersectionObserver(
      (entries) => {
        for (const en of entries) {
          if (!en.isIntersecting) continue;
          const img = en.target as HTMLImageElement;
          const src = img.dataset.src;
          if (src && img.getAttribute("src") !== src) img.src = src;
          this.io.unobserve(img);
        }
      },
      { root: this.host, rootMargin: "200px 0px" },
    );
    this.virt = new Virtualizer<HTMLElement, Element>({
      count: 0,
      getScrollElement: () => this.host,
      estimateSize: () => this.rowH,
      overscan: 3,
      paddingStart: 0,
      observeElementRect,
      observeElementOffset,
      scrollToFn: elementScroll,
      onChange: () => this.paint(),
    });
    this.cleanup = this.virt._didMount();
    this.ro = new ResizeObserver(() => this.layout());
    this.ro.observe(this.host);
    this.host.addEventListener("keydown", (e) => this.onKey(e));
    this.host.addEventListener("click", (e) => {
      const card = (e.target as HTMLElement).closest<HTMLElement>(".card");
      if (card?.dataset.key && !(e.target as HTMLElement).closest("button")) this.opts.onActivate?.(card.dataset.key);
    });
  }

  // Lazy covers: set data-src and let the observer swap it in.
  observeImage(img: HTMLImageElement): void {
    this.io.observe(img);
  }

  setKeys(keys: string[]): void {
    if (this.keys.length === 0 && keys.length > 0 && !window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      this.introUntil = performance.now() + 600;
      this.introCount = 0;
    }
    this.keys = keys;
    const keep = new Set(keys);
    for (const [k, el] of this.mounted) {
      if (!keep.has(k)) {
        el.remove();
        this.mounted.delete(k);
      }
    }
    this.layout();
  }

  // Patch only these cards; no list rebuild (fixes v1's Reset flicker).
  patch(keys: string[]): void {
    for (const k of keys) {
      const el = this.mounted.get(k);
      if (el) this.opts.render(k, el);
    }
  }

  get count(): number {
    return this.keys.length;
  }

  destroy(): void {
    this.cleanup();
    this.ro.disconnect();
    this.io.disconnect();
    this.host.remove();
  }

  private layout(): void {
    const style = getComputedStyle(this.host);
    const padX = parseFloat(style.paddingLeft) + parseFloat(style.paddingRight);
    const width = Math.max(0, this.host.clientWidth - padX);
    const { minCardWidth: min, gap, infoHeight } = this.opts;
    this.cols = Math.max(1, Math.floor((width + gap) / (min + gap)));
    this.cardW = Math.floor((width - gap * (this.cols - 1)) / this.cols);
    // Fixed card size: cover is 16:9, nothing reflows when images arrive.
    const cardH = Math.round((this.cardW * 9) / 16) + infoHeight;
    this.rowH = cardH + gap;
    this.space.style.setProperty("--card-w", `${this.cardW}px`);
    this.space.style.setProperty("--card-h", `${cardH}px`);
    const rows = Math.ceil(this.keys.length / this.cols);
    this.virt.setOptions({ ...this.virt.options, count: rows, estimateSize: () => this.rowH });
    this.virt.measure();
    this.virt._willUpdate();
    this.paint();
  }

  private paint(): void {
    const total = this.virt.getTotalSize();
    this.space.style.height = `${Math.max(0, total - this.opts.gap)}px`;
    const items = this.virt.getVirtualItems();
    const visible = new Set<string>();
    for (const row of items) {
      for (let c = 0; c < this.cols; c++) {
        const idx = row.index * this.cols + c;
        const key = this.keys[idx];
        if (key === undefined) break;
        visible.add(key);
        let el = this.mounted.get(key);
        if (!el) {
          el = this.opts.render(key);
          el.dataset.key = key;
          if (performance.now() < this.introUntil) {
            el.classList.add("card-in");
            el.style.setProperty("--i", String(Math.min(this.introCount++, 16)));
            el.addEventListener("animationend", () => el?.classList.remove("card-in"), { once: true });
          }
          el.setAttribute("role", "listitem");
          this.mounted.set(key, el);
        }
        el.dataset.index = String(idx);
        el.setAttribute("aria-posinset", String(idx + 1));
        el.setAttribute("aria-setsize", String(this.keys.length));
        el.style.transform = `translate(${c * (this.cardW + this.opts.gap)}px, ${row.start}px)`;
        if (el.parentElement !== this.space) this.space.append(el);
      }
    }
    for (const [k, el] of this.mounted) {
      if (!visible.has(k) && !el.contains(document.activeElement)) {
        el.remove();
        this.mounted.delete(k);
      }
    }
    const last = items[items.length - 1];
    if (last && last.index >= Math.ceil(this.keys.length / this.cols) - 2) this.opts.onEndReached?.();
    if (this.pendingFocus !== null) {
      const k = this.keys[this.pendingFocus];
      const el = k ? this.mounted.get(k) : undefined;
      if (el) {
        el.focus({ preventScroll: true });
        this.pendingFocus = null;
      }
    }
  }

  // Arrow keys move through the grid; Enter opens (spec 12.4).
  private onKey(e: KeyboardEvent): void {
    const card = (e.target as HTMLElement).closest<HTMLElement>(".card");
    if (!card || card.parentElement !== this.space) return;
    const idx = Number(card.dataset.index ?? 0);
    let next = idx;
    switch (e.key) {
      case "ArrowRight": next = idx + 1; break;
      case "ArrowLeft": next = idx - 1; break;
      case "ArrowDown": next = idx + this.cols; break;
      case "ArrowUp": next = idx - this.cols; break;
      case "Home": next = 0; break;
      case "End": next = this.keys.length - 1; break;
      case "Enter":
      case " ":
        if (e.target === card && card.dataset.key) {
          e.preventDefault();
          this.opts.onActivate?.(card.dataset.key);
        }
        return;
      default:
        return;
    }
    e.preventDefault();
    next = Math.max(0, Math.min(this.keys.length - 1, next));
    this.pendingFocus = next;
    this.virt.scrollToIndex(Math.floor(next / this.cols), { align: "auto" });
    this.paint();
  }
}
