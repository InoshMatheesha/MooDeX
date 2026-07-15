using System;
using System.Collections.ObjectModel;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Input;
using MooDeX_New_Version_1._0.Helpers;
using MooDeX_New_Version_1._0.Models;
using MooDeX_New_Version_1._0.Services;
using System.Collections.Generic;

namespace MooDeX_New_Version_1._0.ViewModels
{
    public class MyGamesViewModel : ViewModelBase
    {
        private readonly StorageService _storageService;
        private readonly RawgApiService _apiService;
        private RangeObservableCollection<Game> _installedGames = new();
        private Game? _selectedGame;
        private bool _isDialogOpen;
        private bool _isAddMode;
        private bool _isSuggestionDropDownOpen;
        private string _newGameName = string.Empty;
        private string _newExePath = string.Empty;
        private ObservableCollection<string> _gameSuggestions = new();
        private CancellationTokenSource? _suggestionSearchCts;
        private System.Windows.Threading.DispatcherTimer? _refreshTimer;

        // ── Game Matcher Box Properties ───────────────────────────────────────
        private string _searchQuery = string.Empty;
        private ObservableCollection<Game> _matchedGames = new();
        private Game? _selectedMatch;
        private bool _isSearchRunning;
        private bool _showNoMatchesMessage;
        private CancellationTokenSource? _searchCts;

        public RangeObservableCollection<Game> InstalledGames
        {
            get => _installedGames;
            set => SetProperty(ref _installedGames, value);
        }

        public Game? SelectedGame
        {
            get => _selectedGame;
            set => SetProperty(ref _selectedGame, value);
        }

        public bool IsDialogOpen { get => _isDialogOpen; set => SetProperty(ref _isDialogOpen, value); }
        public bool IsAddMode 
        { 
            get => _isAddMode; 
            set 
            {
                SetProperty(ref _isAddMode, value);
                OnPropertyChanged(nameof(IsEditMode));
            }
        }
        public bool IsEditMode => !IsAddMode; 

        public string NewGameName
        {
            get => _newGameName;
            set
            {
                if (SetProperty(ref _newGameName, value) && IsAddMode)
                {
                    DebouncedSuggestionSearch();
                }
            }
        }

        public string NewExePath
        {
            get => _newExePath;
            set
            {
                if (SetProperty(ref _newExePath, value))
                {
                    OnPropertyChanged(nameof(NewExeName));
                }
            }
        }

        public string NewExeName => string.IsNullOrWhiteSpace(NewExePath) ? string.Empty : Path.GetFileName(NewExePath);

        public ObservableCollection<string> GameSuggestions
        {
            get => _gameSuggestions;
            set => SetProperty(ref _gameSuggestions, value);
        }

        public bool IsSuggestionDropDownOpen
        {
            get => _isSuggestionDropDownOpen;
            set => SetProperty(ref _isSuggestionDropDownOpen, value);
        }

        // ── Matcher Properties Getters/Setters ───────────────────────────────
        public string SearchQuery
        {
            get => _searchQuery;
            set
            {
                if (SetProperty(ref _searchQuery, value))
                {
                    DebouncedSearch();
                }
            }
        }

        public ObservableCollection<Game> MatchedGames
        {
            get => _matchedGames;
            set => SetProperty(ref _matchedGames, value);
        }

        public Game? SelectedMatch
        {
            get => _selectedMatch;
            set
            {
                if (SetProperty(ref _selectedMatch, value))
                {
                    if (_selectedMatch != null)
                    {
                        NewGameName = _selectedMatch.Title;
                    }
                }
            }
        }

        public bool IsSearchRunning
        {
            get => _isSearchRunning;
            set => SetProperty(ref _isSearchRunning, value);
        }

        public bool ShowNoMatchesMessage
        {
            get => _showNoMatchesMessage;
            set => SetProperty(ref _showNoMatchesMessage, value);
        }

        public ICommand OpenAddDialogCommand { get; }
        public ICommand OpenEditDialogCommand { get; }
        public ICommand CloseDialogCommand { get; }
        public ICommand SaveGameCommand { get; }
        public ICommand RemoveGameCommand { get; }
        public ICommand PlayGameCommand { get; }
        public ICommand BrowseExeCommand { get; }

