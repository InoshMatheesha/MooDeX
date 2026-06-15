using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using System.Windows.Input;
using MooDeX_New_Version_1._0.Helpers;
using MooDeX_New_Version_1._0.Models;
using MooDeX_New_Version_1._0.Services;

namespace MooDeX_New_Version_1._0.ViewModels
{
    public class LibraryViewModel : ViewModelBase
    {
        private readonly StorageService _storageService;
        private readonly RawgApiService _apiService;

        // ── Main collection (Library source only) ──────────────────────────────
        private RangeObservableCollection<Game> _myGames = new RangeObservableCollection<Game>();
        private Game? _selectedGame;

        // ── Edit dialog ────────────────────────────────────────────────────────
        private bool _isEditDialogOpen;

        // ── Add-to-Library dialog ──────────────────────────────────────────────
        private bool _isAddDialogOpen;
        private string _addSearchQuery = string.Empty;
        private ObservableCollection<Game> _addSearchResults = new ObservableCollection<Game>();
        private bool _isAddSearchRunning;
        private CancellationTokenSource? _addSearchCts;
        private System.Windows.Threading.DispatcherTimer? _refreshTimer;

        // ══════════════════════════════════════════════════════════════════════
        // Properties
        // ══════════════════════════════════════════════════════════════════════

        public RangeObservableCollection<Game> MyGames
        {
            get => _myGames;
            set => SetProperty(ref _myGames, value);
        }

        private string _librarySearchQuery = string.Empty;
        public string LibrarySearchQuery
        {
            get => _librarySearchQuery;
            set
            {
                if (SetProperty(ref _librarySearchQuery, value))
                {
                    _ = ApplySortAsync();
                }
            }
        }

        public Game? SelectedGame
        {
            get => _selectedGame;
            set => SetProperty(ref _selectedGame, value);
        }

        // Kept as IsDialogOpen so existing XAML binding in LibraryView still works
        public bool IsDialogOpen
        {
            get => _isEditDialogOpen;
            set => SetProperty(ref _isEditDialogOpen, value);
        }

        public bool IsAddDialogOpen
        {
            get => _isAddDialogOpen;
            set => SetProperty(ref _isAddDialogOpen, value);
        }

        public string AddSearchQuery
        {
            get => _addSearchQuery;
            set
            {
                if (SetProperty(ref _addSearchQuery, value))
                    DebouncedAddSearch();
            }
        }

        public ObservableCollection<Game> AddSearchResults
        {
            get => _addSearchResults;
            set => SetProperty(ref _addSearchResults, value);
        }

        public bool IsAddSearchRunning
        {
            get => _isAddSearchRunning;
            set => SetProperty(ref _isAddSearchRunning, value);
        }

        public ObservableCollection<string> Statuses { get; } = new ObservableCollection<string>
        {
            "Playing", "Completed", "Stopped", "Like to Play"
        };

        // ══════════════════════════════════════════════════════════════════════
        // Commands
        // ══════════════════════════════════════════════════════════════════════

        public ICommand EditGameCommand { get; }
        public ICommand SaveGameCommand { get; }
        public ICommand CloseDialogCommand { get; }
        public ICommand DeleteGameCommand { get; }

        // New: open the search-add dialog
        public ICommand OpenAddDialogCommand { get; }
        public ICommand CloseAddDialogCommand { get; }
        public ICommand AddSearchResultToLibraryCommand { get; }
        public ICommand SetRatingCommand { get; }

        // ══════════════════════════════════════════════════════════════════════
        // Constructor
        // ══════════════════════════════════════════════════════════════════════

