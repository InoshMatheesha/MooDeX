// ui.ts - Toasts (non-blocking, dismissible) and native <dialog> modals (spec 12.4).
import { h, icon, I } from "./dom";
import type { ApiError } from "./api";

let region: HTMLElement | null = null;

// Inline, dismissible, non-blocking messages replace v1's MessageBox.
export function toast(message: string, kind: "info" | "ok" | "error" = "info", ms = 4500): void {
  if (!region) {
    region = h("div", { class: "toasts", role: "status", "aria-live": "polite" });
    document.body.append(region);
  }
  const close = h("button", { "aria-label": "Dismiss", onclick: () => t.remove() }, "×");
  const t = h("div", { class: `toast ${kind}` }, h("p", {}, message), close);
  region.append(t);
  if (ms > 0) window.setTimeout(() => t.remove(), kind === "error" ? ms * 2 : ms);
}

export function toastError(e: ApiError | unknown, prefix = ""): void {
  const msg = e && typeof e === "object" && "message" in e ? String((e as ApiError).message) : String(e);
  if ((e as ApiError)?.kind === "cancelled") return;
  toast(prefix ? `${prefix}: ${msg}` : msg, "error");
}

export interface Modal {
  el: HTMLDialogElement;
  body: HTMLElement;
  foot: HTMLElement;
  close: (value?: unknown) => void;
  closed: Promise<unknown>;
}

// showModal() makes the page inert and traps focus; Esc closes.
export function modal(title: string, opts: { size?: "wide" | "narrow"; hero?: HTMLElement; bare?: boolean } = {}): Modal {
  const body = h("div", { class: "m-body" });
  const foot = h("div", { class: "m-foot" });
  const closeBtn = h("button", { class: "btn icon ghost", "aria-label": "Close" }, icon(I.close));
  const head = h("div", { class: "m-head" }, h("h2", { id: "m-title" }, title), closeBtn);
  // Bare dialogs draw their own title (the game dialog puts it on the artwork).
  const label = opts.bare ? { "aria-label": title } : { "aria-labelledby": "m-title" };
  const el = h("dialog", { class: `modal ${opts.size ?? ""}`, ...label }, h("div", { class: "m-wrap" }, opts.hero ?? null, opts.bare ? null : head, body, foot));
  let result: unknown;
  let resolve: (v: unknown) => void = () => {};
  const closed = new Promise<unknown>((r) => (resolve = r));
  const close = (v?: unknown) => {
    result = v;
    if (el.open) el.close();
  };
  closeBtn.addEventListener("click", () => close());
  el.addEventListener("close", () => {
    resolve(result);
    el.remove();
  });
  // Click on the backdrop closes.
  el.addEventListener("mousedown", (e) => {
    if (e.target === el) close();
  });
  document.body.append(el);
  el.showModal();
  return { el, body, foot, close, closed };
}

// Promise-based yes/no confirmation.
export async function confirmBox(title: string, message: string, ok = "Confirm", danger = false): Promise<boolean> {
  const m = modal(title, { size: "narrow" });
  m.body.append(h("p", {}, message));
  const yes = h("button", { class: `btn ${danger ? "danger" : "primary"}`, onclick: () => m.close(true) }, ok);
  m.foot.append(h("button", { class: "btn ghost", onclick: () => m.close(false) }, "Cancel"), yes);
  yes.focus();
  return (await m.closed) === true;
}