        public MyGamesViewModel()
        {
            _storageService = new StorageService();
            _apiService = new RawgApiService();
            LoadGames();

            OpenAddDialogCommand = new RelayCommand(_ => {
                BrowseForExecutableAndOpenMatcher();
            });

            BrowseExeCommand = new RelayCommand(_ => BrowseForExecutable());

            OpenEditDialogCommand = new RelayCommand(g => {
                if (g is Game gameToEdit)
                {
                    IsAddMode = false;
                    SelectedGame = gameToEdit;

                    // Pre-populate fields for Matcher Box
                    NewExePath = gameToEdit.ExecutablePath;
                    NewGameName = gameToEdit.Title;
                    SearchQuery = gameToEdit.Title;

                    // Clear old search states
                    MatchedGames.Clear();
                    SelectedMatch = null;
                    ShowNoMatchesMessage = false;

                    IsDialogOpen = true;

                    // Run search immediately
                    _ = ExecuteSearchAsync(SearchQuery, CancellationToken.None);
                }
            });

            CloseDialogCommand = new RelayCommand(async _ => {
                IsDialogOpen = false;
                GameSuggestions.Clear();
                IsSuggestionDropDownOpen = false;
                MatchedGames.Clear();
                SelectedMatch = null;
                ShowNoMatchesMessage = false;
                await ApplySortAsync();
            });

            SaveGameCommand = new RelayCommand(async _ => await SaveGameAsync());

            RemoveGameCommand = new RelayCommand(g => {
                if (g is Game gameToRemove) {
                    // Instant visual removal (0ms delay)
                    var toRemove = InstalledGames.FirstOrDefault(x => x.Id == gameToRemove.Id) ?? gameToRemove;
                    InstalledGames.Remove(toRemove);

                    if (SelectedGame?.Id == gameToRemove.Id)
                    {
                        IsDialogOpen = false;
                        SelectedGame = null;
                    }

                    // Save to disk in background without blocking UI thread
                    Task.Run(() => {
                        var games = _storageService.LoadPCGames();
                        games.RemoveAll(x => x.Id == gameToRemove.Id);
                        _storageService.SaveSubset(games, "PCGames");
                    });
                }
            });

            PlayGameCommand = new RelayCommand(game => LaunchGame(game as Game));

            _refreshTimer = new System.Windows.Threading.DispatcherTimer();
            _refreshTimer.Interval = TimeSpan.FromSeconds(5);
            _refreshTimer.Tick += async (s, e) => await RefreshGameStatsAsync();
            _refreshTimer.Start();
        }

        private async void BrowseForExecutableAndOpenMatcher()
        {
            var dialog = new Microsoft.Win32.OpenFileDialog
            {
                Filter = "Executables (*.exe)|*.exe|All files (*.*)|*.*",
                Title = "Select Game Executable"
            };

            if (dialog.ShowDialog() == true)
            {
                IsAddMode = true;
                NewExePath = dialog.FileName;

                // Clear old search states
                MatchedGames.Clear();
                SelectedMatch = null;
                ShowNoMatchesMessage = false;

                // Get the game folder name
                var folderName = GetGameFolderName(NewExePath);
                SearchQuery = folderName; // This will trigger search via property setter
                NewGameName = folderName; // Fallback title

                // Open dialog
                IsDialogOpen = true;

                // Run search immediately
                await ExecuteSearchAsync(folderName, CancellationToken.None);
            }
        }

        private string GetGameFolderName(string exePath)
        {
            if (string.IsNullOrWhiteSpace(exePath)) return string.Empty;

            try
            {
                var directory = Path.GetDirectoryName(exePath);
                if (string.IsNullOrEmpty(directory)) return string.Empty;

                var dirInfo = new DirectoryInfo(directory);
                var genericNames = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
                {
                    "bin", "binaries", "x64", "x86", "win64", "win32", "retail", "shipping", "win64_shipping", "system", "client", "win_x64", "win-x64", "release"
                };

                // Climb up parent directories if they are generic launcher/engine folders
                while (dirInfo != null && genericNames.Contains(dirInfo.Name))
                {
                    dirInfo = dirInfo.Parent;
                }

                if (dirInfo != null)
                {
                    return dirInfo.Name;
                }
            }
            catch { }

            return Path.GetFileNameWithoutExtension(exePath);
        }

