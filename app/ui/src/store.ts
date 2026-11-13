// store.ts - Render cache keyed by GameId, hydrated by commands, patched by events (spec 12.2).
import { listen } from "@tauri-apps/api/event";
import { api, type GameId, type GameView, type LibraryEvent, type Settings } from "./api";

type ChangeFn = (ids: GameId[]) => void;

const cache = new Map<GameId, GameView>();
const changeListeners = new Set<ChangeFn>();
const running = new Set<GameId>();
const runningListeners = new Set<() => void>();
const settingsListeners = new Set<(s: Settings) => void>();

export const games = {
  get: (id: GameId) => cache.get(id),
  has: (id: GameId) => cache.has(id),
  running: () => [...running],
  onChange(fn: ChangeFn): () => void {
    changeListeners.add(fn);
    return () => changeListeners.delete(fn);
  },
  onRunning(fn: () => void): () => void {
    runningListeners.add(fn);
    return () => runningListeners.delete(fn);
  },
  onSettings(fn: (s: Settings) => void): () => void {
    settingsListeners.add(fn);
    return () => settingsListeners.delete(fn);
  },
  // Fetch views for ids not yet cached (or all when forced).
  async ensure(ids: GameId[], force = false): Promise<void> {
    const need = force ? ids : ids.filter((id) => !cache.has(id));
    if (need.length === 0) return;
    for (let i = 0; i < need.length; i += 400) {
      const chunk = need.slice(i, i + 400);
      const views = await api.getGames(chunk);
      for (const v of views) cache.set(v.id, v);
    }
  },
  // Re-hydrate exactly these ids; ids that came back empty were deleted.
  async refresh(ids: GameId[]): Promise<void> {
    if (ids.length === 0) return;
    const views = await api.getGames(ids);
    const seen = new Set<GameId>();
    for (const v of views) {
      cache.set(v.id, v);
      seen.add(v.id);
      if (v.running_since) running.add(v.id);
      else running.delete(v.id);
    }
    for (const id of ids) {
      if (!seen.has(id)) {
        cache.delete(id);
        running.delete(id);
      }
    }
    changeListeners.forEach((fn) => fn(ids));
    runningListeners.forEach((fn) => fn());
  },
};

// Coalesce bursts of events into one refresh per frame.
const pending = new Set<GameId>();
let scheduled = false;
function queue(ids: GameId[]): void {
  ids.forEach((id) => pending.add(id));
  if (scheduled) return;
  scheduled = true;
  requestAnimationFrame(() => {
    scheduled = false;
    const batch = [...pending];
    pending.clear();
    games.refresh(batch).catch((e) => console.error("refresh failed", e));
  });
}

export async function startStore(): Promise<void> {
  const ids = await api.running();
  await games.ensure(ids, true);
  ids.forEach((id) => running.add(id));
  await listen<LibraryEvent>("library-event", (e) => {
    const ev = e.payload;
    switch (ev.type) {
      case "games_changed":
        queue(ev.ids);
        break;
      case "session_started":
      case "session_ended":
      case "playtime_tick":
      case "cover_ready":
        queue([ev.game_id]);
        break;
      case "settings_changed":
        settingsListeners.forEach((fn) => fn(ev.settings));
        break;
      default:
        break;
    }
  });
  await listen("library-resync", () => queue([...cache.keys()]));
}
