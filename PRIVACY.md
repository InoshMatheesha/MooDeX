# Privacy Policy for MooDeX

**Effective Date:** November 14, 2026  
**Project:** MooDeX (Desktop Game Library & Playtime Tracker)  
**Maintainer:** Inosh Matheesha ([@InoshMatheesha](https://github.com/InoshMatheesha))

MooDeX is designed with a strict privacy-first architecture. It does not collect telemetry, track usage, or require a user account. Your gaming habits, playtime data, and personal library remain strictly your own.

---

## 1. What Data Stays on Your Computer (Local Only)

The core functionality of MooDeX operates offline on your personal computer:
- **Game Library Data:** The list of games, game titles, executable paths, installation directories, custom tags, status, user notes, and ratings are stored exclusively in a local SQLite database (`%LOCALAPPDATA%\MooDeX\v2\moodex.db`).
- **Playtime Tracking & Sessions:** Game session logs, start/end timestamps, heartbeat timestamps, and total playtime durations are calculated and saved strictly on your local PC.
- **Backups & Cache:** Daily automated database backups (`%LOCALAPPDATA%\MooDeX\v2\backups`) and application logs (`%LOCALAPPDATA%\MooDeX\v2\logs`) are stored locally.

> **Crucially, your local game library, playtime history, folder paths, and system information are NEVER uploaded, transmitted, synced, or shared with any server, cloud provider, or third party.**

---

## 2. Outbound Network Requests

MooDeX only establishes network connections for essential functionality explicitly initiated by features in the app. These connections are:

### A. Game Catalogue Lookups & Metadata Search (RAWG API)
- **When it occurs:** When you browse the *Discover* section or manually search for a game to match metadata.
- **What is sent:** Only the search term (e.g., game title) entered by the user. No personal information, library data, or device identifiers are attached.
- **Destination:** [RAWG Video Games Database API](https://rawg.io) (`api.rawg.io`) or a user-configured proxy URL if deployed.
- **API Key Storage:** If a personal RAWG API key is provided, it is stored securely in the Windows Credential Manager and is never saved in plain text files or sent to MooDeX developers.

### B. Game Cover Art Downloads
- **When it occurs:** When downloading artwork for games added to your library or displayed in Discover.
- **What is sent:** Standard HTTP GET requests for specific image assets.
- **Destination:** RAWG media CDN / image host servers.
- **Caching:** Downloaded images are converted and stored locally on your PC as WebP files (`%LOCALAPPDATA%\MooDeX\v2\covers`) so they do not need to be downloaded repeatedly.

### C. Software Update Checks
- **When it occurs:** When you manually click "Check for updates" in Settings, or automatically on startup if the optional "Check for updates on launch" setting is enabled (can be toggled off at any time).
- **What is sent:** A standard HTTPS request to query version release information. No account or user data is included.
- **Destination:** GitHub Releases API (`https://api.github.com/repos/InoshMatheesha/MooDeX/releases/latest`).
- **Verification:** Downloaded updates are cryptographically signature-verified against an embedded public key before installation.

---

## 3. Analytics & Telemetry

- **No Telemetry:** MooDeX contains zero analytics, tracking pixels, crash telemetry SDKs, or diagnostic beacons.
- **No Third-Party Trackers:** No advertising networks, behavioral tracking, or analytics vendors are included.

---

## 4. Data Deletion & Export

You retain complete control over your data:
- **Export:** You can export your full library and session history at any time via **Settings > Your data > Export library**.
- **Deletion:** Deleting `%LOCALAPPDATA%\MooDeX` permanently removes all local databases, cached covers, backups, and logs.

---

## 5. Contact

If you have questions or security concerns regarding this privacy policy, please open an issue or security advisory on the official GitHub repository:
- **Repository:** [https://github.com/InoshMatheesha/MooDeX](https://github.com/InoshMatheesha/MooDeX)
- **Maintainer:** Inosh Matheesha (<inoshmatheesha@gmail.com>)
