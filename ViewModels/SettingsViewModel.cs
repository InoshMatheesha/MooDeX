using System;
using System.Diagnostics;
using System.IO;
using System.Windows;
using System.Windows.Input;
using Microsoft.Win32;
using MooDeX_New_Version_1._0.Helpers;
using MooDeX_New_Version_1._0.Models;
using MooDeX_New_Version_1._0.Services;
using MooDeX_New_Version_1._0.Views;

namespace MooDeX_New_Version_1._0.ViewModels
{
    public class SettingsViewModel : ViewModelBase
    {
        // ─────────────────────────────────────────────────────────────
        //  EXISTING FIELDS
        // ─────────────────────────────────────────────────────────────

        private readonly StorageService _storageService;
        private bool _minimizeToTray;
        private bool _launchAtStartup;
        private bool _launchMinimized;

        // ─────────────────────────────────────────────────────────────
        //  UPDATE SYSTEM FIELDS
        // ─────────────────────────────────────────────────────────────

        private readonly UpdateService _updateService;
        private bool _isCheckingForUpdates;
        private string _updateStatusText = string.Empty;

        // ─────────────────────────────────────────────────────────────
        //  CONSTRUCTOR
        // ─────────────────────────────────────────────────────────────

        public SettingsViewModel()
        {
            _storageService = new StorageService();
            _updateService = new UpdateService();
            LoadSettings();

            // Initialize the "Check for Updates" command
            CheckForUpdatesCommand = new RelayCommand(
                async _ => await CheckForUpdatesAsync(),
                _ => !IsCheckingForUpdates);  // Disable while a check is in progress
        }

        // ═══════════════════════════════════════════════════════════
        //  EXISTING SETTINGS PROPERTIES (unchanged)
        // ═══════════════════════════════════════════════════════════

        public bool MinimizeToTray
        {
            get => _minimizeToTray;
            set
            {
                if (SetProperty(ref _minimizeToTray, value))
                {
                    SaveSettings();
                }
            }
        }

        public bool LaunchAtStartup
        {
            get => _launchAtStartup;
            set
            {
                if (SetProperty(ref _launchAtStartup, value))
                {
                    SaveSettings();
                    SetRegistryStartup(value);
                }
            }
        }

        public bool LaunchMinimized
        {
            get => _launchMinimized;
            set
            {
                if (SetProperty(ref _launchMinimized, value))
                {
                    SaveSettings();
                    if (LaunchAtStartup)
                    {
                        SetRegistryStartup(true);
                    }
                }
            }
        }

        // ═══════════════════════════════════════════════════════════
        //  UPDATE SYSTEM PROPERTIES
        // ═══════════════════════════════════════════════════════════

        /// <summary>
        /// Command bound to the "Check for Updates" button in the Settings view.
        /// </summary>
        public ICommand CheckForUpdatesCommand { get; }

        /// <summary>
        /// True while the update check is in progress.
        /// Used to show a spinner and disable the button.
        /// </summary>
        public bool IsCheckingForUpdates
        {
            get => _isCheckingForUpdates;
            set
            {
                if (SetProperty(ref _isCheckingForUpdates, value))
                {
                    // Force WPF to re-evaluate CanExecute on the command
                    CommandManager.InvalidateRequerySuggested();
                }
            }
        }

        /// <summary>
        /// Status text displayed below the button (e.g., "You are using the latest version.").
        /// </summary>
        public string UpdateStatusText
        {
            get => _updateStatusText;
            set => SetProperty(ref _updateStatusText, value);
        }

        // ═══════════════════════════════════════════════════════════
        //  UPDATE CHECK LOGIC
        // ═══════════════════════════════════════════════════════════

