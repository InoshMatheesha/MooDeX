<div align="center">

<br>
<img src="Icon%20Logo/logo%20with%20name.png" alt="MooDeX Logo" width="400"/>

### *Your Games. Your Stats. Your Vibe.*

A sleek, dark-themed **desktop game library manager** built with WPF & Material Design —  
organize your collection, discover new titles, track playtime, and launch games in one click.

[![.NET](https://img.shields.io/badge/.NET%2010-512BD4?style=for-the-badge&logo=dotnet&logoColor=white)](https://dotnet.microsoft.com/)
[![WPF](https://img.shields.io/badge/WPF-0078D4?style=for-the-badge&logo=windows&logoColor=white)](https://learn.microsoft.com/en-us/dotnet/desktop/wpf/)
[![Material Design](https://img.shields.io/badge/Material%20Design-BB86FC?style=for-the-badge&logo=materialdesign&logoColor=white)](https://github.com/MaterialDesignInXAML/MaterialDesignInXamlToolkit)
[![RAWG API](https://img.shields.io/badge/RAWG%20API-1F1F1F?style=for-the-badge&logo=gamepad&logoColor=white)](https://rawg.io/apidocs)

---

</div>

<br>

## What is MooDeX?

**MooDeX** is a personal game library manager for Windows that lets you keep all your games — whether from your PC or discovered online — in one beautiful, organized hub. Think of it as your own mini-Steam dashboard, designed just for *you*.
Download Now - [MooDeX_v1.0 Setup.exe](https://github.com/InoshMatheesha/MooDeX/releases/tag/MooDex)

<br>

## Features at a Glance

| Feature | Description |
|---|---|
| **Dashboard** | See total games, playtime stats, top 3 most-played games, and pick your all-time **G.O.A.T.** game |
| **My Library** | Curated list of games added via search — edit details, rate them with custom rating descriptions, set status, and add notes |
| **My PC Games** | Import local `.exe` games, auto-match them via RAWG, and **launch directly** from the app |
| **Discover** | Browse **Trending**, **Upcoming**, and **Popular** games powered by the RAWG API — add any to your library instantly |
| **Playtime Tracking** | Background process monitor tracks how long you play each game — updated in real time |
| **Settings** | Minimize to tray, launch at startup, start minimized — your app, your rules |

<br>

## Architecture

MooDeX follows the **MVVM** (Model-View-ViewModel) pattern:

```
MooDeX
├── Models/            # Game, AppSettings — data classes
├── Views/             # XAML pages (Dashboard, Library, MyGames, Discover, Settings)
├── ViewModels/        # Logic layer — binds data to UI
├── Services/          # RAWG API client, Storage (JSON), Process Monitor
├── Helpers/           # Image caching, RelayCommand, ObservableCollection extensions
└── Icon Logo/         # App icon & branding assets
```

<br>

## Getting Started

### Prerequisites

- **Windows 10/11**
- [.NET 10 SDK](https://dotnet.microsoft.com/download) (or later)

### Quick Install

> Just want to use the app? Run the included installer:

```
[MooDeX_v1.0 Setup.exe](https://github.com/InoshMatheesha/MooDeX/releases/tag/MooDex)
```

### Build from Source

```bash
# Clone the repo
git clone https://github.com/your-username/MooDeX.git
cd MooDeX

# Restore & build
dotnet restore
dotnet build

# Run
dotnet run
```

<br>

## How It Works

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Discover    │────▶│  My Library   │────▶│  Dashboard   │
│  (RAWG API)   │     │ (Rate/Edit)   │     │  (Stats)     │
└──────────────┘     └──────────────┘     └──────────────┘
                            │
                     ┌──────┴──────┐
                     │ My PC Games  │
                     │ (Launch/Track)│
                     └─────────────┘
```

1. **Discover** games from the RAWG database → add them to your library  
2. **Import PC games** by browsing for `.exe` files → auto-match cover art via RAWG  
3. **Launch games** directly and MooDeX tracks playtime in the background  
4. **Dashboard** shows your stats — top games, total playtime, status breakdown  

<br>

## UI Highlights

- **Dark theme** with deep purple accents (`#BB86FC`)
- **Card-based layout** with hover animations & elevation effects
- **Debounced search** — fast, responsive, no API spam
- **Live status indicator** — shows "Playing Now" or "Last Played" in the sidebar
- **Rating system** — from *Awful* to *Masterpiece*
- **Ultra-thin modern scrollbars** — minimal & clean

<br>

## Screenshots

A visual tour of MooDeX — from your personal stats hub to the game discovery feed.

<br>

**Dashboard** — Your gaming command centre at a glance

![Dashboard](App%20Screenshots/Dashboard.png)

<br>

**My Library** — Every game you own, beautifully organized

![My Library](App%20Screenshots/My%20Library.png)

<br>

**Discover** — Browse trending, upcoming, and popular titles powered by RAWG

![Discover Games](App%20Screenshots/Discover%20Games.png)

<br>

**My PC Games** — Import local executables and track playtime automatically

![PC Games](App%20Screenshots/PC%20Games.png)

<br>

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

<br>

## Tech Stack

| Layer | Technology |
|---|---|
| **Framework** | WPF (.NET 10) |
| **UI Library** | Material Design In XAML Toolkit v5.3 |
| **API** | RAWG Video Games Database |
| **Data Storage** | Local JSON files |
| **Architecture** | MVVM |
| **Language** | C# |

<br>

## Configuration Options

| Setting | Default | Description |
|---|---|---|
| Minimize to Tray | `Off` | Close button sends app to system tray |
| Launch at Startup | `Off` | Auto-start with Windows via registry |
| Launch Minimized | `Off` | Start hidden in tray (requires startup enabled) |

<br>

## Game Statuses

Every game in your library can be tagged with one of these statuses:

| Status | Meaning |
|---|---|
| Playing | Currently playing |
| Completed | Finished the game |
| Like to Play | On your wishlist |
| Stopped | Dropped / paused |

<br>

## Try MooDeX

MooDeX ships as a ready-to-run Windows installer — no configuration required.
Download, install, and your game library is one click away.

> **System Requirements:** Windows 10 / 11 — .NET 10 Runtime (bundled with the installer)

```
[MooDeX_v1.0 Setup.exe](https://github.com/InoshMatheesha/MooDeX/releases/tag/MooDex)
```

Once installed you can:

- **Import** your local PC games in seconds
- **Discover** trending and upcoming titles from the RAWG database
- **Track** every minute of playtime automatically in the background
- **Rate and review** your games with a personal score and notes
- **Set your G.O.A.T.** — the one game that stands above the rest


<br>

## Contributing

Contributions, issues, and feature requests are welcome!

1. Fork the project  
2. Create your branch (`git checkout -b feature/your-feature`)  
3. Commit your changes (`git commit -m 'Add your feature'`)  
4. Push to the branch (`git push origin feature/your-feature`)  
5. Open a Pull Request  


---

<div align="center">

<img src="Icon%20Logo/icon.png" alt="MooDeX" width="64"/>

**MooDeX** — Your Games. Your Stats. Your Vibe.

Built with WPF · Powered by RAWG · Designed for gamers

</div>