        public LibraryViewModel()
        {
            _storageService = new StorageService();
            _apiService     = new RawgApiService();

            SetRatingCommand = new RelayCommand(param =>
            {
                if (SelectedGame != null && param != null)
                {
                    if (double.TryParse(param.ToString(), System.Globalization.NumberStyles.Any, System.Globalization.CultureInfo.InvariantCulture, out double val))
                    {
                        SelectedGame.Rating = val;
                    }
                }
            });

            LoadLibrary();

            // ── Edit dialog ────────────────────────────────────────────────
            EditGameCommand = new RelayCommand(game =>
            {
                SelectedGame = game as Game;
                IsDialogOpen = true;
            });

            SaveGameCommand = new RelayCommand(async _ =>
            {
                if (SelectedGame != null)
                {
                    // Persist only Library-source games (safe merge)
                    var libraryGames = await Task.Run(() => _storageService.LoadLibraryGames());
                    var existing = libraryGames.FirstOrDefault(g => g.Id == SelectedGame.Id);
                    if (existing != null)
                    {
                        existing.Title = SelectedGame.Title;
                        existing.CoverUrl = SelectedGame.CoverUrl;
                        existing.Playtime = SelectedGame.Playtime;
                        existing.Status = SelectedGame.Status;
                        existing.Rating = SelectedGame.Rating;
                        existing.Genres = SelectedGame.Genres;
                        existing.Notes = SelectedGame.Notes;
                    }
                    var saveTask = Task.Run(() => _storageService.SaveSubset(libraryGames, "Library"));
                    
                    IsDialogOpen = false; // Hide dialog immediately
                    
                    await saveTask; // Wait for save to finish
                    await ApplySortAsync();
                }
            });

            CloseDialogCommand = new RelayCommand(async _ =>
            {
                IsDialogOpen = false; // Hide dialog immediately
                await ApplySortAsync(); // Revert unsaved edits from disk
            });

            DeleteGameCommand = new RelayCommand(async game =>
            {
                if (game is Game g)
                {
                    var libraryGames = await Task.Run(() => _storageService.LoadLibraryGames());
                    libraryGames.RemoveAll(x => x.Id == g.Id);
                    await Task.Run(() => _storageService.SaveSubset(libraryGames, "Library"));
                    await ApplySortAsync();
                }
            });

            // ── Add-to-Library dialog ──────────────────────────────────────
            OpenAddDialogCommand = new RelayCommand(_ =>
            {
                AddSearchQuery = string.Empty;
                AddSearchResults.Clear();
                IsAddSearchRunning = false;
                IsAddDialogOpen = true;
            });

            CloseAddDialogCommand = new RelayCommand(_ =>
            {
                IsAddDialogOpen = false;
                AddSearchResults.Clear();
                AddSearchQuery = string.Empty;
            });

            AddSearchResultToLibraryCommand = new RelayCommand(async game =>
            {
                if (game is Game newGame)
                {
                    var libraryGames = await Task.Run(() => _storageService.LoadLibraryGames());

                    bool alreadyInLibrary = libraryGames.Any(g => 
                        g.Id == newGame.Id || 
                        string.Equals(g.Title?.Trim(), newGame.Title?.Trim(), StringComparison.OrdinalIgnoreCase));

                    if (!alreadyInLibrary)
                    {
                        newGame.Status  = "Like to Play";
                        newGame.Source  = "Library"; // strict segregation tag
                        libraryGames.Add(newGame);
                        await Task.Run(() => _storageService.SaveSubset(libraryGames, "Library"));
                        await ApplySortAsync(); // refresh the card grid
                    }

                    // Visual feedback in the search list
                    newGame.Title = "✓ " + newGame.Title;
                }
            });

            _refreshTimer = new System.Windows.Threading.DispatcherTimer();
            _refreshTimer.Interval = TimeSpan.FromSeconds(5);
            _refreshTimer.Tick += async (s, e) => await RefreshGameStatsAsync();
            _refreshTimer.Start();
        }

        // ══════════════════════════════════════════════════════════════════════
        // Private helpers
        // ══════════════════════════════════════════════════════════════════════

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
            "A-Z", "Recently Added", "Playing", "Like to Play", "Stopped", "Completed", "Ratings"
        };