        private async void DebouncedSearch()
        {
            _searchCts?.Cancel();
            _searchCts = new CancellationTokenSource();
            var token = _searchCts.Token;

            try
            {
                await Task.Delay(500, token);
                if (!token.IsCancellationRequested)
                {
                    await ExecuteSearchAsync(SearchQuery, token);
                }
            }
            catch (TaskCanceledException)
            {
            }
            catch
            {
                MatchedGames.Clear();
                ShowNoMatchesMessage = true;
            }
        }

        private async Task ExecuteSearchAsync(string query, CancellationToken token)
        {
            if (string.IsNullOrWhiteSpace(query))
            {
                System.Windows.Application.Current.Dispatcher.Invoke(() =>
                {
                    MatchedGames.Clear();
                    ShowNoMatchesMessage = false;
                });
                return;
            }

            IsSearchRunning = true;
            ShowNoMatchesMessage = false;

            try
            {
                var results = await _apiService.SearchGamesAsync(query);

                if (!token.IsCancellationRequested)
                {
                    System.Windows.Application.Current.Dispatcher.Invoke(() =>
                    {
                        MatchedGames.Clear();
                        foreach (var g in results)
                        {
                            MatchedGames.Add(g);
                        }

                        // Select the first match automatically as a default
                        if (MatchedGames.Count > 0)
                        {
                            SelectedMatch = MatchedGames[0];
                        }
                        else
                        {
                            SelectedMatch = null;
                        }

                        ShowNoMatchesMessage = MatchedGames.Count == 0;
                    });
                }
            }
            catch
            {
                System.Windows.Application.Current.Dispatcher.Invoke(() =>
                {
                    MatchedGames.Clear();
                    ShowNoMatchesMessage = true;
                });
            }
            finally
            {
                IsSearchRunning = false;
            }
        }

        private async void DebouncedSuggestionSearch()
        {
            _suggestionSearchCts?.Cancel();
            _suggestionSearchCts = new CancellationTokenSource();
            var token = _suggestionSearchCts.Token;

            try
            {
                await Task.Delay(350, token);

                if (!token.IsCancellationRequested)
                {
                    await UpdateGameSuggestionsAsync(NewGameName);
                }
            }
            catch (TaskCanceledException)
            {
            }
            catch
            {
                GameSuggestions.Clear();
                IsSuggestionDropDownOpen = false;
            }
        }

        private async Task UpdateGameSuggestionsAsync(string query)
        {
            if (!IsAddMode || string.IsNullOrWhiteSpace(query))
            {
                System.Windows.Application.Current.Dispatcher.Invoke(() =>
                {
                    GameSuggestions.Clear();
                    IsSuggestionDropDownOpen = false;
                });
                return;
            }

            try
            {
                var results = await _apiService.SearchGamesAsync(query);
                var suggestions = results
                    .Select(g => g.Title)
                    .Where(title => !string.IsNullOrWhiteSpace(title))
                    .Distinct(StringComparer.OrdinalIgnoreCase)
                    .Take(6)
                    .ToList();

                System.Windows.Application.Current.Dispatcher.Invoke(() =>
                {
                    GameSuggestions.Clear();
                    foreach (var suggestion in suggestions)
                    {
                        GameSuggestions.Add(suggestion);
                    }

                    IsSuggestionDropDownOpen = GameSuggestions.Count > 0;
                });
            }
            catch
            {
                System.Windows.Application.Current.Dispatcher.Invoke(() =>
                {
                    GameSuggestions.Clear();
                    IsSuggestionDropDownOpen = false;
                });
            }
        }

