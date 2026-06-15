using System.Windows.Controls;
using System.Windows.Input;
using MooDeX_New_Version_1._0.Models;

namespace MooDeX_New_Version_1._0.Views
{
    public partial class MyGamesView : System.Windows.Controls.UserControl
    {
        public MyGamesView()
        {
            InitializeComponent();
        }

        private void OnListBoxItemDoubleClick(object sender, MouseButtonEventArgs e)
        {
            if (sender is ListBoxItem item && item.DataContext is Game selectedGame)
            {
                if (this.DataContext is ViewModels.MyGamesViewModel vm)
                {
                    vm.SelectedMatch = selectedGame;
                    if (vm.SaveGameCommand.CanExecute(null))
                    {
                        vm.SaveGameCommand.Execute(null);
                    }
                }
            }
        }

        private void UserControl_Unloaded(object sender, System.Windows.RoutedEventArgs e)
        {
            if (this.DataContext is ViewModels.MyGamesViewModel vm)
            {
                vm.Cleanup();
            }
        }
    }
}
