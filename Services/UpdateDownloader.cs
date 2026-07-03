using System;
using System.IO;
using System.Net.Http;
using System.Threading;
using System.Threading.Tasks;

namespace MooDeX_New_Version_1._0.Services
{
    /// <summary>
    /// Downloads the update installer with real-time progress reporting.
    /// 
    /// Design decisions:
    /// - Uses HttpCompletionOption.ResponseHeadersRead so we can start reading
    ///   the response stream immediately without buffering the entire file in memory.
    /// - Downloads to a .tmp file first, then renames on success (atomic write pattern)
    ///   to avoid leaving a corrupt partial file if the download is interrupted.
    /// - Reports progress as a percentage (0.0 to 100.0) via IProgress&lt;double&gt;.
    /// - Supports CancellationToken for user-initiated cancellation.
    /// </summary>
    public class UpdateDownloader
    {
        /// <summary>
        /// Buffer size for reading the download stream.
        /// 8 KB is a good balance between memory usage and read syscall overhead.
        /// </summary>
        private const int BufferSize = 8192;

        /// <summary>
        /// Static HttpClient instance — shared across download calls.
        /// Timeout is disabled here because large file downloads can legitimately
        /// take a long time; cancellation is handled via CancellationToken instead.
        /// </summary>
        private static readonly HttpClient _httpClient = new()
        {
            Timeout = Timeout.InfiniteTimeSpan
        };

        /// <summary>
        /// Downloads a file from the given URL to the specified destination path,
        /// reporting progress as a percentage.
        /// </summary>
        /// <param name="url">The direct download URL for the installer.</param>
        /// <param name="destinationPath">
        /// The full local file path where the downloaded file will be saved
        /// (e.g., "C:\Users\...\AppData\Local\Temp\MooDeXSetup.exe").
        /// </param>
        /// <param name="progress">
        /// An IProgress&lt;double&gt; that receives percentage values from 0.0 to 100.0.
        /// Typically bound to a ProgressBar in the UI.
        /// </param>
        /// <param name="cancellationToken">
        /// Token to cancel the download. When cancelled, the partial temp file is deleted.
        /// </param>
        /// <exception cref="HttpRequestException">Thrown on network failures.</exception>
        /// <exception cref="OperationCanceledException">Thrown when the user cancels.</exception>
        public async Task DownloadAsync(
            string url,
            string destinationPath,
            IProgress<double> progress,
            CancellationToken cancellationToken = default)
        {
            // Download to a temporary file first — prevents leaving a corrupt file on failure
            string tempPath = destinationPath + ".tmp";

            try
            {
                // Send the request but only read the headers initially
                // This lets us access Content-Length before downloading the body
                using var response = await _httpClient.GetAsync(
                    url,
                    HttpCompletionOption.ResponseHeadersRead,
                    cancellationToken);

                // Ensure we got a success status code (200 OK)
                response.EnsureSuccessStatusCode();

                // Try to get the total file size for progress calculation
                // Some servers may not provide Content-Length — in that case we show indeterminate progress
                long? totalBytes = response.Content.Headers.ContentLength;

                // Ensure the destination directory exists
                string? directory = Path.GetDirectoryName(destinationPath);
                if (!string.IsNullOrEmpty(directory))
                {
                    Directory.CreateDirectory(directory);
                }

                // Open the response stream and a file stream for writing
                await using var contentStream = await response.Content.ReadAsStreamAsync(cancellationToken);
                await using var fileStream = new FileStream(
                    tempPath,
                    FileMode.Create,
                    FileAccess.Write,
                    FileShare.None,
                    BufferSize,
                    useAsync: true);

                long totalBytesRead = 0;
                var buffer = new byte[BufferSize];
                int bytesRead;

                // Read the download stream in chunks, reporting progress after each chunk
                while ((bytesRead = await contentStream.ReadAsync(
                    buffer, 0, buffer.Length, cancellationToken)) > 0)
                {
                    await fileStream.WriteAsync(buffer, 0, bytesRead, cancellationToken);
                    totalBytesRead += bytesRead;

                    // Calculate and report progress percentage
                    if (totalBytes.HasValue && totalBytes.Value > 0)
                    {
                        double percentage = (double)totalBytesRead / totalBytes.Value * 100.0;
                        progress.Report(percentage);
                    }
                    else
                    {
                        // If Content-Length is unknown, report -1 to indicate indeterminate
                        progress.Report(-1);
                    }
                }

                // Ensure all data is flushed to disk before renaming
                await fileStream.FlushAsync(cancellationToken);

                // Close the file stream explicitly before moving the file
                await fileStream.DisposeAsync();

                // Atomic rename: delete existing destination if present, then move temp file
                if (File.Exists(destinationPath))
                {
                    File.Delete(destinationPath);
                }

                File.Move(tempPath, destinationPath);

                // Report 100% completion
                progress.Report(100.0);
            }
            catch
            {
                // Clean up the partial temp file on any failure
                try
                {
                    if (File.Exists(tempPath))
                    {
                        File.Delete(tempPath);
                    }
                }
                catch
                {
                    // Swallow cleanup errors — the original exception is more important
                }

                throw; // Re-throw the original exception to the caller
            }
        }
    }
}
