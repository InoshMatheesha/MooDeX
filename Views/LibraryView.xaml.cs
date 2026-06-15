using System.Windows.Controls;

namespace MooDeX_New_Version_1._0.Views
{
    public partial class LibraryView : System.Windows.Controls.UserControl
    {
        public LibraryView()
        {
            InitializeComponent();
        }

        private void UserControl_Unloaded(object sender, System.Windows.RoutedEventArgs e)
        {
            if (this.DataContext is ViewModels.LibraryViewModel vm)
            {
                vm.Cleanup();
            }
        }
    }
}