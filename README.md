<div align="center">

# 🎮 MooDeX

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

## ✨ What is MooDeX?

**MooDeX** is a personal game library manager for Windows that lets you keep all your games — whether from your PC or discovered online — in one beautiful, organized hub. Think of it as your own mini-Steam dashboard, designed just for *you*.

<br>

## 🧩 Features at a Glance

| Feature | Description |
|---|---|
| 📊 **Dashboard** | See total games, playtime stats, top 3 most-played games, and pick your all-time **G.O.A.T.** game |
| 📚 **My Library** | Curated list of games added via search — edit details, rate them (with emoji descriptions!), set status, and add notes |
| 🖥️ **My PC Games** | Import local `.exe` games, auto-match them via RAWG, and **launch directly** from the app |
| 🔍 **Discover** | Browse **Trending**, **Upcoming**, and **Popular** games powered by the RAWG API — add any to your library instantly |
| ⏱️ **Playtime Tracking** | Background process monitor tracks how long you play each game — updated in real time |
| ⚙️ **Settings** | Minimize to tray, launch at startup, start minimized — your app, your rules |

<br>

## 🏗️ Architecture

MooDeX follows the **MVVM** (Model-View-ViewModel) pattern:

```
📂 MooDeX
├── 📁 Models/            # Game, AppSettings — data classes
├── 📁 Views/             # XAML pages (Dashboard, Library, MyGames, Discover, Settings)
├── 📁 ViewModels/        # Logic layer — binds data to UI
├── 📁 Services/          # RAWG API client, Storage (JSON), Process Monitor
├── 📁 Helpers/           # Image caching, RelayCommand, ObservableCollection extensions
└── 📁 Icon Logo/         # App icon & branding assets
```

<br>

## 🚀 Getting Started

### Prerequisites

- **Windows 10/11**
- [.NET 10 SDK](https://dotnet.microsoft.com/download) (or later)

### Quick Install

> Just want to use the app? Run the included installer:

```
MooDeX_v1.0 Setup.exe
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

## 🎯 How It Works

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

## 🎨 UI Highlights

- 🌑 **Dark theme** with deep purple accents (`#BB86FC`)
- 🃏 **Card-based layout** with hover animations & elevation effects
- 🔎 **Debounced search** — fast, responsive, no API spam
- 📊 **Live status indicator** — shows "Playing Now" or "Last Played" in the sidebar
- ⭐ **Emoji ratings** — from *"Awful 😠"* to *"Masterpiece! 👑"*
- 🖱️ **Ultra-thin modern scrollbars** — minimal & clean

<br>

## 🛠️ Tech Stack

| Layer | Technology |
|---|---|
| **Framework** | WPF (.NET 10) |
| **UI Library** | Material Design In XAML Toolkit v5.3 |
| **API** | RAWG Video Games Database |
| **Data Storage** | Local JSON files |
| **Architecture** | MVVM |
| **Language** | C# |

<br>

## ⚙️ Configuration Options

| Setting | Default | Description |
|---|---|---|
| Minimize to Tray | `Off` | Close button sends app to system tray |
| Launch at Startup | `Off` | Auto-start with Windows via registry |
| Launch Minimized | `Off` | Start hidden in tray (requires startup enabled) |

<br>

## 📋 Game Statuses

Every game in your library can be tagged with one of these statuses:

| Status | Meaning |
|---|---|
| 🎮 Playing | Currently playing |
| ✅ Completed | Finished the game |
| 💜 Like to Play | On your wishlist |
| ⛔ Stopped | Dropped / paused |

<br>

## 🤝 Contributing

Contributions, issues, and feature requests are welcome!

1. Fork the project  
2. Create your branch (`git checkout -b feature/cool-feature`)  
3. Commit your changes (`git commit -m 'Add cool feature'`)  
4. Push to the branch (`git push origin feature/cool-feature`)  
5. Open a Pull Request  

<br>

## 📜 License

This project is for educational purposes as part of a university project at **SLIIT**.

<br>

---

<div align="center">

**Made with 💜 and a lot of late-night gaming sessions**

*MooDeX v1.0*

</div>
