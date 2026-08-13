using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Threading;
using MooDeX_New_Version_1._0.Services;

namespace MooDeX_New_Version_1._0
{
    public partial class App : System.Windows.Application
    {
        private static void LogCrash(string source, Exception? ex)
        {
            try
            {
                var appFolder = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "MooDeX");
                Directory.CreateDirectory(appFolder);
                var logPath = Path.Combine(appFolder, "crash_log.txt");
                File.AppendAllText(logPath, $"[{DateTime.Now:O}] {source}\n{ex}\n\n");
            }
            catch { }
        }
        private static Mutex? _mutex;
        private ProcessMonitor? _processMonitor;

        public static readonly int RestoreWindowMessage = RegisterWindowMessage("WM_MOODEX_RESTORE_WINDOW_MSG");

        [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
        private static extern int RegisterWindowMessage(string lpString);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern bool PostMessage(IntPtr hWnd, int Msg, IntPtr wParam, IntPtr lParam);

        private const int HWND_BROADCAST = 0xffff;

        protected override void OnStartup(StartupEventArgs e)
        {
            AppDomain.CurrentDomain.UnhandledException += (s, args) =>
                LogCrash("AppDomain.UnhandledException", args.ExceptionObject as Exception);
            DispatcherUnhandledException += (s, args) =>
                LogCrash("DispatcherUnhandledException", args.Exception);
            TaskScheduler.UnobservedTaskException += (s, args) =>
                LogCrash("TaskScheduler.UnobservedTaskException", args.Exception);

            const string mutexName = "MooDeX_SingleInstance_Mutex_9981A";
            _mutex = new Mutex(true, mutexName, out bool createdNew);

            if (!createdNew)
            {
                // Another instance of MooDeX is already running in the background or tray!
                // Broadcast Win32 message so the running instance restores its window
                PostMessage((IntPtr)HWND_BROADCAST, RestoreWindowMessage, IntPtr.Zero, IntPtr.Zero);

                // Instantly shut down this second process before creating any UI or System Tray icon
                Shutdown();
                return;
            }

            base.OnStartup(e);

            var storageService = new StorageService();
            _processMonitor = new ProcessMonitor(storageService);
            _processMonitor.StartMonitoring();
        }

        protected override void OnExit(ExitEventArgs e)
        {
            _processMonitor?.StopMonitoring();
            if (_mutex != null)
            {
                try
                {
                    _mutex.ReleaseMutex();
                    _mutex.Dispose();
                }
                catch { }
            }
            base.OnExit(e);
        }
    }
}
