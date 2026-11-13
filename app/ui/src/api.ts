// api.ts - Typed IPC surface; the frontend owns no state, it asks the actor (spec 12.2).
import { invoke } from "@tauri-apps/api/core";

export type Status = "playing" | "completed" | "backlog" | "dropped";
export type SortOrder = "title" | "recently_added" | "playtime" | "rating" | "last_played" | "release_date";
export type GameId = string;

export type LaunchTarget =
  | { kind: "Exe"; path: string; args: string[]; working_dir: string | null }
  | { kind: "Uri"; uri: string }
  | { kind: "Shortcut"; path: string };

export interface Install {
  launch: LaunchTarget;
  detect_exe: string | null;
}

export interface GameView {
  id: GameId;
  rawg_id: number | null;
  title: string;
  cover_url: string | null;
  cover_key: string | null;
  cover_cached: boolean;
  release_date: string | null;
  status: Status;
  rating: number | null;
  notes: string;
  is_goat: boolean;
  genres: string[];
  install: Install | null;
  installed: boolean;
  install_missing: boolean;
  added_at: string;
  updated_at: string;
  minutes: number;
  playtime: string;
  play_count: number;
  last_played: string | null;
  running_since: string | null;
  has_estimate: boolean;
}

export interface GameFilter {
  status?: Status | null;
  installed_only?: boolean;
  query?: string | null;
}

export interface GameDraft {
  title: string;
  rawg_id?: number | null;
  cover_url?: string | null;
  release_date?: string | null;
  status?: Status;
  rating?: number | null;
  notes?: string;
  genres?: string[];
  install?: Install | null;
}

export interface GamePatch {
  title?: string;
  status?: Status;
  rating?: number | null;
  notes?: string;
  cover_url?: string | null;
  rawg_id?: number | null;
  release_date?: string | null;
  genres?: string[];
}

export interface DashboardStats {
  total: number;
  playing: number;
  completed: number;
  backlog: number;
  dropped: number;
  installed: number;
  total_minutes: number;
  total_playtime: string;
  total_sessions: number;
  average_rating: number | null;
}

export interface DashboardView {
  stats: DashboardStats;
  most_played: GameView[];
  recently_played: GameView[];
  playing_now: GameView[];
  goat: GameView | null;
}

export interface SessionRecord {
  id: number;
  started_at: string;
  ended_at: string | null;
  minutes: number;
  source: "job_object" | "wmi" | "manual";
}

export interface DuplicateCandidate {
  a: GameId;
  b: GameId;
  reason: { kind: "same_title" } | { kind: "similar_title"; score: number };
}

export interface RawgGame {
  id: number;
  name: string;
  background_image: string | null;
  released: string | null;
  rating: number;
  metacritic: number | null;
  added: number;
  genres: string[];
}

export interface DiscoverGame extends RawgGame {
  cover_key: string | null;
  library_id: GameId | null;
}

export interface DiscoverPage {
  results: DiscoverGame[];
  has_next: boolean;
  count: number;
}

export interface ApiError {
  kind: string;
  message: string;
  retry_after_secs?: number;
}

export interface RawgGameDetail {
  id: number;
  name: string;
  description: string;
  background_image: string | null;
  released: string | null;
  rating: number;
  metacritic: number | null;
  playtime_hours: number;
  website: string | null;
  genres: string[];
  developers: string[];
  publishers: string[];
  platforms: string[];
}

export interface ExeMatch {
  title: string;
  candidates: DiscoverGame[];
  error: ApiError | null;
}

export interface PlayOutcome {
  tracked: boolean;
  method: string;
  note: string | null;
}

export interface Settings {
  minimize_to_tray: boolean;
  launch_at_startup: boolean;
  start_minimized: boolean;
  min_session_seconds: number;
  stub_threshold_seconds: number;
  image_cache_budget_mb: number;
  rawg_key_source: "proxy" | "user" | "none";
  rawg_proxy_url: string;
  backup_retention_days: number;
  v1_import_done: boolean;
  scan_folders: string[];
  scan_ignored: string[];
  auto_update_check: boolean;
}

export interface UpdateInfo {
  current: string;
  available: boolean;
  version: string | null;
  notes: string | null;
  date: string | null;
}

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
}

export interface ScanReport {
  scanned: number;
  added: string[];
  linked: string[];
  errors: string[];
}

export type ScanEvent = { type: "started" } | { type: "finished"; report: ScanReport };

export interface SettingsView {
  settings: Settings;
  rawg_key_set: boolean;
  rawg_active: string;
  autostart_registered: boolean;
}

export interface Diagnostics {
  version: string;
  build_hash: string;
  webview: string;
  data_dir: string;
  log_dir: string;
  games: number;
  installed: number;
  open_sessions: number;
  schema_version: number;
  detect_registry: number;
  recent_problems: string[];
}