        /// <summary>
        /// Checks for updates asynchronously, shows the update dialog if a newer
        /// version is available, or displays an "up to date" message.
        /// 
        /// Flow:
        /// 1. Set IsCheckingForUpdates = true (shows spinner, disables button).
        /// 2. Call UpdateService.CheckForUpdateAsync() to fetch update.json.
        /// 3. Compare versions with UpdateService.IsNewerVersion().
        /// 4a. If newer → open UpdateDialog with release notes.
        /// 4b. If current → show "already up to date" status text.
        /// 5. On error → show friendly error message.
        /// 6. Set IsCheckingForUpdates = false (hides spinner, re-enables button).
        /// </summary>
        private async System.Threading.Tasks.Task CheckForUpdatesAsync()
        {
            IsCheckingForUpdates = true;
            UpdateStatusText = string.Empty;

            try
            {
                // Step 1: Fetch the remote update manifest
                var updateInfo = await _updateService.CheckForUpdateAsync();

                // Step 2: Compare versions
                if (_updateService.IsNewerVersion(updateInfo.Version))
                {
                    // Step 3a: A newer version is available — show the update dialog
                    var dialog = new UpdateDialog(updateInfo)
                    {
                        Owner = System.Windows.Application.Current.MainWindow
                    };
                    dialog.ShowDialog();

                    // If the user accepted and the app is shutting down,
                    // we don't need to update the status text
                    if (!dialog.UserAcceptedUpdate)
                    {
                        UpdateStatusText = "Update skipped.";
                    }
                }
                else
                {
                    // Step 3b: Already on the latest version
                    UpdateStatusText = "✓ You are using the latest version.";
                }
            }
            catch (UpdateCheckException ex)
            {
                // User-friendly error message from UpdateService
                UpdateStatusText = $"⚠ {ex.Message}";
                Debug.WriteLine($"[SettingsVM] Update check failed: {ex}");
            }
            catch (Exception ex)
            {
                // Catch-all for truly unexpected errors
                UpdateStatusText = "⚠ An unexpected error occurred. Please try again later.";
                Debug.WriteLine($"[SettingsVM] Unexpected update error: {ex}");
            }
            finally
            {
                IsCheckingForUpdates = false;
            }
        }

        // ═══════════════════════════════════════════════════════════
        //  EXISTING SETTINGS METHODS (unchanged)
        // ═══════════════════════════════════════════════════════════

        private void LoadSettings()
        {
            var settings = _storageService.LoadSettings();
            _minimizeToTray = settings.MinimizeToTray;
            _launchAtStartup = settings.LaunchAtStartup;
            _launchMinimized = settings.LaunchMinimized;

            // Sync registry with saved settings
            bool registryEnabled = IsRegistryStartupEnabled();
            if (registryEnabled != _launchAtStartup)
            {
                SetRegistryStartup(_launchAtStartup);
            }
            else if (_launchAtStartup)
            {
                SetRegistryStartup(true);
            }
        }

        private void SaveSettings()
        {
            var settings = _storageService.LoadSettings();
            settings.MinimizeToTray = MinimizeToTray;
            settings.LaunchAtStartup = LaunchAtStartup;
            settings.LaunchMinimized = LaunchMinimized;
            _storageService.SaveSettings(settings);
        }

        private const string RegistryKeyPath = @"Software\Microsoft\Windows\CurrentVersion\Run";
        private const string RegistryValueName = "MooDeX";

        private void SetRegistryStartup(bool enable)
        {
            try
            {
                using (var key = Registry.CurrentUser.OpenSubKey(RegistryKeyPath, true))
                {
                    if (key != null)
                    {
                        if (enable)
                        {
                            string appPath = System.Diagnostics.Process.GetCurrentProcess().MainModule?.FileName 
                                ?? Path.Combine(System.AppDomain.CurrentDomain.BaseDirectory, "MooDeX-New_Version 1.0.exe");
                            string args = LaunchMinimized ? " --minimized" : "";
                            key.SetValue(RegistryValueName, $"\"{appPath}\"{args}");
                        }
                        else
                        {
                            key.DeleteValue(RegistryValueName, false);
                        }
                    }
                }
            }
            catch (System.Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"Failed to set registry startup: {ex.Message}");
            }
        }

        private bool IsRegistryStartupEnabled()
        {
            try
            {
                using (var key = Registry.CurrentUser.OpenSubKey(RegistryKeyPath, false))
                {
                    if (key != null)
                    {
                        return key.GetValue(RegistryValueName) != null;
                    }
                }
            }
            catch {}
            return false;
        }
    }
}
