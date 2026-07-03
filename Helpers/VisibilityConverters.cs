using System;
using System.Globalization;
using System.Windows;
using System.Windows.Data;

namespace MooDeX_New_Version_1._0.Helpers
{
    /// <summary>
    /// Converts a boolean value to Visibility.
    /// true  → Visible
    /// false → Collapsed
    /// This is a duplicate-safe version of the built-in BooleanToVisibilityConverter
    /// that can be shared across views.
    /// </summary>
    public class InverseBooleanToVisibilityConverter : IValueConverter
    {
        /// <summary>
        /// Converts a boolean to Visibility with INVERSE logic:
        /// true  → Collapsed  (hide when true)
        /// false → Visible    (show when false)
        /// 
        /// Useful for showing an element only when a condition is NOT met,
        /// e.g., showing a static icon when IsCheckingForUpdates is false.
        /// </summary>
        public object Convert(object value, Type targetType, object parameter, CultureInfo culture)
        {
            if (value is bool boolValue)
            {
                return boolValue ? Visibility.Collapsed : Visibility.Visible;
            }
            return Visibility.Visible;
        }

        public object ConvertBack(object value, Type targetType, object parameter, CultureInfo culture)
        {
            if (value is Visibility visibility)
            {
                return visibility != Visibility.Visible;
            }
            return false;
        }
    }

    /// <summary>
    /// Converts a string to Visibility based on whether it is null or empty.
    /// Non-empty string → Visible
    /// Null or empty     → Collapsed
    /// 
    /// Used to hide status text labels when there's nothing to display.
    /// </summary>
    public class NullOrEmptyToVisibilityConverter : IValueConverter
    {
        public object Convert(object value, Type targetType, object parameter, CultureInfo culture)
        {
            if (value is string str && !string.IsNullOrWhiteSpace(str))
            {
                return Visibility.Visible;
            }
            return Visibility.Collapsed;
        }

        public object ConvertBack(object value, Type targetType, object parameter, CultureInfo culture)
        {
            throw new NotSupportedException("NullOrEmptyToVisibilityConverter is one-way only.");
        }
    }
}
