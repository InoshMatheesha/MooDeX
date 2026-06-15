using System.IO;
using Microsoft.Win32;
using MooDeX_New_Version_1._0.Models;
using MooDeX_New_Version_1._0.Services;

namespace MooDeX_New_Version_1._0.ViewModels
{
    public class SettingsViewModel : ViewModelBase
    {
        private readonly StorageService _storageService;
        private bool _minimizeToTray;
        private bool _launchAtStartup;
        private bool _launchMinimized;

        public SettingsViewModel()
        {
            _storageService = new StorageService();
            LoadSettings();
        }

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
