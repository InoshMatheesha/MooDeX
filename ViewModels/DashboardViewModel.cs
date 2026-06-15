using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;
using System.Threading.Tasks;
using System.Windows.Input;
using MooDeX_New_Version_1._0.Helpers;
using MooDeX_New_Version_1._0.Models;
using MooDeX_New_Version_1._0.Services;

namespace MooDeX_New_Version_1._0.ViewModels
{
    public class DashboardViewModel : ViewModelBase
    {
        private readonly StorageService _storageService;
        private int _totalGames;
        private string _totalPlaytimeFormatted = "0m";
        private int _playingCount;
        private int _completedCount;
        private int _likeToPlayCount;
        private int _stoppedCount;

        private Game? _topGame1;
        private Game? _topGame2;
        private Game? _topGame3;
        private Game? _goatGame;
        private bool _isSelectingGoat;

        private ObservableCollection<Game> _uniqueGames = new();

        public int TotalGames
        {
            get => _totalGames;
            set => SetProperty(ref _totalGames, value);
        }

        public string TotalPlaytimeFormatted
        {
            get => _totalPlaytimeFormatted;
            set => SetProperty(ref _totalPlaytimeFormatted, value);
        }

        public int PlayingCount
        {
            get => _playingCount;
            set => SetProperty(ref _playingCount, value);
        }

        public int CompletedCount
        {
            get => _completedCount;
            set => SetProperty(ref _completedCount, value);
        }

        public int LikeToPlayCount
        {
            get => _likeToPlayCount;
            set => SetProperty(ref _likeToPlayCount, value);
        }

        public int StoppedCount
        {
            get => _stoppedCount;
            set => SetProperty(ref _stoppedCount, value);
        }

        public Game? TopGame1
        {
            get => _topGame1;
            set
            {
                if (SetProperty(ref _topGame1, value))
                {
                    OnPropertyChanged(nameof(HasTopGame1));
                }
            }
        }

        public bool HasTopGame1 => TopGame1 != null;

        public Game? TopGame2
        {
            get => _topGame2;
            set
            {
                if (SetProperty(ref _topGame2, value))
                {
                    OnPropertyChanged(nameof(HasTopGame2));
                }
            }
        }

        public bool HasTopGame2 => TopGame2 != null;

        public Game? TopGame3
        {
            get => _topGame3;
            set
            {
                if (SetProperty(ref _topGame3, value))
                {
                    OnPropertyChanged(nameof(HasTopGame3));
                }
            }
        }

        public bool HasTopGame3 => TopGame3 != null;

        public Game? GoatGame
        {
            get => _goatGame;
            set
            {
                if (SetProperty(ref _goatGame, value))
                {
                    SaveGoatGame(value);
                    _isSelectingGoat = false;
                    OnPropertyChanged(nameof(IsSelectingGoat));
                    OnPropertyChanged(nameof(ShowGoatDetails));
                    OnPropertyChanged(nameof(ShowCancelGoatButton));
                }
            }
        }

        public bool IsSelectingGoat
        {
            get => _isSelectingGoat || _goatGame == null;
            set
            {
                if (SetProperty(ref _isSelectingGoat, value))
                {
                    OnPropertyChanged(nameof(ShowGoatDetails));
                    OnPropertyChanged(nameof(ShowCancelGoatButton));
                }
            }
        }

        public bool ShowGoatDetails => GoatGame != null && !IsSelectingGoat;

        public bool ShowCancelGoatButton => GoatGame != null && IsSelectingGoat;

        public ICommand ChangeGoatCommand { get; }
        public ICommand CancelChangeGoatCommand { get; }

        public ObservableCollection<Game> UniqueGames
        {
            get => _uniqueGames;
            set => SetProperty(ref _uniqueGames, value);
        }

        public DashboardViewModel()
        {
            _storageService = new StorageService();
            
            ChangeGoatCommand = new RelayCommand(_ => {
                IsSelectingGoat = true;
            });
            
            CancelChangeGoatCommand = new RelayCommand(_ => {
                IsSelectingGoat = false;
            });

            _ = LoadStatsAsync();
        }

        private async Task LoadStatsAsync()
        {
            var games = await Task.Run(() => _storageService.LoadGames());
            
            // Get unique list of games based on title to avoid double counting local vs library duplicates
            var uniqueList = games
                .GroupBy(g => g.Title.Trim().ToLower())
                .Select(g => g.First())
                .ToList();

            TotalGames = uniqueList.Count;

            double totalMinutes = uniqueList.Sum(g => g.Playtime);
            int hours = (int)(totalMinutes / 60);
            int minutes = (int)(Math.Round(totalMinutes) % 60);
            TotalPlaytimeFormatted = hours > 0 ? $"{hours}h {minutes}m" : $"{minutes}m";

            PlayingCount = uniqueList.Count(g => g.Status.Equals("Playing", StringComparison.OrdinalIgnoreCase));
            CompletedCount = uniqueList.Count(g => g.Status.Equals("Completed", StringComparison.OrdinalIgnoreCase));
            LikeToPlayCount = uniqueList.Count(g => g.Status.Equals("Like to Play", StringComparison.OrdinalIgnoreCase));
            StoppedCount = uniqueList.Count(g => g.Status.Equals("Stopped", StringComparison.OrdinalIgnoreCase));

            // Load top 3 most played games
            var topGamesList = uniqueList
                .OrderByDescending(g => g.Playtime)
                .ThenByDescending(g => g.PlayCount)
                .Take(3)
                .ToList();

            TopGame1 = topGamesList.Count > 0 ? topGamesList[0] : null;
            TopGame2 = topGamesList.Count > 1 ? topGamesList[1] : null;
            TopGame3 = topGamesList.Count > 2 ? topGamesList[2] : null;

            // Populate unique games list for the selection dropdown
            System.Windows.Application.Current.Dispatcher.Invoke(() =>
            {
                UniqueGames.Clear();
                foreach (var ug in uniqueList.OrderBy(g => g.Title))
                {
                    UniqueGames.Add(ug);
                }
            });

            // Load saved G.O.A.T. game setting
            var settings = _storageService.LoadSettings();
            var savedGoatTitle = settings.GoatGameTitle;
            if (!string.IsNullOrEmpty(savedGoatTitle))
            {
                var matched = uniqueList.FirstOrDefault(g => string.Equals(g.Title?.Trim(), savedGoatTitle.Trim(), StringComparison.OrdinalIgnoreCase));
                if (matched != null)
                {
                    System.Windows.Application.Current.Dispatcher.Invoke(() =>
                    {
                        GoatGame = matched;
                    });
                }
            }
        }

        private void SaveGoatGame(Game? game)
        {
            try
            {
                var settings = _storageService.LoadSettings();
                settings.GoatGameTitle = game?.Title ?? string.Empty;
                _storageService.SaveSettings(settings);
            }
            catch {}
        }
    }
}