        private async Task ApplySortAsync()
        {
            var games = await Task.Run(() => _storageService.LoadLibraryGames());
            
            System.Collections.Generic.IEnumerable<Game> queryable = games;

            // Filter by search query
            if (!string.IsNullOrWhiteSpace(LibrarySearchQuery))
            {
                var query = LibrarySearchQuery.Trim();
                queryable = queryable.Where(g => g.Title != null && g.Title.Contains(query, System.StringComparison.OrdinalIgnoreCase));
            }

            System.Collections.Generic.IEnumerable<Game> sorted;

            switch (SelectedSortOption)
            {
                case "A-Z":
                    sorted = queryable.OrderBy(g => g.Title);
                    break;
                case "Recently Added":
                    sorted = queryable.OrderByDescending(g => g.Id);
                    break;
                case "Playing":
                    sorted = queryable.Where(g => g.Status == "Playing").OrderBy(g => g.Title);
                    break;
                case "Like to Play":
                    sorted = queryable.Where(g => g.Status == "Like to Play").OrderBy(g => g.Title);
                    break;
                case "Stopped":
                    sorted = queryable.Where(g => g.Status == "Stopped").OrderBy(g => g.Title);
                    break;
                case "Completed":
                    sorted = queryable.Where(g => g.Status == "Completed").OrderBy(g => g.Title);
                    break;
                case "Ratings":
                    sorted = queryable.OrderByDescending(g => g.Rating).ThenBy(g => g.Title);
                    break;
                default:
                    sorted = queryable.OrderBy(g => g.Title);
                    break;
            }

            var list = sorted.ToList();
            System.Windows.Application.Current.Dispatcher.Invoke(() =>
            {
                bool sequenceEqual = MyGames.Count == list.Count && 
                                     MyGames.Select(g => g.Id).SequenceEqual(list.Select(g => g.Id));
                if (!sequenceEqual)
                {
                    MyGames.ReplaceRange(list);
                }
            });
        }

        private void LoadLibrary()
        {
            _ = ApplySortAsync();
        }

        private async void DebouncedAddSearch()
        {
            _addSearchCts?.Cancel();
            _addSearchCts = new CancellationTokenSource();
            var token = _addSearchCts.Token;

            try
            {
                await Task.Delay(500, token);
                if (!token.IsCancellationRequested)
                    await ExecuteAddSearchAsync(AddSearchQuery, token);
            }
            catch (TaskCanceledException) { }
            catch { AddSearchResults.Clear(); }
        }

        private async Task ExecuteAddSearchAsync(string query, CancellationToken token)
        {
            if (string.IsNullOrWhiteSpace(query))
            {
                System.Windows.Application.Current.Dispatcher.Invoke(() => AddSearchResults.Clear());
                return;
            }

            IsAddSearchRunning = true;
            try
            {
                var results = await _apiService.SearchGamesAsync(query);

                if (!token.IsCancellationRequested)
                {
                    System.Windows.Application.Current.Dispatcher.Invoke(() =>
                    {
                        AddSearchResults.Clear();
                        foreach (var g in results)
                            AddSearchResults.Add(g);
                    });
                }
            }
            catch { /* swallow API errors gracefully */ }
            finally { IsAddSearchRunning = false; }
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
                var games = await Task.Run(() => _storageService.LoadLibraryGames());
                
                var visibleGames = games;
                
                // If search query is active, filter visible list accordingly
                if (!string.IsNullOrWhiteSpace(LibrarySearchQuery))
                {
                    var query = LibrarySearchQuery.Trim();
                    visibleGames = visibleGames.Where(g => g.Title != null && g.Title.Contains(query, StringComparison.OrdinalIgnoreCase)).ToList();
                }

                bool needsRefresh = false;
                if (MyGames.Count != visibleGames.Count)
                {
                    needsRefresh = true;
                }
                else
                {
                    foreach (var g in visibleGames)
                    {
                        var existing = MyGames.FirstOrDefault(x => x.Id == g.Id);
                        if (existing == null)
                        {
                            needsRefresh = true;
                            break;
                        }
                        if (existing.Playtime != g.Playtime || existing.LastPlayed != g.LastPlayed)
                        {
                            needsRefresh = true;
                            break;
                        }
                    }
                }

                if (needsRefresh)
                {
                    await ApplySortAsync();
                }
            }
            catch {}
        }
    }
}
