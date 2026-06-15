using System;
using System.Drawing; // For Icon
using System.IO;
using System.Windows;
using System.Windows.Forms;
using MooDeX_New_Version_1._0.Services;
using MooDeX_New_Version_1._0.ViewModels;

namespace MooDeX_New_Version_1._0.Views
{
    public partial class MainWindow : Window
    {
        private NotifyIcon? _notifyIcon;
        private readonly StorageService _storageService;

        public MainWindow()
        {
            InitializeComponent();
            _storageService = new StorageService();
            DataContext = new ViewModels.MainViewModel();

            // Set up System Tray
            SetupSystemTray();
        }

        protected override void OnSourceInitialized(EventArgs e)
        {
            base.OnSourceInitialized(e);

            // Restore last adjusted window size, default to 1340x820
            try
            {
                var settings = _storageService.LoadSettings();
                double width = settings.WindowWidth > 0 ? settings.WindowWidth : 1340;
                double height = settings.WindowHeight > 80 ? settings.WindowHeight : 820;
                Width = width;
                Height = height;
            }
            catch
            {
                Width = 1340;
                Height = 820;
            }

            // Check if launched minimized via startup argument
            var args = Environment.GetCommandLineArgs();
            if (Array.Exists(args, arg => arg.Equals("--minimized", StringComparison.OrdinalIgnoreCase)))
            {
                var settings = _storageService.LoadSettings();
                if (settings.MinimizeToTray)
                {
                    WindowState = WindowState.Minimized;
                    Hide();
                }
                else
                {
                    WindowState = WindowState.Minimized;
                }
            }
        }

        private void SetupSystemTray()
        {
            _notifyIcon = new NotifyIcon();
            _notifyIcon.Text = "MooDeX";

            // Load application icon for tray
            try
            {
                var iconStream = System.Windows.Application.GetResourceStream(new Uri("pack://application:,,,/Icon Logo/icon.ico"))?.Stream;
                if (iconStream != null)
                {
                    _notifyIcon.Icon = new Icon(iconStream);
                }
            }
            catch
            {
                // Fallback to system standard icon
                _notifyIcon.Icon = SystemIcons.Application;
            }

            _notifyIcon.Visible = true;
            _notifyIcon.DoubleClick += (s, e) => RestoreWindow();

            // Tray Context Menu
            var contextMenu = new ContextMenuStrip();
            contextMenu.Items.Add("Open MooDeX", null, (s, e) => RestoreWindow());
            contextMenu.Items.Add("Exit", null, (s, e) => ForceClose());
            _notifyIcon.ContextMenuStrip = contextMenu;

            // Handle minimizing
            StateChanged += MainWindow_StateChanged;
            Closing += MainWindow_Closing;
        }

        private void SaveWindowSize()
        {
            try
            {
                var settings = _storageService.LoadSettings();
                if (WindowState == WindowState.Normal)
                {
                    settings.WindowWidth = Width;
                    settings.WindowHeight = Height;
                }
                else if (WindowState == WindowState.Maximized || WindowState == WindowState.Minimized)
                {
                    var rect = RestoreBounds;
                    if (rect.Width > 0 && rect.Height > 0)
                    {
                        settings.WindowWidth = rect.Width;
                        settings.WindowHeight = rect.Height;
                    }
                }
                _storageService.SaveSettings(settings);
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"Failed to save window size: {ex.Message}");
            }
        }

        private void MainWindow_StateChanged(object? sender, EventArgs e)
        {
            var settings = _storageService.LoadSettings();
            if (settings.MinimizeToTray && WindowState == WindowState.Minimized)
            {
                SaveWindowSize();
                Hide();
                if (_notifyIcon != null)
                {
                    _notifyIcon.ShowBalloonTip(2000, "MooDeX", "MooDeX has been minimized to the system tray.", ToolTipIcon.Info);
                }
            }
        }

        private void MainWindow_Closing(object? sender, System.ComponentModel.CancelEventArgs e)
        {
            var settings = _storageService.LoadSettings();
            if (settings.MinimizeToTray)
            {
                SaveWindowSize();
                e.Cancel = true;
                Hide();
                if (_notifyIcon != null)
                {
                    _notifyIcon.ShowBalloonTip(2000, "MooDeX", "MooDeX is running in the background.", ToolTipIcon.Info);
                }
            }
            else
            {
                SaveWindowSize();
                CleanupTray();
            }
        }

        private void RestoreWindow()
        {
            Show();
            WindowState = WindowState.Normal;
            Activate();
        }

        private void ForceClose()
        {
            SaveWindowSize();
            CleanupTray();
            System.Windows.Application.Current.Shutdown();
        }

        private void CleanupTray()
        {
            if (_notifyIcon != null)
            {
                _notifyIcon.Visible = false;
                _notifyIcon.Dispose();
                _notifyIcon = null;
            }
        }
    }
}
