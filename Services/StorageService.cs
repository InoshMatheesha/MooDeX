using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Serialization;
using MooDeX_New_Version_1._0.Models;

namespace MooDeX_New_Version_1._0.Services
{
    public class StorageService
    {
        private readonly string _filePath;
        private readonly string _settingsPath;

        public StorageService()
        {
            var appDataPath = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
            var appFolder = Path.Combine(appDataPath, "MooDeX");
            if (!Directory.Exists(appFolder))
            {
                Directory.CreateDirectory(appFolder);
            }
            _filePath = Path.Combine(appFolder, "games_data.json");
            _settingsPath = Path.Combine(appFolder, "user_settings.json");
        }

        public List<Game> LoadGames()
        {
            if (!File.Exists(_filePath))
                return new List<Game>();

            try
            {
                var json = File.ReadAllText(_filePath);
                var wrapper = JsonSerializer.Deserialize<GamesDataWrapper>(json) ?? new GamesDataWrapper();
                
                bool needsSave = false;
                int maxId = 0;

                // Find max non-zero ID across all lists to handle auto-increment
                if (wrapper.Games != null)
                {
                    foreach (var g in wrapper.Games)
                    {
                        if (g.Id > maxId) maxId = g.Id;
                    }
                }
                if (wrapper.LocalGames != null)
                {
                    foreach (var g in wrapper.LocalGames)
                    {
                        if (g.Id > maxId) maxId = g.Id;
                    }
                }

                if (wrapper.Games != null)
                {
                    foreach (var g in wrapper.Games)
                    {
                        g.Source = "Library";
                        g.IsLocal = false;
                        if (g.Id == 0)
                        {
                            maxId++;
                            g.Id = maxId;
                            needsSave = true;
                        }
                    }
                }
                if (wrapper.LocalGames != null)
                {
                    foreach (var g in wrapper.LocalGames)
                    {
                        g.Source = "PCGames";
                        g.IsLocal = true;
                        if (g.Id == 0)
                        {
                            maxId++;
                            g.Id = maxId;
                            needsSave = true;
                        }
                    }
                }

                // ── Auto-create missing library entries for local games ───────────
                if (wrapper.Games != null && wrapper.LocalGames != null)
                {
                    foreach (var pcGame in wrapper.LocalGames)
                    {
                        bool existsInLibrary = wrapper.Games.Any(g => 
                            (pcGame.Id != 0 && g.Id != 0 && pcGame.Id == g.Id) || 
                            string.Equals(pcGame.Title?.Trim(), g.Title?.Trim(), StringComparison.OrdinalIgnoreCase));

                        if (!existsInLibrary)
                        {
                            maxId++;
                            var newLibGame = new Game
                            {
                                Id = pcGame.Id != 0 ? pcGame.Id : maxId,
                                Title = pcGame.Title,
                                CoverUrl = pcGame.CoverUrl,
                                Playtime = pcGame.Playtime,
                                Status = "Like to Play",
                                Source = "Library",
                                Genres = pcGame.Genres,
                                Notes = pcGame.Notes,
                                IsLocal = false,
                                AddedDate = pcGame.AddedDate,
                                ReleaseDate = pcGame.ReleaseDate,
                                LastPlayed = pcGame.LastPlayed
                            };
                            
                            if (pcGame.Id == 0)
                            {
                                pcGame.Id = newLibGame.Id;
                            }
                            
                            wrapper.Games.Add(newLibGame);
                            needsSave = true;
                        }
                    }
                }

                // ── Synchronize Playtime & LastPlayed on load ─────────────────────
                if (wrapper.Games != null && wrapper.LocalGames != null)
                {
                    foreach (var localGame in wrapper.LocalGames)
                    {
                        var libGame = wrapper.Games.FirstOrDefault(g => 
                            (localGame.Id != 0 && g.Id != 0 && localGame.Id == g.Id) ||
                            string.Equals(g.Title?.Trim(), localGame.Title?.Trim(), StringComparison.OrdinalIgnoreCase));
                        
                        if (libGame != null)
                        {
                            var maxPlaytime = Math.Max(localGame.Playtime, libGame.Playtime);
                            if (localGame.Playtime != maxPlaytime || libGame.Playtime != maxPlaytime)
                            {
                                localGame.Playtime = maxPlaytime;
                                libGame.Playtime = maxPlaytime;
                                needsSave = true;
                            }

                            var maxPlayCount = Math.Max(localGame.PlayCount, libGame.PlayCount);
                            if (localGame.PlayCount != maxPlayCount || libGame.PlayCount != maxPlayCount)
                            {
                                localGame.PlayCount = maxPlayCount;
                                libGame.PlayCount = maxPlayCount;
                                needsSave = true;
                            }

                            DateTime? maxLastPlayed = null;
                            if (localGame.LastPlayed != null && libGame.LastPlayed != null)
                            {
                                maxLastPlayed = localGame.LastPlayed > libGame.LastPlayed ? localGame.LastPlayed : libGame.LastPlayed;
                            }
                            else
                            {
                                maxLastPlayed = localGame.LastPlayed ?? libGame.LastPlayed;
                            }

                            if (localGame.LastPlayed != maxLastPlayed || libGame.LastPlayed != maxLastPlayed)
                            {
                                localGame.LastPlayed = maxLastPlayed;
                                libGame.LastPlayed = maxLastPlayed;
                                needsSave = true;
                            }
                        }
                    }
                }

                var all = new List<Game>();
                if (wrapper.Games != null) all.AddRange(wrapper.Games);
                if (wrapper.LocalGames != null) all.AddRange(wrapper.LocalGames);

                if (needsSave)
                {
                    // Directly serialize wrapper to save changes to disk without triggering recursion
                    var options = new JsonSerializerOptions { WriteIndented = true };
                    var jsonToSave = JsonSerializer.Serialize(wrapper, options);
                    File.WriteAllText(_filePath, jsonToSave);
                }

                return all;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error loading games: {ex.Message}");
                return new List<Game>();
            }
        }

