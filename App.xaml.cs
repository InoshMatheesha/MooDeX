using System.Windows;
using MooDeX_New_Version_1._0.Services;
using MooDeX_New_Version_1._0.ViewModels;

namespace MooDeX_New_Version_1._0
{
    public partial class App : System.Windows.Application
    {
        private ProcessMonitor? _processMonitor;
        private static System.Threading.Mutex? _mutex = null;
        private System.Threading.Thread? _listenThread = null;
        private System.Threading.EventWaitHandle? _eventWaitHandle = null;

        protected override void OnStartup(StartupEventArgs e)
        {
            const string appName = "MooDeX_SingleInstance_Mutex_v1";
            bool createdNew;

            _mutex = new System.Threading.Mutex(true, appName, out createdNew);

            if (!createdNew)
            {
                // App is already running. Signal the event to bring it to front.
                try 
                {
                    var waitHandle = System.Threading.EventWaitHandle.OpenExisting("MooDeX_BringToFront_Event_v1");
                    waitHandle.Set();
                } 
                catch { }

                // Exit this instance
                System.Windows.Application.Current.Shutdown();
                return;
            }

            // This is the first instance. Create the wait handle.
            _eventWaitHandle = new System.Threading.EventWaitHandle(false, System.Threading.EventResetMode.AutoReset, "MooDeX_BringToFront_Event_v1");
            _listenThread = new System.Threading.Thread(() =>
            {
                while (_eventWaitHandle.WaitOne())
                {
                    System.Windows.Application.Current.Dispatcher.Invoke(() =>
                    {
                        var mainWindow = System.Windows.Application.Current.MainWindow;
                        if (mainWindow != null)
                        {
                            mainWindow.Show();
                            if (mainWindow.WindowState == WindowState.Minimized)
                            {
                                mainWindow.WindowState = WindowState.Normal;
                            }
                            mainWindow.Activate();
                            mainWindow.Topmost = true;
                            mainWindow.Topmost = false;
                            mainWindow.Focus();
                        }
                    });
                }
            });
            _listenThread.IsBackground = true;
            _listenThread.Start();

            base.OnStartup(e);

            var storageService = new StorageService();
            _processMonitor = new ProcessMonitor(storageService);
            _processMonitor.StartMonitoring();
        }

        protected override void OnExit(ExitEventArgs e)
        {
            _processMonitor?.StopMonitoring();
            _eventWaitHandle?.Close();
            _mutex?.ReleaseMutex();
            _mutex?.Dispose();
            base.OnExit(e);
        }
    }
}