        private async void LaunchGame(Game? game)
        {
            if (game == null)
            {
                return;
            }

            var exePath = game.ExecutablePath;
            if (string.IsNullOrWhiteSpace(exePath) || !File.Exists(exePath))
            {
                System.Windows.MessageBox.Show("This game does not have a saved executable path yet. Edit it and choose the .exe file first.", "Play Game", MessageBoxButton.OK, MessageBoxImage.Information);
                return;
            }

            try
            {
                var startInfo = new ProcessStartInfo
                {
                    FileName = exePath,
                    WorkingDirectory = Path.GetDirectoryName(exePath) ?? string.Empty,
                    UseShellExecute = true
                };

                var process = Process.Start(startInfo);
                if (process != null)
                {
                    game.PlayCount++;
                    ProcessMonitor.Instance?.RegisterProcess(game.Id, process);
                }

                game.LastPlayed = DateTime.Now;

                var pcGames = await Task.Run(() => _storageService.LoadPCGames());
                var existing = pcGames.FirstOrDefault(x => x.Id == game.Id);
                if (existing != null)
                {
                    existing.LastPlayed = game.LastPlayed;
                    existing.PlayCount = game.PlayCount;
                    await Task.Run(() => _storageService.SaveSubset(pcGames, "PCGames"));
                }

                // Force process monitor to check immediately so the sidebar updates instantly
                ProcessMonitor.Instance?.ForceCheck();

                await ApplySortAsync();
            }
            catch (Exception ex)
            {
                System.Windows.MessageBox.Show($"Unable to launch the game.\n\n{ex.Message}", "Play Game", MessageBoxButton.OK, MessageBoxImage.Error);
            }
        }

        private async Task SaveGameAsync()
        {
            var games = await Task.Run(() => _storageService.LoadPCGames()); // ← PCGames subset only

            if (IsAddMode)
            {
                if (string.IsNullOrWhiteSpace(NewGameName)) return;

                // Auto-fetch cover art from RAWG API
                string coverUrl = string.Empty;
                if (SelectedMatch != null)
                {
                    coverUrl = SelectedMatch.CoverUrl;
                }
                else
                {
                    try
                    {
                        var searchResults = await _apiService.SearchGamesAsync(NewGameName);
                        coverUrl = searchResults.FirstOrDefault()?.CoverUrl ?? string.Empty;
                    }
                    catch { /* Ignore API failure to not block local addition */ }
                }

                // Generate an ID that is unique across ALL games (both sources)
                var allGames = await Task.Run(() => _storageService.LoadGames());
                int newId = allGames.Any() ? allGames.Max(g => g.Id) + 1 : 1;

                games.Add(new Game
                {
                    Id             = newId,
                    Title          = NewGameName,
                    CoverUrl       = coverUrl,
                    ExecutableName = string.IsNullOrWhiteSpace(NewExePath) ? string.Empty : Path.GetFileName(NewExePath),
                    ExecutablePath = NewExePath,
                    AddedDate      = DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss"),
                    ReleaseDate    = SelectedMatch != null ? SelectedMatch.ReleaseDate : string.Empty,
                    Status         = "Playing",
                    Playtime       = 0,
                    Source         = "PCGames" // ← strict segregation tag
                });

                // Also add to My Library if not already present
                var libraryGames = await Task.Run(() => _storageService.LoadLibraryGames());
                bool alreadyInLibrary = libraryGames.Any(g => 
                    (SelectedMatch != null && g.Id == SelectedMatch.Id) ||
                    string.Equals(g.Title?.Trim(), NewGameName?.Trim(), StringComparison.OrdinalIgnoreCase));

                if (!alreadyInLibrary)
                {
                    int libId = SelectedMatch != null ? SelectedMatch.Id : (newId + 1); // ensures uniqueness
                    var newLibGame = new Game
                    {
                        Id = libId,
                        Title = NewGameName,
                        CoverUrl = coverUrl,
                        AddedDate = DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss"),
                        ReleaseDate = SelectedMatch != null ? SelectedMatch.ReleaseDate : string.Empty,
                        Status = "Like to Play",
                        Playtime = 0,
                        Source = "Library"
                    };
                    libraryGames.Add(newLibGame);
                    await Task.Run(() => _storageService.SaveSubset(libraryGames, "Library"));
                }
            }
            else
            {
                if (SelectedGame == null) return;
                var existing = games.FirstOrDefault(x => x.Id == SelectedGame.Id);
                if (existing != null)
                {
                    existing.Title          = NewGameName;
                    existing.ExecutableName = string.IsNullOrWhiteSpace(NewExePath) ? string.Empty : Path.GetFileName(NewExePath);
                    existing.ExecutablePath = NewExePath;

                    // If they selected a match, update the cover art and release date!
                    if (SelectedMatch != null)
                    {
                        existing.CoverUrl = SelectedMatch.CoverUrl;
                        existing.ReleaseDate = SelectedMatch.ReleaseDate;
                    }
                }

                // Mutate the in-memory SelectedGame instance directly so UI updates immediately
                SelectedGame.Title          = NewGameName;
                SelectedGame.ExecutableName = string.IsNullOrWhiteSpace(NewExePath) ? string.Empty : Path.GetFileName(NewExePath);
                SelectedGame.ExecutablePath = NewExePath;
                if (SelectedMatch != null)
                {
                    SelectedGame.CoverUrl = SelectedMatch.CoverUrl;
                    SelectedGame.ReleaseDate = SelectedMatch.ReleaseDate;
                }
            }

            var saveTask = Task.Run(() => _storageService.SaveSubset(games, "PCGames")); // safe merge – Library untouched
            IsDialogOpen = false;
            GameSuggestions.Clear();
            IsSuggestionDropDownOpen = false;
            MatchedGames.Clear();
            SelectedMatch = null;
            ShowNoMatchesMessage = false;
            
            await saveTask; // Wait for save to complete
            await ApplySortAsync();
        }