        /// <summary>Returns only games added via Discover/search (Source == "Library").</summary>
        public List<Game> LoadLibraryGames() =>
            LoadGames().Where(g => g.Source == "Library").ToList();

        /// <summary>Returns only games manually imported via PC Games section (Source == "PCGames").</summary>
        public List<Game> LoadPCGames() =>
            LoadGames().Where(g => g.Source == "PCGames").ToList();

        public void SaveGames(List<Game> games)
        {
            try
            {
                GamesDataWrapper wrapper;
                if (File.Exists(_filePath))
                {
                    var existingJson = File.ReadAllText(_filePath);
                    wrapper = JsonSerializer.Deserialize<GamesDataWrapper>(existingJson) ?? new GamesDataWrapper();
                }
                else
                {
                    wrapper = new GamesDataWrapper();
                }

                wrapper.Games = games.Where(g => g.Source == "Library").ToList();
                wrapper.LocalGames = games.Where(g => g.Source == "PCGames").ToList();

                // ── Sync Playtime & LastPlayed prior to serialization ────────────
                foreach (var localGame in wrapper.LocalGames)
                {
                    var libGame = wrapper.Games.FirstOrDefault(g => 
                        (localGame.Id != 0 && g.Id != 0 && localGame.Id == g.Id) ||
                        string.Equals(g.Title?.Trim(), localGame.Title?.Trim(), StringComparison.OrdinalIgnoreCase));
                    
                    if (libGame != null)
                    {
                        var maxPlaytime = Math.Max(localGame.Playtime, libGame.Playtime);
                        localGame.Playtime = maxPlaytime;
                        libGame.Playtime = maxPlaytime;

                        var maxPlayCount = Math.Max(localGame.PlayCount, libGame.PlayCount);
                        localGame.PlayCount = maxPlayCount;
                        libGame.PlayCount = maxPlayCount;

                        DateTime? maxLastPlayed = null;
                        if (localGame.LastPlayed != null && libGame.LastPlayed != null)
                        {
                            maxLastPlayed = localGame.LastPlayed > libGame.LastPlayed ? localGame.LastPlayed : libGame.LastPlayed;
                        }
                        else
                        {
                            maxLastPlayed = localGame.LastPlayed ?? libGame.LastPlayed;
                        }
                        localGame.LastPlayed = maxLastPlayed;
                        libGame.LastPlayed = maxLastPlayed;
                    }
                }

                var options = new JsonSerializerOptions { WriteIndented = true };
                var json = JsonSerializer.Serialize(wrapper, options);
                File.WriteAllText(_filePath, json);
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error saving games: {ex.Message}");
            }
        }

        /// <summary>
        /// Merges an updated subset back into the full library without overwriting the other source's games,
        /// safeguarding against overwriting newer playtimes/last-played dates with stale UI states.
        /// </summary>
        public void SaveSubset(List<Game> updatedSubset, string source)
        {
            var all = LoadGames();

            // Preserve higher playtime and more recent last played from existing entries to prevent stale UI states from overwriting them
            foreach (var updatedGame in updatedSubset)
            {
                var existingGame = all.FirstOrDefault(g => 
                    g.Source == source && 
                    ((g.Id != 0 && updatedGame.Id != 0 && g.Id == updatedGame.Id) || 
                     string.Equals(g.Title?.Trim(), updatedGame.Title?.Trim(), StringComparison.OrdinalIgnoreCase)));
                
                if (existingGame != null)
                {
                    updatedGame.Playtime = Math.Max(updatedGame.Playtime, existingGame.Playtime);
                    updatedGame.PlayCount = Math.Max(updatedGame.PlayCount, existingGame.PlayCount);
                    if (existingGame.LastPlayed != null && (updatedGame.LastPlayed == null || existingGame.LastPlayed > updatedGame.LastPlayed))
                    {
                        updatedGame.LastPlayed = existingGame.LastPlayed;
                    }
                }
            }

            // Remove existing entries for this source, then add the updated ones
            all.RemoveAll(g => g.Source == source);
            all.AddRange(updatedSubset);
            SaveGames(all);
        }

        public AppSettings LoadSettings()
        {
            if (!File.Exists(_settingsPath))
                return new AppSettings();

            try
            {
                var json = File.ReadAllText(_settingsPath);
                return JsonSerializer.Deserialize<AppSettings>(json) ?? new AppSettings();
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error loading settings: {ex.Message}");
                return new AppSettings();
            }
        }

        public void SaveSettings(AppSettings settings)
        {
            try
            {
                var options = new JsonSerializerOptions { WriteIndented = true };
                var json = JsonSerializer.Serialize(settings, options);
                File.WriteAllText(_settingsPath, json);
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error saving settings: {ex.Message}");
            }
        }
    }

    public class GamesDataWrapper
    {
        [JsonPropertyName("games")]
        public List<Game> Games { get; set; } = new List<Game>();

        [JsonPropertyName("local_games")]
        public List<Game> LocalGames { get; set; } = new List<Game>();

        [JsonPropertyName("now_playing")]
        public System.Text.Json.Nodes.JsonNode? NowPlaying { get; set; }
    }
}
