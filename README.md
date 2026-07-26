<div align="center"> 


<br>
<img src="Icon%20Logo/logo%20with%20name.png" alt="MooDeX Logo" width="300"/>

### Your Games. Your Stats. Your Vibe. 

**MooDeX** is a sleek, dark-themed game library manager for Windows and the Web. Organize your collection, discover new titles, track playtime, and launch games — all from one place.

<br>

[![.NET](https://img.shields.io/badge/.NET%2010-512BD4?style=for-the-badge&logo=dotnet&logoColor=white)](https://dotnet.microsoft.com/)
[![WPF](https://img.shields.io/badge/WPF-0078D4?style=for-the-badge&logo=windows&logoColor=white)](https://learn.microsoft.com/en-us/dotnet/desktop/wpf/)
[![Material Design](https://img.shields.io/badge/Material%20Design-BB86FC?style=for-the-badge&logo=materialdesign&logoColor=white)](https://github.com/MaterialDesignInXAML/MaterialDesignInXamlToolkit)
[![RAWG API](https://img.shields.io/badge/RAWG%20API-1F1F1F?style=for-the-badge&logo=gamepad&logoColor=white)](https://rawg.io/apidocs)
[![Vercel Deployment](https://img.shields.io/badge/Vercel_Web-000000?style=for-the-badge&logo=vercel&logoColor=white)](https://moodex-web.vercel.app/)

<br>

### 🔥 DON'T WANT TO INSTALL ANYTHING? TRY MOODEX IN YOUR BROWSER NOW! 🌐

[![MooDeX Web](https://img.shields.io/badge/🌐_LAUNCH_LIVE_WEB_DEMO-4c29a6?style=for-the-badge&logo=vercel&logoColor=black)](https://moodex-web.vercel.app/)
[![Download on SourceForge](https://img.shields.io/badge/Download%20on-SourceForge-FF6600?style=for-the-badge&logo=sourceforge&logoColor=white)](https://sourceforge.net/projects/moodex/)
[![Download on GitHub Releases](https://img.shields.io/badge/Download%20on-GitHub%20Releases-181717?style=for-the-badge&logo=github&logoColor=white)](https://github.com/InoshMatheesha/MooDeX/releases)

*⚡ **Instant 1-Click Web Access:** Experience the vibe right now at **[moodex-web.vercel.app](https://moodex-web.vercel.app/)**!*
<br/>
*💻 Windows 10/11 Desktop App · .NET 10 Runtime bundled with the installer · No configuration required*

</div>

---

## Overview

MooDeX is a personal game library manager for Windows that brings together the games on your PC and the titles you discover online into a single, organized hub — a mini dashboard designed entirely around your own collection and play habits.

## Features

| Feature | Description |
|---|---|
| **Dashboard** | View total games, playtime statistics, your top three most-played titles, and mark your all-time favorite game (G.O.A.T.) |
| **My Library** | A curated list of games added via search, with editable details, custom ratings, status tracking, and personal notes |
| **My PC Games** | Import local `.exe` files, auto-match them against the RAWG database, and launch them directly from the app |
| **Discover** | Browse Trending, Upcoming, and Popular titles powered by the RAWG API, and add any of them to your library instantly |
| **Playtime Tracking** | A background process monitor records how long you play each game, updated in real time |
| **Settings** | Minimize to tray, launch at startup, and start minimized, with full control over app behavior |

## Architecture

MooDeX follows the MVVM (Model-View-ViewModel) pattern:

```
MooDeX
├── Models/            # Game, AppSettings — data classes
├── Views/             # XAML pages (Dashboard, Library, MyGames, Discover, Settings)
├── ViewModels/        # Logic layer — binds data to UI
├── Services/          # RAWG API client, storage (JSON), process monitor
├── Helpers/           # Image caching, RelayCommand, ObservableCollection extensions
└── Icon Logo/         # App icon and branding assets
```

## Getting Started

### 🌐 Instant Access (No Installation Required!)

Want to test drive MooDeX right away without downloading anything?

👉 **[🔥 Launch MooDeX Web Live Demo →](https://moodex-web.vercel.app/)**

### Prerequisites (For Desktop App)

- Windows 10 or 11
- [.NET 10 SDK](https://dotnet.microsoft.com/download) (or later) — only required for building from source

### Install the Desktop App

The fastest way to get started on Windows is to download the installer:

- **[SourceForge →](https://sourceforge.net/projects/moodex/)**
- **[GitHub Releases →](https://github.com/InoshMatheesha/MooDeX/releases)**

Run the installer and MooDeX will be ready to use — no additional setup required.

### Build from Source

```bash
# Clone the repository
git clone https://github.com/InoshMatheesha/MooDeX.git
cd MooDeX

# Restore and build
dotnet restore
dotnet build

# Run
dotnet run
```

## How It Works

```
┌───────────────┐     ┌───────────────┐     ┌──────────────┐
│   Discover    │────▶│   My Library   │────▶│  Dashboard   │
│  (RAWG API)   │     │  (Rate / Edit) │     │   (Stats)    │
└───────────────┘     └───────────────┘     └──────────────┘
                              │
                       ┌──────┴───────┐
                       │ My PC Games  │
                       │(Launch/Track)│
                       └──────────────┘
```

1. **Discover** games from the RAWG database and add them to your library
2. **Import** PC games by browsing for `.exe` files, with cover art auto-matched via RAWG
3. **Launch** games directly from MooDeX, which tracks playtime in the background
4. **Review** your stats on the Dashboard — top games, total playtime, and status breakdown

## UI Highlights

- Dark theme with deep purple accents (`#BB86FC`)
- Card-based layout with hover animations and elevation effects
- Debounced search for fast, responsive results without excess API calls
- Live status indicator showing "Playing Now" or "Last Played" in the sidebar
- Five-tier rating system, from *Awful* to *Masterpiece*
- Ultra-thin, modern scrollbars throughout

## Screenshots

**Dashboard** — your gaming command center at a glance

![Dashboard](App%20Screenshots/Dashboard.png)

**My Library** — every game you own, beautifully organized

![My Library](App%20Screenshots/My%20Library.png)

**Discover** — browse trending, upcoming, and popular titles powered by RAWG

![Discover Games](App%20Screenshots/Discover%20Games.png)

**My PC Games** — import local executables and track playtime automatically

![PC Games](App%20Screenshots/PC%20Games.png)

<table>
  <tr>
    <td align="center" width="50%">
      <strong>Edit Game Details</strong><br/><br/>
      <img src="App%20Screenshots/Edit%20Game%20Details.png" alt="Edit Game Details" width="340"/>
      <br/><em>Set status, rate the game, and add personal notes</em>
    </td>
    <td align="center" width="50%">
      <strong>Match Game &amp; Cover</strong><br/><br/>
      <img src="App%20Screenshots/Match%20Game%20%26%20Cover.png" alt="Match Game and Cover" width="340"/>
      <br/><em>Auto-match your local .exe to the RAWG database for cover art</em>
    </td>
  </tr>
</table>

## Tech Stack

| Layer | Technology |
|---|---|
| Desktop Framework | WPF (.NET 10) |
| Web Platform | Vercel Live Showcase ([moodex-web.vercel.app](https://moodex-web.vercel.app/)) |
| UI Library | Material Design In XAML Toolkit v5.3 |
| API | RAWG Video Games Database |
| Data Storage | Local JSON files |
| Architecture | MVVM |
| Language | C# |

## Configuration Options

| Setting | Default | Description |
|---|---|---|
| Minimize to Tray | Off | Close button sends the app to the system tray instead of exiting |
| Launch at Startup | Off | Automatically starts MooDeX with Windows via the registry |
| Launch Minimized | Off | Starts the app hidden in the tray (requires startup enabled) |

## Game Statuses

Every game in your library can be tagged with one of the following:

| Status | Meaning |
|---|---|
| Playing | Currently in progress |
| Completed | Finished |
| Like to Play | On your wishlist |
| Stopped | Dropped or paused |

## Contributing

Contributions, issues, and feature requests are welcome.

1. Fork the project
2. Create your feature branch: `git checkout -b feature/your-feature`
3. Commit your changes: `git commit -m 'Add your feature'`
4. Push to the branch: `git push origin feature/your-feature`
5. Open a Pull Request

---

<div align="center">

<img src="Icon%20Logo/icon.ico" alt="MooDeX" width="64"/>

**MooDeX** — Your Games. Your Stats. Your Vibe.

Built with WPF · Powered by RAWG · Designed for gamers

</div>
