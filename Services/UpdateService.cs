using System;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;
using MooDeX_New_Version_1._0.Models;

namespace MooDeX_New_Version_1._0.Services
{
    /// <summary>
    /// Handles checking for application updates by fetching and parsing
    /// a remote update manifest (update.json).
    /// 
    /// Design decisions:
    /// - Uses a single static HttpClient to avoid socket exhaustion.
    /// - 15-second timeout prevents the UI from hanging on slow networks.
    /// - All exceptions are caught and re-thrown as a single UpdateCheckException
    ///   so callers only need one catch block.
    /// </summary>
    public class UpdateService
    {
        // ─────────────────────────────────────────────────────────────
        //  IMPORTANT: Replace this URL with your actual hosted update.json location.
        //  Common options:
        //    - GitHub raw URL:  https://raw.githubusercontent.com/<user>/<repo>/main/update.json
        //    - Your own server: https://yourdomain.com/updates/update.json
        // ─────────────────────────────────────────────────────────────
        private const string UpdateManifestUrl =
            "https://raw.githubusercontent.com/InoshMatheesha/MooDeX/main/update.json";

        /// <summary>
        /// Static HttpClient instance — best practice per Microsoft docs.
        /// Creating a new HttpClient per request can exhaust available sockets.
        /// </summary>
        private static readonly HttpClient _httpClient = new()
        {
            Timeout = TimeSpan.FromSeconds(15)
        };

        /// <summary>
        /// Fetches the remote update.json and deserializes it into an UpdateInfo object.
        /// </summary>
        /// <returns>An UpdateInfo containing the latest version metadata.</returns>
        /// <exception cref="UpdateCheckException">
        /// Thrown when the network request fails, times out, or the response
        /// cannot be deserialized. The inner exception contains the original error.
        /// </exception>
        public async Task<UpdateInfo> CheckForUpdateAsync()
        {
            try
            {
                // Fetch the raw JSON from the remote server
                string json = await _httpClient.GetStringAsync(UpdateManifestUrl);

                // Deserialize into our model — case-insensitive to be forgiving
                var options = new JsonSerializerOptions
                {
                    PropertyNameCaseInsensitive = true
                };

                var updateInfo = JsonSerializer.Deserialize<UpdateInfo>(json, options);

                if (updateInfo == null)
                {
                    throw new UpdateCheckException(
                        "The update manifest was empty or could not be parsed.");
                }

                return updateInfo;
            }
            catch (UpdateCheckException)
            {
                // Don't wrap our own exception type again
                throw;
            }
            catch (HttpRequestException ex)
            {
                throw new UpdateCheckException(
                    "Could not reach the update server. Please check your internet connection.", ex);
            }
            catch (TaskCanceledException ex)
            {
                throw new UpdateCheckException(
                    "The update check timed out. Please try again later.", ex);
            }
            catch (JsonException ex)
            {
                throw new UpdateCheckException(
                    "The update manifest has an invalid format.", ex);
            }
            catch (Exception ex)
            {
                throw new UpdateCheckException(
                    $"An unexpected error occurred while checking for updates: {ex.Message}", ex);
            }
        }

        /// <summary>
        /// Compares the remote version against the currently running application version.
        /// Uses System.Version for reliable semantic version comparison.
        /// </summary>
        /// <param name="remoteVersionString">
        /// The version string from the update manifest (e.g., "1.2.0").
        /// </param>
        /// <returns>
        /// True if the remote version is strictly newer than the current version.
        /// Returns false if versions are equal or the remote version string is invalid.
        /// </returns>
        public bool IsNewerVersion(string remoteVersionString)
        {
            try
            {
                // Get the current application version from the assembly metadata
                var currentVersion = System.Reflection.Assembly
                    .GetExecutingAssembly()
                    .GetName()
                    .Version;

                // Fallback if assembly version is unavailable
                if (currentVersion == null)
                {
                    currentVersion = new Version(1, 0, 0);
                }

                var remoteVersion = Version.Parse(remoteVersionString);

                // Strictly greater — equal versions are NOT treated as an update
                return remoteVersion > currentVersion;
            }
            catch (Exception)
            {
                // If the remote version string is malformed, don't crash — just say "no update"
                System.Diagnostics.Debug.WriteLine(
                    $"[UpdateService] Failed to parse remote version: '{remoteVersionString}'");
                return false;
            }
        }
    }

    /// <summary>
    /// Custom exception type for update-check failures.
    /// Provides user-friendly messages that can be displayed directly in the UI.
    /// </summary>
    public class UpdateCheckException : Exception
    {
        public UpdateCheckException(string message) : base(message) { }
        public UpdateCheckException(string message, Exception innerException)
            : base(message, innerException) { }
    }
}
