using System.Text.Json.Serialization;

namespace MooDeX_New_Version_1._0.Models
{
    /// <summary>
    /// Represents the metadata for an available application update.
    /// This model is deserialized from the remote update.json manifest file.
    /// </summary>
    public class UpdateInfo
    {
        /// <summary>
        /// The latest available version string (e.g., "1.2.0").
        /// </summary>
        [JsonPropertyName("version")]
        public string Version { get; set; } = string.Empty;

        /// <summary>
        /// Human-readable release notes describing what changed in this version.
        /// Supports newline characters for multi-line notes.
        /// </summary>
        [JsonPropertyName("release_notes")]
        public string ReleaseNotes { get; set; } = string.Empty;

        /// <summary>
        /// Direct download URL for the installer executable (MooDeXSetup.exe).
        /// </summary>
        [JsonPropertyName("download_url")]
        public string DownloadUrl { get; set; } = string.Empty;

        /// <summary>
        /// The date this version was published (e.g., "2026-07-01").
        /// Used for display purposes only.
        /// </summary>
        [JsonPropertyName("published_at")]
        public string PublishedAt { get; set; } = string.Empty;
    }
}
