using System.Windows.Input;
using MooDeX_New_Version_1._0.Helpers;
using MooDeX_New_Version_1._0.Services;
using System;
using System.Windows;

namespace MooDeX_New_Version_1._0.ViewModels
{
    public class MainViewModel : ViewModelBase
    {
        private object? _currentView;
        private string _activeTab = "Dashboard";

        public object? CurrentView
        {
            get => _currentView;
            set => SetProperty(ref _currentView, value);
        }

        public string ActiveTab
        {
            get => _activeTab;
            set => SetProperty(ref _activeTab, value);
        }

        public bool IsGamePlaying => ProcessMonitor.Instance?.CurrentlyPlayingGame != null;

        public string GameStatusTitle
        {
            get
            {
                if (ProcessMonitor.Instance?.CurrentlyPlayingGame != null)
                {
                    return ProcessMonitor.Instance.CurrentlyPlayingGame.Title;
                }
                var lastPlayed = ProcessMonitor.Instance?.GetLastPlayedGame();
                return lastPlayed != null ? lastPlayed.Title : "None";
            }
        }

        public ICommand NavigateToDashboardCommand { get; }
        public ICommand NavigateToLibraryCommand { get; }
        public ICommand NavigateToMyGamesCommand { get; }
        public ICommand NavigateToDiscoverCommand { get; }
        public ICommand NavigateToSettingsCommand { get; }

        public MainViewModel()
        {
            NavigateToDashboardCommand = new RelayCommand(_ => { CurrentView = new DashboardViewModel(); ActiveTab = "Dashboard"; });
            NavigateToLibraryCommand = new RelayCommand(_ => { CurrentView = new LibraryViewModel(); ActiveTab = "Library"; });
            NavigateToMyGamesCommand = new RelayCommand(_ => { CurrentView = new MyGamesViewModel(); ActiveTab = "MyGames"; });
            NavigateToDiscoverCommand = new RelayCommand(_ => { CurrentView = new DiscoverViewModel(); ActiveTab = "Discover"; });
            NavigateToSettingsCommand = new RelayCommand(_ => { CurrentView = new SettingsViewModel(); ActiveTab = "Settings"; });

            CurrentView = new DashboardViewModel(); // Default to Dashboard view

            if (ProcessMonitor.Instance != null)
            {
                ProcessMonitor.Instance.CurrentlyPlayingGameChanged += (s, e) =>
                {
                    System.Windows.Application.Current?.Dispatcher.Invoke(() =>
                    {
                        OnPropertyChanged(nameof(IsGamePlaying));
                        OnPropertyChanged(nameof(GameStatusTitle));
                    });
                };
            }
        }
    }
}
