using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Collections.Specialized;
using System.ComponentModel;

namespace MooDeX_New_Version_1._0.Helpers
{
    public class RangeObservableCollection<T> : ObservableCollection<T>
    {
        public void ReplaceRange(IEnumerable<T> range)
        {
            Items.Clear();
            foreach (var item in range)
            {
                Items.Add(item);
            }
            OnNotifyCollectionChanged();
        }

        private void OnNotifyCollectionChanged()
        {
            OnPropertyChanged(new PropertyChangedEventArgs("Count"));
            OnPropertyChanged(new PropertyChangedEventArgs("Item[]"));
            OnCollectionChanged(new NotifyCollectionChangedEventArgs(NotifyCollectionChangedAction.Reset));
        }
    }
}
