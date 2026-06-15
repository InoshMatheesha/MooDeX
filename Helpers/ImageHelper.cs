using System;
using System.IO;
using System.Net.Http;
using System.Security.Cryptography;
using System.Text;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;

namespace MooDeX_New_Version_1._0.Helpers
{
    public static class ImageHelper
    {
        public static readonly DependencyProperty SourceUrlProperty =
            DependencyProperty.RegisterAttached(
                "SourceUrl",
                typeof(string),
                typeof(ImageHelper),
                new PropertyMetadata(string.Empty, OnSourceUrlChanged));

        public static string GetSourceUrl(DependencyObject obj) => (string)obj.GetValue(SourceUrlProperty);
        public static void SetSourceUrl(DependencyObject obj, string value) => obj.SetValue(SourceUrlProperty, value);

        private static readonly HttpClient _httpClient = new HttpClient();

        private static async void OnSourceUrlChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
        {
            if (d is System.Windows.Controls.Image image)
            {
                string url = e.NewValue as string ?? string.Empty;
                if (string.IsNullOrWhiteSpace(url))
                {
                    image.Source = null;
                    return;
                }

                // Check local file status asynchronously to prevent blocking the UI thread
                bool isLocal = url.StartsWith("pack://") || await Task.Run(() => File.Exists(url));
                if (isLocal)
                {
                    try
                    {
                        var bitmap = await Task.Run(() =>
                        {
                            var bmp = new BitmapImage();
                            bmp.BeginInit();
                            bmp.UriSource = new Uri(url);
                            bmp.CacheOption = BitmapCacheOption.OnLoad;
                            bmp.DecodePixelWidth = 300;
                            bmp.EndInit();
                            bmp.Freeze();
                            return bmp;
                        });

                        if (GetSourceUrl(image) == url)
                        {
                            image.Source = bitmap;
                        }
                    }
                    catch
                    {
                        if (GetSourceUrl(image) == url)
                        {
                            image.Source = null;
                        }
                    }
                    return;
                }

                if (!url.StartsWith("http://", StringComparison.OrdinalIgnoreCase) &&
                    !url.StartsWith("https://", StringComparison.OrdinalIgnoreCase))
                {
                    image.Source = null;
                    return;
                }

                // Calculate local cache path
                string cacheFolder = Path.Combine(
                    Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                    "MooDeX",
                    "cache"
                );

                string hashedName = GetMd5Hash(url) + GetExtension(url);
                string localPath = Path.Combine(cacheFolder, hashedName);

                bool cacheExists = await Task.Run(() => File.Exists(localPath));
                if (cacheExists)
                {
                    try
                    {
                        var bitmap = await Task.Run(() =>
                        {
                            var bmp = new BitmapImage();
                            bmp.BeginInit();
                            bmp.UriSource = new Uri(localPath);
                            bmp.CacheOption = BitmapCacheOption.OnLoad;
                            bmp.DecodePixelWidth = 300;
                            bmp.EndInit();
                            bmp.Freeze();
                            return bmp;
                        });

                        if (GetSourceUrl(image) == url)
                        {
                            image.Source = bitmap;
                            return;
                        }
                    }
                    catch
                    {
                        // Cache file corrupted, delete it and try downloading
                        try { await Task.Run(() => File.Delete(localPath)); } catch {}
                    }
                }

                // Set image to null/placeholder while downloading
                image.Source = null;

                try
                {
                    // Download in background
                    byte[] data = await Task.Run(() => _httpClient.GetByteArrayAsync(url));
                    
                    // Scale and save to local file path (Max 500x250 low quality JPEG)
                    await Task.Run(() =>
                    {
                        try
                        {
                            Directory.CreateDirectory(cacheFolder);
                            
                            using (var inStream = new MemoryStream(data))
                            {
                                var decoder = BitmapDecoder.Create(
                                    inStream, 
                                    BitmapCreateOptions.None, 
                                    BitmapCacheOption.OnLoad
                                );
                                if (decoder.Frames.Count > 0)
                                {
                                    var frame = decoder.Frames[0];
                                    
                                    double targetWidth = 500;
                                    double targetHeight = 250;
                                    
                                    // Scale preserving aspect ratio
                                    double scaleX = targetWidth / frame.PixelWidth;
                                    double scaleY = targetHeight / frame.PixelHeight;
                                    double scale = Math.Min(scaleX, scaleY);
                                    
                                    if (scale >= 1.0)
                                    {
                                        File.WriteAllBytes(localPath, data);
                                    }
                                    else
                                    {
                                        var scaleTransform = new ScaleTransform(scale, scale);
                                        var scaledBitmap = new TransformedBitmap(frame, scaleTransform);
                                        
                                        using (var outStream = new FileStream(localPath, FileMode.Create, FileAccess.Write, FileShare.None))
                                        {
                                            var encoder = new JpegBitmapEncoder();
                                            encoder.QualityLevel = 75; // Low-to-medium JPEG quality
                                            encoder.Frames.Add(BitmapFrame.Create(scaledBitmap));
                                            encoder.Save(outStream);
                                        }
                                    }
                                }
                                else
                                {
                                    File.WriteAllBytes(localPath, data);
                                }
                            }
                        }
                        catch
                        {
                            // Fallback to saving original data on any error
                            try
                            {
                                File.WriteAllBytes(localPath, data);
                            }
                            catch {}
                        }
                    });

                    // Set source on UI thread if the URL hasn't changed in the meantime
                    if (GetSourceUrl(image) == url)
                    {
                        var bitmap = await Task.Run(() =>
                        {
                            var bmp = new BitmapImage();
                            bmp.BeginInit();
                            bmp.UriSource = new Uri(localPath);
                            bmp.CacheOption = BitmapCacheOption.OnLoad;
                            bmp.DecodePixelWidth = 300;
                            bmp.EndInit();
                            bmp.Freeze();
                            return bmp;
                        });

                        if (GetSourceUrl(image) == url)
                        {
                            image.Source = bitmap;
                        }
                    }
                }
                catch
                {
                    // Fallback to null on failure
                    if (GetSourceUrl(image) == url)
                    {
                        image.Source = null;
                    }
                }
            }
        }

        private static string GetMd5Hash(string input)
        {
            using (var md5 = MD5.Create())
            {
                byte[] hashBytes = md5.ComputeHash(Encoding.UTF8.GetBytes(input));
                var sb = new StringBuilder();
                foreach (byte b in hashBytes)
                {
                    sb.Append(b.ToString("x2"));
                }
                return sb.ToString();
            }
        }

        private static string GetExtension(string url)
        {
            try
            {
                var uri = new Uri(url);
                string path = uri.AbsolutePath;
                string ext = Path.GetExtension(path);
                if (ext.Equals(".jpg", StringComparison.OrdinalIgnoreCase) ||
                    ext.Equals(".jpeg", StringComparison.OrdinalIgnoreCase) ||
                    ext.Equals(".png", StringComparison.OrdinalIgnoreCase) ||
                    ext.Equals(".webp", StringComparison.OrdinalIgnoreCase))
                {
                    return ext;
                }
            }
            catch {}
            return ".jpg"; // Default extension
        }
    }
}
