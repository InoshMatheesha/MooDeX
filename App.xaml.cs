using System.Windows;
using MooDeX_New_Version_1._0.Services;
using MooDeX_New_Version_1._0.ViewModels;

namespace MooDeX_New_Version_1._0
{
    public partial class App : System.Windows.Application
    {
        private ProcessMonitor? _processMonitor;

        protected override void OnStartup(StartupEventArgs e)
        {
            base.OnStartup(e);

            var storageService = new StorageService();
            _processMonitor = new ProcessMonitor(storageService);
            _processMonitor.StartMonitoring();
        }

        protected override void OnExit(ExitEventArgs e)
        {
            _processMonitor?.StopMonitoring();
            base.OnExit(e);
        }
    }
}
