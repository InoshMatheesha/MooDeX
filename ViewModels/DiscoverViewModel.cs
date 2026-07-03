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
    public class DiscoverViewModel : ViewModelBase
    {
        private readonly RawgApiService _apiService;
        private readonly StorageService _storageService;
        private string _searchQuery = string.Empty;
        private ObservableCollection<Game> _searchResults = new ObservableCollection<Game>();
        private CancellationTokenSource? _searchCts;
        private string _selectedTab = "Trending";
        private bool _isLoading;

        public DiscoverViewModel()
        {
            _apiService = new RawgApiService();
            _storageService = new StorageService();
            SearchCommand = new RelayCommand(async _ => await ExecuteSearchAsync(SearchQuery));
            AddToLibraryCommand = new RelayCommand(async parameter => await ExecuteAddToLibraryAsync(parameter));

            // Load default tab games on initialization
            _ = LoadTabGamesAsync();
        }

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

        public ObservableCollection<Game> SearchResults
        {
            get => _searchResults;
            set => SetProperty(ref _searchResults, value);
        }

        public string SelectedTab
        {
            get => _selectedTab;
            set
            {
                if (SetProperty(ref _selectedTab, value))
                {
                    // Clear search query when switching tabs to go back to the tab view
                    _searchQuery = string.Empty;
                    OnPropertyChanged(nameof(SearchQuery));
                    _ = LoadTabGamesAsync();
                }
            }
        }

        public bool IsLoading
        {
            get => _isLoading;
            set => SetProperty(ref _isLoading, value);
        }

        public ICommand SearchCommand { get; }
        public ICommand AddToLibraryCommand { get; }

        private async Task ExecuteAddToLibraryAsync(object? parameter)
        {
            if (parameter is Game newGame)
            {
                var libraryGames = await Task.Run(() => _storageService.LoadLibraryGames());

                // Prevent duplicate additions (by RAWG Id or Title)
                bool alreadyInLibrary = libraryGames.Any(g => 
                    g.Id == newGame.Id || 
                    string.Equals(g.Title?.Trim(), newGame.Title?.Trim(), StringComparison.OrdinalIgnoreCase));

                if (!alreadyInLibrary)
                {
                    newGame.Status = "Like to Play";
                    newGame.UserRating = 0;
                    newGame.Source = "Library"; // ← strict segregation tag
                    newGame.AddedDate = DateTime.Now.ToString("yyyy-MM-dd");

                    libraryGames.Add(newGame);
                    await Task.Run(() => _storageService.SaveSubset(libraryGames, "Library"));
                }

                // Visual feedback: set IsAdded to true
                newGame.IsAdded = true;

                // Immediately remove the added game from search results/discover view
                System.Windows.Application.Current.Dispatcher.Invoke(() =>
                {
                    SearchResults.Remove(newGame);
                });
            }
        }

        private async void DebouncedSearch()
        {
            _searchCts?.Cancel();
            _searchCts = new CancellationTokenSource();
            var token = _searchCts.Token;

            try
            {
                // Wait 600ms before executing the search to prevent spamming the API
                await Task.Delay(600, token);

                if (!token.IsCancellationRequested)
                {
                    await ExecuteSearchAsync(SearchQuery);
                }
            }
            catch (TaskCanceledException)
            {
                // Ignore cancellations
            }
            catch (System.Exception)
            {
                // Catch any other exceptions to prevent app crashes in async void
            }
        }

        private async Task ExecuteSearchAsync(string query)
        {
            if (string.IsNullOrWhiteSpace(query))
            {
                await LoadTabGamesAsync();
                return;
            }

            IsLoading = true;
            try
            {
                var results = await _apiService.SearchGamesAsync(query);

                var libraryGames = await Task.Run(() => _storageService.LoadLibraryGames());
                var libraryIds = new HashSet<int>(libraryGames.Select(g => g.Id));
                var libraryTitles = new HashSet<string>(libraryGames.Select(g => g.Title.Trim().ToLowerInvariant()));

                var filteredResults = results
                    .Where(g => !libraryIds.Contains(g.Id) && !libraryTitles.Contains(g.Title.Trim().ToLowerInvariant()))
                    .ToList();

                await MarkAddedGamesAsync(filteredResults);

                // Update UI Collection strictly on the UI working thread to prevent crashes
                System.Windows.Application.Current.Dispatcher.Invoke(() =>
                {
                    SearchResults.Clear();
                    foreach (var game in filteredResults)
                    {
                        SearchResults.Add(game);
                    }
                });
            }
            catch (System.Exception)
            {
                // Optionally handle API/Connection timeouts here gracefully
            }
            finally
            {
                IsLoading = false;
            }
        }

        public async Task LoadTabGamesAsync()
        {
            IsLoading = true;
            try
            {
                List<Game> games;
                switch (SelectedTab)
                {
                    case "Trending":
                        games = await _apiService.GetTrendingGamesAsync();
                        break;
                    case "Upcoming":
                        games = await _apiService.GetUpcomingGamesAsync();
                        break;
                    case "Popular":
                        games = await _apiService.GetPopularGamesAsync();
                        break;
                    default:
                        games = new List<Game>();
                        break;
                }

                var libraryGames = await Task.Run(() => _storageService.LoadLibraryGames());
                var libraryIds = new HashSet<int>(libraryGames.Select(g => g.Id));
                var libraryTitles = new HashSet<string>(libraryGames.Select(g => g.Title.Trim().ToLowerInvariant()));

                var filteredGames = games
                    .Where(g => !libraryIds.Contains(g.Id) && !libraryTitles.Contains(g.Title.Trim().ToLowerInvariant()))
                    .Take(20)
                    .ToList();

                await MarkAddedGamesAsync(filteredGames);

                System.Windows.Application.Current.Dispatcher.Invoke(() =>
                {
                    SearchResults.Clear();
                    foreach (var game in filteredGames)
                    {
                        SearchResults.Add(game);
                    }
                });
            }
            catch (System.Exception)
            {
                // Handle API error
            }
            finally
            {
                IsLoading = false;
            }
        }

        private async Task MarkAddedGamesAsync(IEnumerable<Game> games)
        {
            var libraryGames = await Task.Run(() => _storageService.LoadLibraryGames());
            var libraryIds = new HashSet<int>(libraryGames.Select(g => g.Id));
            var libraryTitles = new HashSet<string>(libraryGames.Select(g => g.Title.Trim().ToLowerInvariant()));
            foreach (var g in games)
            {
                g.IsAdded = libraryIds.Contains(g.Id) || libraryTitles.Contains(g.Title.Trim().ToLowerInvariant());
            }
        }
    }
}
