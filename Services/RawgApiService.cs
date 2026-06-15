using System;
using System.Net.Http;
using System.Threading.Tasks;
using System.Text.Json;
using System.Collections.Generic;
using MooDeX_New_Version_1._0.Models;

namespace MooDeX_New_Version_1._0.Services
{
    public class RawgApiService
    {
        private const string API_KEY = "3e485ee2b0a54eae8eb1dcb3a93760ec";
        private readonly HttpClient _httpClient;

        public RawgApiService()
        {
            _httpClient = new HttpClient();
            _httpClient.BaseAddress = new Uri("https://api.rawg.io/api/");
        }

        public async Task<List<Game>> SearchGamesAsync(string query)
        {
            var url = $"games?key={API_KEY}&search={Uri.EscapeDataString(query)}";
            return await GetGamesByUrlAsync(url);
        }

        public async Task<List<Game>> GetTrendingGamesAsync()
        {
            var startDate = DateTime.Today.AddMonths(-6).ToString("yyyy-MM-dd");
            var endDate = DateTime.Today.ToString("yyyy-MM-dd");
            var url = $"games?key={API_KEY}&dates={startDate},{endDate}&ordering=-added&page_size=50";
            return await GetGamesByUrlAsync(url);
        }

        public async Task<List<Game>> GetUpcomingGamesAsync()
        {
            var startDate = DateTime.Today.AddDays(1).ToString("yyyy-MM-dd");
            var endDate = DateTime.Today.AddYears(1).ToString("yyyy-MM-dd");
            var url = $"games?key={API_KEY}&dates={startDate},{endDate}&ordering=-added&page_size=50";
            return await GetGamesByUrlAsync(url);
        }

        public async Task<List<Game>> GetPopularGamesAsync()
        {
            var url = $"games?key={API_KEY}&ordering=-added&page_size=50";
            return await GetGamesByUrlAsync(url);
        }

        private async Task<List<Game>> GetGamesByUrlAsync(string url)
        {
            var response = await _httpClient.GetAsync(url);

            if (response.IsSuccessStatusCode)
            {
                var json = await response.Content.ReadAsStringAsync();

                try 
                {
                    using (JsonDocument document = JsonDocument.Parse(json))
                    {
                        var root = document.RootElement;
                        if (root.TryGetProperty("results", out var resultsElement) && resultsElement.ValueKind == JsonValueKind.Array)
                        {
                            var games = new List<Game>();
                            foreach (var item in resultsElement.EnumerateArray())
                            {
                                int id = item.GetProperty("id").GetInt32();
                                string name = item.GetProperty("name").GetString() ?? string.Empty;

                                string coverUrl = string.Empty;
                                if (item.TryGetProperty("background_image", out var bgImageElement) && bgImageElement.ValueKind == JsonValueKind.String)
                                {
                                    coverUrl = bgImageElement.GetString() ?? string.Empty;
                                }

                                string releaseDate = string.Empty;
                                if (item.TryGetProperty("released", out var releasedElement) && releasedElement.ValueKind == JsonValueKind.String)
                                {
                                    releaseDate = releasedElement.GetString() ?? string.Empty;
                                }

                                games.Add(new Game
                                {
                                    Id = id,
                                    Title = name,
                                    CoverUrl = coverUrl,
                                    ReleaseDate = releaseDate,
                                    Status = "Discovered"
                                });
                            }
                            return games;
                        }
                    }
                }
                catch (JsonException ex)
                {
                    Console.WriteLine($"Error parsing JSON: {ex.Message}");
                }
            }

            return new List<Game>();
        }
    }
}
