using System;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Windows;
using System.Windows.Input;
using MooDeX_New_Version_1._0.Models;
using MooDeX_New_Version_1._0.Services;

namespace MooDeX_New_Version_1._0.Views
{
    /// <summary>
    /// Code-behind for the UpdateDialog window.
    /// 
    /// This dialog displays release notes for a new version and allows the user
    /// to either download + install the update or dismiss it. The download runs
    /// asynchronously with a visible progress bar and status text.
    /// 
    /// Flow:
    /// 1. Dialog opens → shows version, date, and release notes.
    /// 2. User clicks "Update Now" → progress bar appears, installer downloads.
    /// 3. Download completes → installer launches, application shuts down.
    /// 4. User clicks "Later" → dialog closes, no action taken.
    /// </summary>
    public partial class UpdateDialog : Window
    {
        private readonly UpdateInfo _updateInfo;
        private readonly UpdateDownloader _downloader;
        private CancellationTokenSource? _cancellationTokenSource;

        /// <summary>
        /// Indicates whether the user chose to proceed with the update.
        /// Set to true only after the installer is successfully launched.
        /// </summary>
        public bool UserAcceptedUpdate { get; private set; }

        /// <summary>
        /// Creates a new UpdateDialog for the given update information.
        /// </summary>
        /// <param name="updateInfo">The update metadata to display.</param>
        public UpdateDialog(UpdateInfo updateInfo)
        {
            InitializeComponent();

            _updateInfo = updateInfo ?? throw new ArgumentNullException(nameof(updateInfo));
            _downloader = new UpdateDownloader();

            // Populate the UI with update information
            VersionText.Text = $"v{_updateInfo.Version}";
            DateText.Text = !string.IsNullOrWhiteSpace(_updateInfo.PublishedAt)
                ? $"Released on {_updateInfo.PublishedAt}"
                : string.Empty;
            ReleaseNotesText.Text = !string.IsNullOrWhiteSpace(_updateInfo.ReleaseNotes)
                ? _updateInfo.ReleaseNotes
                : "No release notes provided.";
        }

        // ─────────────────────────────────────────────────────────────
        //  WINDOW CHROME — Drag support for the borderless window
        // ─────────────────────────────────────────────────────────────

        private void Header_MouseLeftButtonDown(object sender, MouseButtonEventArgs e)
        {
            // Allow the user to drag the window by clicking the header area
            if (e.LeftButton == MouseButtonState.Pressed)
            {
                DragMove();
            }
        }

        private void CloseButton_Click(object sender, RoutedEventArgs e)
        {
            // Cancel any in-progress download before closing
            _cancellationTokenSource?.Cancel();
            UserAcceptedUpdate = false;
            Close();
        }

        private void LaterButton_Click(object sender, RoutedEventArgs e)
        {
            // User chose to skip this update
            _cancellationTokenSource?.Cancel();
            UserAcceptedUpdate = false;
            Close();
        }

        // ─────────────────────────────────────────────────────────────
        //  UPDATE DOWNLOAD — Triggered by the "Update Now" button
        // ─────────────────────────────────────────────────────────────

        private async void UpdateNowButton_Click(object sender, RoutedEventArgs e)
        {
            // Prevent double-clicks
            UpdateNowButton.IsEnabled = false;
            LaterButton.IsEnabled = false;

            // Show the progress bar and status text
            DownloadProgressBar.Visibility = Visibility.Visible;
            StatusText.Visibility = Visibility.Visible;
            StatusText.Text = "Preparing download...";

            // Build the destination path in the user's temp folder
            string tempDir = Path.GetTempPath();
            string installerPath = Path.Combine(tempDir, "MooDeXSetup.exe");

            _cancellationTokenSource = new CancellationTokenSource();

            try
            {
                // Create a progress reporter that updates the UI on the dispatcher thread
                var progress = new Progress<double>(percentage =>
                {
                    if (percentage < 0)
                    {
                        // Indeterminate — server didn't provide Content-Length
                        DownloadProgressBar.IsIndeterminate = true;
                        StatusText.Text = "Downloading...";
                    }
                    else
                    {
                        DownloadProgressBar.IsIndeterminate = false;
                        DownloadProgressBar.Value = percentage;
                        StatusText.Text = $"Downloading... {percentage:F0}%";
                    }
                });

                // Perform the download asynchronously
                await _downloader.DownloadAsync(
                    _updateInfo.DownloadUrl,
                    installerPath,
                    progress,
                    _cancellationTokenSource.Token);

                // Download succeeded — launch the installer
                StatusText.Text = "Download complete. Launching installer...";

                // Start the installer process
                var startInfo = new ProcessStartInfo
                {
                    FileName = installerPath,
                    UseShellExecute = true // Required for elevation prompts (UAC)
                };
                Process.Start(startInfo);

                UserAcceptedUpdate = true;

                // Shut down the application so the installer can replace files
                System.Windows.Application.Current.Shutdown();
            }
            catch (OperationCanceledException)
            {
                // User cancelled the download — silently handle
                StatusText.Text = "Download cancelled.";
                ResetButtons();
            }
            catch (Exception ex)
            {
                // Network error, disk error, etc.
                StatusText.Text = $"Download failed: {ex.Message}";
                StatusText.Foreground = new System.Windows.Media.SolidColorBrush(
                    System.Windows.Media.Color.FromRgb(0xFF, 0x6B, 0x6B)); // Red-ish

                ResetButtons();

                Debug.WriteLine($"[UpdateDialog] Download failed: {ex}");
            }
        }

        /// <summary>
        /// Re-enables the action buttons after a failed or cancelled download,
        /// allowing the user to retry or dismiss.
        /// </summary>
        private void ResetButtons()
        {
            UpdateNowButton.IsEnabled = true;
            LaterButton.IsEnabled = true;
            DownloadProgressBar.Value = 0;
        }
    }
}
