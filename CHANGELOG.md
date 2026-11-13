# Changelog

## 2.0.5 - unreleased

### Fixed
- A stray icon (the hidden duplicates button) showed beside the search box in My PC Games. Elements marked hidden now always stay hidden.

## 2.0.4

### Added
- **In-app updates.** Go to Settings > Updates and click **Check for updates**. If a newer version is out, click **Download & install**: MooDeX downloads it from GitHub Releases, checks its signature, closes any running play sessions cleanly, installs it and restarts. MooDeX also checks quietly at startup; you can turn that off.

## 2.0.3

### Changed
- **New logo.** New MooDeX logo for the app icon (corners are now transparent) and the sidebar wordmark.
- **Last played in the sidebar.** When no game is running, the sidebar shows your last played game instead of "Nothing running".
- **Artwork is never cropped.** The game and Discover dialogs show the whole picture over a blurred copy of itself, instead of cutting it to a thin strip.

## 2.0.2

### Added
- **Auto-detect games.** In Settings > Auto-detect games, add the folders your games install into (Steam libraries included). MooDeX finds each game's exe, matches a RAWG cover, and keeps watching the folders so new installs appear on their own. A game you delete is not re-added.
- **Play from the card.** Installed games have a Play button right on their card, so there's no need to open the game first.

### Changed
- **Status names.** "Backlog" is now **Want to Play** and "Dropped" is now **Stopped**.
- **Game details fit on one screen.** The title sits on a shorter artwork banner, with the last 3 sessions shown and a "Show all" button for the rest.
- **v1 import moved.** It is now in Settings and no longer appears as a banner on the Dashboard.
- **Faster covers.** Thumbnails come from RAWG's pre-resized copies and are compressed more, so they are much smaller to download. Thumbnails no longer wait for the large artwork.

### Fixed
- Discover card titles were covered.
- Long game names in the sidebar's "Playing now" spilled outside the sidebar.

## 2.0.1

### Changed
- Redesigned Dashboard: colour-coded status cards (click to open the filtered library) with a library-mix bar, Total Games and Playtime cards with icon chips, a G.O.A.T. card drawn over the game's artwork, a ranked Most Played podium (gold, silver, bronze) showing each game's share of all playtime, a live "Playing now" strip and a Recently Played row.
- Status colours: Playing green, Completed gold, Backlog purple, Dropped red.
- PC game titles now come from the folder when it spells out an abbreviated exe name (OnimushaWotS.exe -> "Onimusha Way of the Sword").

## 2.0.0

This is a full rewrite of MooDeX in Rust with Tauri 2. It replaces v1.3, which was built on C#, WPF and .NET.

### Added
- **Accurate playtime.** Playtime is now the sum of individual play sessions, and you can see each session in a game's history. When a game is started by Steam, Epic or Ubisoft, MooDeX tracks the real game rather than the launcher that opens it.
- **Correct game detection.** A game counts as running only when its full executable path matches. In v1, part of a name was enough, so Steam's overlay or Game Bar could add playtime to the wrong game.
- **Sleep and crash recovery.** If the PC sleeps, the time asleep is not counted. If MooDeX crashes, the open session ends at the last moment the game was known to be running.
- **Safer library storage.** Your library is a SQLite database that survives power cuts. MooDeX also keeps a daily backup for seven days and can export and import your library as JSON.
- **Duplicate suggestions.** MooDeX suggests games that may be duplicates. It never merges anything unless you choose to.
- **v1 import.** MooDeX shows you the import plan before applying anything. It backs up your v1 file first and asks you to confirm each merge.
- **Faster covers.** Covers are cached in memory and on disk as WebP files. At most six downloads run at once, and covers load only as they scroll into view.
- **Discover works offline.** Discover shows cached results immediately and refreshes them in the background, so it works without a connection. Searches are cancelled as you keep typing.
- **Smoother large libraries.** The game grid only draws the cards on screen and updates individual cards in place, so it no longer flickers when the library changes.
- **Diagnostics pane.** Settings shows the version, build, and data and log folders, plus the last 10 warnings or errors. Logs are kept for seven days.

### Security
- **No key in the app.** The RAWG API key is no longer built into the app. Discover uses a proxy that holds the key on the server, or your own key stored in Windows Credential Manager. The v1 key (`3e485ee2...`) must be revoked.

### Changed
- **One game list.** v1 kept separate "games" and "local_games" lists. They are now one list, and "My PC Games" is a filter on it.
- **Ratings use 1-5 stars.** Ratings imported from v1's 10-point scale are converted.
- **Separate data folder.** Data is stored in `%LOCALAPPDATA%\MooDeX\v2`. The v1 files next to it are only read, never changed.
