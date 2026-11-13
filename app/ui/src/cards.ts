// cards.ts - Game and Discover cards; built once, patched in place by key.
import { imgUrl, type DiscoverGame, type GameView } from "./api";
import { h, icon, I, initials, STATUS_LABEL, stars } from "./dom";

// Cover with skeleton; the image is attached lazily via data-src.
function coverEl(key: string | null, title: string, observe: (img: HTMLImageElement) => void): HTMLElement {
  const cover = h("div", { class: "cover skeleton" });
  const src = imgUrl(key, "thumb");
  if (src) {
    const img = h("img", { alt: "", decoding: "async", draggable: "false" });
    img.dataset.src = src;
    img.addEventListener("load", () => {
      img.classList.add("loaded");
      cover.classList.remove("skeleton");
    });
    img.addEventListener("error", () => {
      cover.classList.remove("skeleton");
      img.remove();
      cover.prepend(h("div", { class: "ph" }, initials(title)));
    });
    cover.append(img);
    observe(img);
  } else {
    cover.classList.remove("skeleton");
    cover.append(h("div", { class: "ph" }, initials(title)));
  }
  return cover;
}

function badgesFor(g: GameView): HTMLElement {
  const b = h("div", { class: "badges" });
  if (g.running_since) b.append(h("span", { class: "badge live" }, h("span", { class: "dot" }), "Playing now"));
  else b.append(h("span", { class: `badge ${g.status}` }, h("span", { class: "dot" }), STATUS_LABEL[g.status]));
  if (g.install_missing) b.append(h("span", { class: "badge warn", title: "The linked executable is missing" }, "Missing"));
  if (g.is_goat) b.append(h("span", { class: "badge goat", title: "Greatest of all time" }, "G.O.A.T."));
  return b;
}

// Signature of what a card shows; unchanged signature means no DOM work.
function sig(g: GameView): string {
  return [g.title, g.cover_key, g.status, g.rating, g.playtime, g.is_goat, g.install_missing, !!g.running_since, g.installed].join("|");
}

export type PlayFn = (g: GameView, btn: HTMLButtonElement) => void;

// Round Play button on installed covers; launches without opening the game.
function playButton(g: GameView, onPlay?: PlayFn): HTMLElement | null {
  if (!onPlay || !g.installed) return null;
  const running = !!g.running_since;
  const b = h(
    "button",
    {
      class: `card-play ${running ? "running" : ""}`,
      "aria-label": running ? `${g.title} is running` : `Play ${g.title}`,
      title: running ? "Running" : g.install_missing ? "Executable missing - open to re-link" : "Play",
      disabled: running,
    },
    running ? h("span", { class: "live-dot" }) : icon(I.play),
  ) as HTMLButtonElement;
  b.addEventListener("click", (e) => {
    e.stopPropagation();
    onPlay(g, b);
  });
  b.addEventListener("keydown", (e) => e.stopPropagation());
  return b;
}

export function gameCard(g: GameView, observe: (img: HTMLImageElement) => void, existing?: HTMLElement, onPlay?: PlayFn): HTMLElement {
  const s = sig(g) + (onPlay ? "|p" : "");
  if (existing && existing.dataset.sig === s) return existing;
  const label = `${g.title}, ${STATUS_LABEL[g.status]}, ${g.playtime} played`;
  const cover = coverEl(g.cover_key, g.title, observe);
  cover.append(badgesFor(g));
  const play = playButton(g, onPlay);
  if (play) cover.append(play);
  const info = h(
    "div",
    { class: "info" },
    h("h3", { title: g.title }, g.title),
    h("div", { class: "meta" }, h("span", {}, g.playtime), g.rating ? h("span", { class: "stars", "aria-label": `${g.rating} of 5 stars` }, stars(g.rating)) : null),
  );
  const el = existing ?? h("article", { class: "card", tabindex: 0 });
  // Keep the cover element if its image is unchanged, to avoid refetch flicker.
  if (existing && existing.dataset.cover === (g.cover_key ?? "")) {
    const oldCover = existing.querySelector(".cover");
    oldCover?.querySelector(".badges")?.replaceWith(badgesFor(g));
    oldCover?.querySelector(".card-play")?.remove();
    const p2 = playButton(g, onPlay);
    if (p2) oldCover?.append(p2);
    existing.querySelector(".info")?.replaceWith(info);
  } else {
    el.replaceChildren(cover, info);
  }
  el.dataset.sig = s;
  el.dataset.cover = g.cover_key ?? "";
  el.setAttribute("aria-label", label);
  return el;
}

export function discoverCard(
  g: DiscoverGame,
  observe: (img: HTMLImageElement) => void,
  onAdd: (g: DiscoverGame, btn: HTMLButtonElement) => void,
  existing?: HTMLElement,
): HTMLElement {
  const s = `${g.id}|${g.library_id ?? ""}`;
  if (existing && existing.dataset.sig === s) return existing;
  const cover = coverEl(g.cover_key, g.name, observe);
  if (g.library_id) cover.append(h("div", { class: "badges" }, h("span", { class: "badge owned" }, h("span", { class: "dot" }), "In library")));
  const year = g.released ? g.released.slice(0, 4) : "TBA";
  const genre = g.genres.slice(0, 2).join(", ");
  const add = h(
    "button",
    { class: "btn sm add", "aria-label": `Add ${g.name} to library`, disabled: !!g.library_id },
    g.library_id ? "Added" : "+ Add",
  ) as HTMLButtonElement;
  add.addEventListener("click", (e) => {
    e.stopPropagation();
    onAdd(g, add);
  });
  const info = h("div", { class: "info" }, h("h3", { title: g.name }, g.name), h("div", { class: "meta" }, h("span", {}, year), genre ? h("span", {}, genre) : null, add));
  const el = existing ?? h("article", { class: "card", tabindex: 0 });
  el.replaceChildren(cover, info);
  el.dataset.sig = s;
  el.setAttribute("aria-label", `${g.name}, ${year}`);
  return el;
}
