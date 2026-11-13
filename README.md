<p align="center"><img src="assets/logo-wordmark.png" alt="MooDeX" width="420"></p>

# MooDeX 2.0

MooDeX keeps every PC game you own in one place. It records how long you really play, adds artwork from the RAWG catalogue, and lets you rate games and keep notes. Everything stays on your PC.

This is the Rust rewrite of MooDeX, built with Tauri 2. The design contract it follows is [MOODEX_RUST_MVP.md](MOODEX_RUST_MVP.md).

## Requirements

- Windows 10 or 11, 64-bit.
- **Microsoft Edge WebView2 Runtime.** It ships with Windows 11 and is already on most Windows 10 PCs. The installer downloads it if it is missing, so the first install on such a PC needs an internet connection. **The runtime is not bundled.**

## How playtime tracking works

| How the game starts | How MooDeX tracks it |
|---|---|
| **Play** on a linked `.exe` | MooDeX starts the game inside a Windows *job object*. The session ends when the game and every process it started have exited. |
| **Play** on a Steam/Epic/Ubisoft URI or shortcut, with a **detection exe** set | The launcher exits right away. MooDeX then waits for the real game's executable to appear and tracks that instead. |
| The game was started outside MooDeX | MooDeX notices a linked executable starting, using WMI process events. |

- **Exact matching.** A process counts as a game only if its full, resolved path matches the linked path. A partial name is never enough.
- **Sessions are kept separately.** Every session is saved as its own record, and playtime is the sum of them.
- **Sleep and crashes.** A 30-second heartbeat lets MooDeX detect when the PC slept, so that time is not counted. If MooDeX crashes, the next start closes the open session at the last heartbeat.
- **Short sessions are dropped.** Sessions under 60 seconds are discarded. You can change this in Settings.
- **When WMI is busy.** WMI shares a limit across the whole machine. If other software has used it up, MooDeX checks the list of linked executables every 2 seconds instead, and keeps retrying WMI.

## Discover (RAWG)

The app contains no API key. You can choose one of three options in **Settings > Discover**:

1. **Proxy.** Deploy [`proxy/`](proxy/README.md) to Vercel with your key, then paste its URL into Settings.
2. **Your own key.** Paste a personal RAWG key. It is stored in **Windows Credential Manager**, never in a file or the database.
3. **Off.** The local library still works fully.

## Your data

Everything lives in `%LOCALAPPDATA%\MooDeX\v2\`:

| Path | What it holds |
|---|---|
| `moodex.db` | Your library (SQLite in WAL mode, safe across crashes) |
| `backups\moodex-YYYY-MM-DD.db` | Daily backups. The last 7 are kept. |
| `covers\` | Cached cover images (WebP). Old ones are removed once the cache reaches 500 MB. |
| `logs\moodex.YYYY-MM-DD.log` | Log files. The last 7 days are kept. Panics also go to `crash.log`. |

The v1 file `%LOCALAPPDATA%\MooDeX\games_data.json` is read, never modified. On the first import it is copied to `games_data.v1.backup.json`.

When you uninstall, your library is kept. If you tick **"Delete the application data"**, the uninstaller first copies `moodex.db` to `Documents\MooDeX backup\`, then removes only the `v2` folder.

No telemetry is sent. The only network traffic is RAWG lookups and cover downloads.

## Building

**Prerequisites:**
- Rust, pinned in [rust-toolchain.toml](rust-toolchain.toml).
- Node 20 or later.
- A C compiler for the bundled SQLite, ring and libwebp sources:
  - MSVC Build Tools, or
  - on the GNU toolchain, MSYS2 MinGW-w64 (`C:\msys64\mingw64\bin`).

```powershell
# Full release build: MooDeX.exe plus the NSIS and MSI installers
powershell -ExecutionPolicy Bypass -File scripts\build.ps1

# Checks (the same gates CI runs)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Build outputs:
- `target/release/MooDeX.exe`
- `target/release/bundle/nsis/MooDeX_2.0.0_x64-setup.exe`
- `target/release/bundle/msi/MooDeX_2.0.0_x64_en-US.msi`

> **GNU toolchain note:** on the `x86_64-pc-windows-gnu` target, WebView2's loader is a separate `WebView2Loader.dll`. The installers include it. If you copy the `.exe` by hand, copy the DLL with it. MSVC builds link the loader statically.

## Updates

MooDeX updates itself from [GitHub Releases](https://github.com/InoshMatheesha/MooDeX/releases). Open **Settings > Updates** and click **Check for updates**. If a newer version is out, click **Download & install**. MooDeX also checks quietly at startup; you can turn that off in the same place.

Every update is signed. The app checks the signature against a public key built into it, and refuses any update that doesn't match. This works with the private key at `%USERPROFILE%\.tauri\moodex.key`, which only the maintainer has.

**To publish an update (maintainer):**

1. Bump `version` in `app/src-tauri/tauri.conf.json`, `Cargo.toml` and `app/package.json`, and add a `## <version>` section to `CHANGELOG.md`.
2. Run `scripts\build.ps1`. It writes `target\release\bundle\latest.json` next to the signed installer.
3. Create the release `v<version>` and attach:
   - the `-setup.exe`
   - its `.sig`
   - the `.msi`
   - `latest.json`

Installed copies pick up the new version at their next check.

## Signing and SmartScreen

A local build is **not signed**. Do not hand it out, not even to a friend for testing (spec §13.8). Every build that is meant for other people goes through [`.github/workflows/release.yml`](.github/workflows/release.yml), which:
1. Signs the exe and both installers with Authenticode and an RFC-3161 timestamp.
2. Signs the update files with minisign.
3. Produces an SBOM and SHA-256 checksums.
4. Fails if any program file is left unsigned.

Even once releases are signed, Windows SmartScreen may warn about the first few versions until enough people have installed them. This is normal for a new publisher and does not mean the app is malware.

## Layout

```
crates/moodex-core      domain types and pure logic (no I/O)
crates/moodex-store     SQLite, migrations, crash-safe writes, backups
crates/moodex-rawg      RAWG client, HTTP cache, rate limiting
crates/moodex-images    cover cache (memory + disk), WebP variants
crates/moodex-winproc   job objects, WMI, launching (the only crate with `unsafe`)
crates/moodex-library   the single owner of library state, the play tracker, v1 import
app/src-tauri           the window, tray, IPC commands, logging, cover protocol
app/ui                  TypeScript frontend (no framework), virtualized grid
proxy/                  Vercel edge function that holds the RAWG key
```