        private string _selectedSortOption = "A-Z";
        public string SelectedSortOption
        {
            get => _selectedSortOption;
            set
            {
                if (SetProperty(ref _selectedSortOption, value))
                {
                    _ = ApplySortAsync();
                }
            }
        }

        public ObservableCollection<string> SortOptions { get; } = new()
        {
            "A-Z", "Recently Added", "Recently Played", "Playtime"
        };

        private async Task ApplySortAsync()
        {
            var games = await Task.Run(() => _storageService.LoadPCGames());
            List<Game> sorted;

            switch (SelectedSortOption)
            {
                case "A-Z":
                    sorted = games.OrderBy(g => g.Title).ToList();
                    break;
                case "Recently Added":
                    sorted = games.OrderByDescending(g => g.AddedDateParsed).ThenByDescending(g => g.Id).ToList();
                    break;
                case "Recently Played":
                    sorted = games.OrderByDescending(g => g.LastPlayed ?? DateTime.MinValue).ToList();
                    break;
                case "Playtime":
                    sorted = games.OrderByDescending(g => g.Playtime).ToList();
                    break;
                default:
                    sorted = games.OrderBy(g => g.Title).ToList();
                    break;
            }

            System.Windows.Application.Current.Dispatcher.Invoke(() =>
            {
                bool sequenceEqual = InstalledGames.Count == sorted.Count &&
                                     InstalledGames.Select(g => g.Id).SequenceEqual(sorted.Select(g => g.Id));
                if (!sequenceEqual)
                {
                    InstalledGames.ReplaceRange(sorted);
                }
            });
        }

        private void LoadGames()
        {
            _ = ApplySortAsync();
        }

        private void BrowseForExecutable()
        {
            var dialog = new Microsoft.Win32.OpenFileDialog
            {
                Filter = "Executables (*.exe)|*.exe|All files (*.*)|*.*",
                Title = "Select Game Executable"
            };

            if (dialog.ShowDialog() == true)
            {
                NewExePath = dialog.FileName;

                // If it's Add mode, we also update the title to the file name
                if (IsAddMode)
                {
                    var fileName = Path.GetFileNameWithoutExtension(dialog.FileName);
                    NewGameName = fileName.Replace("_", " ").Replace(".", " ");
                }
            }
        }

        public void Cleanup()
        {
            _refreshTimer?.Stop();
            _refreshTimer = null;
        }

        private async Task RefreshGameStatsAsync()
        {
            try
            {
                var games = await Task.Run(() => _storageService.LoadPCGames());
                foreach (var g in games)
                {
                    var existing = InstalledGames.FirstOrDefault(x => x.Id == g.Id);
                    if (existing != null)
                    {
                        if (existing.Playtime != g.Playtime)
                        {
                            existing.Playtime = g.Playtime;
                        }
                        if (existing.LastPlayed != g.LastPlayed)
                        {
                            existing.LastPlayed = g.LastPlayed;
                        }
                    }
                }
            }
            catch {}
        }
    }
}