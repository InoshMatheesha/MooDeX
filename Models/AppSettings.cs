using System.Text.Json.Serialization;

namespace MooDeX_New_Version_1._0.Models
{
    public class AppSettings
    {
        [JsonPropertyName("close_to_tray")]
        public bool MinimizeToTray { get; set; } = false;

        [JsonPropertyName("autostart")]
        public bool LaunchAtStartup { get; set; } = false;

        [JsonPropertyName("lazy_discover")]
        public bool LazyDiscover { get; set; } = true;

        [JsonPropertyName("api_cache")]
        public bool ApiCache { get; set; } = true;

        [JsonPropertyName("launch_minimized")]
        public bool LaunchMinimized { get; set; } = false;

        [JsonPropertyName("goat_game_title")]
        public string GoatGameTitle { get; set; } = string.Empty;

        [JsonPropertyName("window_width")]
        public double WindowWidth { get; set; } = 1340;

        [JsonPropertyName("window_height")]
        public double WindowHeight { get; set; } = 820;
    }
}