export interface V1Status {
  path: string | null;
  already_imported: boolean;
}

export interface ImportGame {
  key: string;
  v1_id: number;
  draft: GameDraft & { status: Status };
  minutes: number;
  play_count: number;
  last_played: string | null;
  added_at: string;
}

export interface ProposedMerge {
  library_key: string;
  local_key: string;
  title: string;
  reason: string;
}

export interface ImportPlan {
  source_path: string;
  games: ImportGame[];
  merges: ProposedMerge[];
  warnings: { key: string | null; message: string }[];
  goat_key: string | null;
}

export interface ImportReport {
  imported: number;
  merged: number;
  skipped: number;
  sessions: number;
  warnings: string[];
}

export type LibraryEvent =
  | { type: "games_changed"; ids: GameId[] }
  | { type: "session_started"; game_id: GameId; at: string }
  | { type: "session_ended"; game_id: GameId; minutes: number }
  | { type: "playtime_tick"; game_id: GameId; minutes: number }
  | { type: "cover_ready"; game_id: GameId; hash: string }
  | { type: "scan_progress"; done: number; total: number }
  | { type: "settings_changed"; settings: Settings };

// Normalise any rejection into ApiError.
export function asError(e: unknown): ApiError {
  if (e && typeof e === "object" && "message" in e) {
    const o = e as { kind?: unknown; message: unknown; retry_after_secs?: number };
    return { kind: typeof o.kind === "string" ? o.kind : "error", message: String(o.message), retry_after_secs: o.retry_after_secs };
  }
  return { kind: "error", message: String(e) };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw asError(e);
  }
}

export const api = {
  listIds: (filter: GameFilter, sort: SortOrder) => call<GameId[]>("list_ids", { filter, sort }),
  getGames: (ids: GameId[]) => call<GameView[]>("get_games", { ids }),
  getGame: (id: GameId) => call<GameView | null>("get_game", { id }),
  addGame: (draft: GameDraft) => call<GameId>("add_game", { draft }),
  updateGame: (id: GameId, patch: GamePatch) => call<void>("update_game", { id, patch }),
  deleteGame: (id: GameId) => call<void>("delete_game", { id }),
  setInstall: (id: GameId, install: Install | null) => call<void>("set_install", { id, install }),
  setGoat: (id: GameId | null) => call<void>("set_goat", { id }),
  mergeGames: (keep: GameId, absorb: GameId) => call<void>("merge_games", { keep, absorb }),
  duplicates: () => call<DuplicateCandidate[]>("duplicates"),
  sessions: (id: GameId) => call<SessionRecord[]>("sessions", { id }),
  dashboard: () => call<DashboardView>("dashboard"),
  running: () => call<GameId[]>("running"),
  play: (id: GameId) => call<PlayOutcome>("play_game", { id }),
  scanNow: () => call<ScanReport>("scan_now"),
  checkUpdate: () => call<UpdateInfo>("check_update"),
  installUpdate: () => call<void>("install_update"),
  feed: (feed: "trending" | "upcoming" | "popular", page: number) => call<DiscoverPage>("discover_feed", { feed, page }),
  search: (query: string, page: number) => call<DiscoverPage>("discover_search", { query, page }),
  rawgDetails: (id: number) => call<RawgGameDetail>("rawg_details", { id }),
  matchTitle: (path: string | null, query: string | null) => call<ExeMatch>("match_title", { path, query }),
  addRawgGame: (game: RawgGame, status: Status) => call<GameId>("add_rawg_game", { game, status }),
  addPcGame: (path: string, title: string, game: RawgGame | null) => call<GameId>("add_pc_game", { path, title, game }),
  getSettings: () => call<SettingsView>("get_settings"),
  setSetting: (key: keyof Settings, value: unknown) => call<SettingsView>("set_setting", { key, value }),
  setRawgKey: (key: string | null) => call<SettingsView>("set_rawg_key", { key }),
  testRawg: () => call<number>("test_rawg"),
  diagnostics: () => call<Diagnostics>("diagnostics"),
  openFolder: (which: "logs" | "data" | "backups") => call<void>("open_folder", { which }),
  v1Status: () => call<V1Status>("v1_status"),
  v1Plan: (path: string | null) => call<ImportPlan>("v1_plan", { path }),
  v1Apply: (plan: ImportPlan, accepted: [string, string][]) => call<ImportReport>("v1_apply", { plan, accepted }),
  exportLibrary: (path: string) => call<number>("export_library", { path }),
  importLibrary: (path: string) => call<ImportReport>("import_library", { path }),
};

// Covers are served by the moodex-img protocol straight from the cache (spec 10).
export function imgUrl(key: string | null, variant: "thumb" | "hero"): string | null {
  return key ? `http://moodex-img.localhost/${key}/${variant}` : null;
}
