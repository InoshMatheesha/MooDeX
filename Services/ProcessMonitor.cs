using System;
using System.Diagnostics;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using MooDeX_New_Version_1._0.Models;
using System.Collections.Generic;

namespace MooDeX_New_Version_1._0.Services
{
    public class ProcessMonitor
    {
        private readonly StorageService _storageService;
        private CancellationTokenSource? _cancellationTokenSource;
        private DateTime _lastScanTime = DateTime.Now;
        private readonly Dictionary<int, Process> _trackedProcesses = new();
        private readonly HashSet<int> _previouslyRunningIds = new();

        public static ProcessMonitor? Instance { get; private set; }

        private Game? _currentlyPlayingGame;
        public Game? CurrentlyPlayingGame
        {
            get => _currentlyPlayingGame;
            private set
            {
                if (_currentlyPlayingGame?.Title != value?.Title)
                {
                    _currentlyPlayingGame = value;
                    CurrentlyPlayingGameChanged?.Invoke(this, EventArgs.Empty);
                }
            }
        }

        public string StatusText
        {
            get
            {
                if (CurrentlyPlayingGame != null)
                {
                    return $"Playing: {CurrentlyPlayingGame.Title}";
                }

                var lastPlayed = GetLastPlayedGame();
                return lastPlayed != null ? $"Last Played: {lastPlayed.Title}" : "No games played yet";
            }
        }

        public event EventHandler? CurrentlyPlayingGameChanged;

        public ProcessMonitor(StorageService storageService)
        {
            _storageService = storageService;
            Instance = this;
        }

        public void RegisterProcess(int gameId, Process process)
        {
            lock (_lock)
            {
                _trackedProcesses[gameId] = process;
                _previouslyRunningIds.Add(gameId);
            }
        }

        public void StartMonitoring()
        {
            _cancellationTokenSource = new CancellationTokenSource();
            var token = _cancellationTokenSource.Token;
            _lastScanTime = DateTime.Now;

            Task.Run(async () =>
            {
                while (!token.IsCancellationRequested)
                {
                    CheckProcesses();
                    await Task.Delay(TimeSpan.FromSeconds(10), token);
                }
            }, token);
        }

        public void StopMonitoring()
        {
            _cancellationTokenSource?.Cancel();
        }

        public void ForceCheck()
        {
            CheckProcesses();
        }

        public Game? GetLastPlayedGame()
        {
            try
            {
                var games = _storageService.LoadGames();
                return games
                    .Where(g => g.LastPlayed != null)
                    .OrderByDescending(g => g.LastPlayed)
                    .FirstOrDefault();
            }
            catch
            {
                return null;
            }
        }

        private readonly object _lock = new object();

        private void CheckProcesses()
        {
            lock (_lock)
            {
                var games = _storageService.LoadGames();
                bool updated = false;

                var now = DateTime.Now;
                var elapsedMinutes = (now - _lastScanTime).TotalMinutes;
                _lastScanTime = now;

                // Handle system sleep/resume or thread delays
                if (elapsedMinutes < 0) elapsedMinutes = 0;
                if (elapsedMinutes > 5.0) elapsedMinutes = 10.0 / 60.0; 

                var runningProcesses = Process.GetProcesses().Select(p => p.ProcessName.ToLower()).ToHashSet();
                Game? currentPlaying = null;
                var currentlyRunningIds = new HashSet<int>();

                foreach (var game in games)
                {
                    bool isRunning = false;

                    // 1. Check if we have a directly tracked process
                    if (_trackedProcesses.TryGetValue(game.Id, out var proc))
                    {
                        try
                        {
                            if (!proc.HasExited)
                            {
                                isRunning = true;
                            }
                            else
                            {
                                _trackedProcesses.Remove(game.Id);
                            }
                        }
                        catch
                        {
                            _trackedProcesses.Remove(game.Id);
                        }
                    }

                    // 2. Fallback to process name match
                    if (!isRunning && !string.IsNullOrEmpty(game.ExecutableName))
                    {
                        var cleanExeName = game.ExecutableName.Replace(".exe", "").ToLower();
                        isRunning = runningProcesses.Any(p => 
                            p == cleanExeName || 
                            (cleanExeName.Length > 3 && (p.StartsWith(cleanExeName) || p.Contains(cleanExeName)))
                        );
                    }

                    if (isRunning)
                    {
                        currentlyRunningIds.Add(game.Id);

                        // Increment play count if just started running
                        if (!_previouslyRunningIds.Contains(game.Id))
                        {
                            game.PlayCount++;
                            updated = true;
                        }

                        if (elapsedMinutes > 0)
                        {
                            game.Playtime += elapsedMinutes;
                        }
                        game.LastPlayed = now;
                        updated = true;
                        currentPlaying = game;
                    }
                }

                // Update previously running ids
                _previouslyRunningIds.Clear();
                foreach (var id in currentlyRunningIds)
                {
                    _previouslyRunningIds.Add(id);
                }

                if (updated)
                {
                    _storageService.SaveGames(games);
                }

                CurrentlyPlayingGame = currentPlaying;
            }
        }
    }
}
