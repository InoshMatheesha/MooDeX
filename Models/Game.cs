using System;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Collections.Generic;
using System.Text.Json.Serialization;
using System.IO;

namespace MooDeX_New_Version_1._0.Models
{
    public class Game : INotifyPropertyChanged
    {
        private int _id;
        private string _title = string.Empty;
        private string _coverUrl = string.Empty;
        private double _playtime;
        private string _status = "Not Played";
        private string _executableName = string.Empty;
        private string _executablePath = string.Empty;
        private double _rating;
        private string _source = "Library";
        private List<string> _genres = new List<string>();
        private string _notes = string.Empty;

        [JsonPropertyName("id")]
        public int Id 
        { 
            get => _id; 
            set => SetProperty(ref _id, value); 
        }

        [JsonPropertyName("name")]
        public string Title 
        { 
            get => _title; 
            set => SetProperty(ref _title, value); 
        }

        [JsonPropertyName("background_image")]
        public string CoverUrl 
        { 
            get => _coverUrl; 
            set => SetProperty(ref _coverUrl, value); 
        }

        [JsonPropertyName("playtime")]
        public double Playtime 
        { 
            get => _playtime; 
            set
            {
                if (SetProperty(ref _playtime, value))
                {
                    OnPropertyChanged(nameof(PlaytimeFormatted));
                }
            }
        }

        [JsonIgnore]
        public string PlaytimeFormatted
        {
            get
            {
                if (_playtime <= 0) return "0m";
                int totalMinutes = (int)Math.Round(_playtime);
                int hours = totalMinutes / 60;
                int minutes = totalMinutes % 60;
                if (hours > 0 && minutes > 0) return $"{hours}h {minutes}m";
                if (hours > 0) return $"{hours}h";
                return $"{minutes}m";
            }
        }


        [JsonPropertyName("status")]
        public string Status 
        { 
            get => _status; 
            set => SetProperty(ref _status, value); 
        }

        [JsonPropertyName("executable_name")]
        public string ExecutableName 
        { 
            get => _executableName; 
            set => SetProperty(ref _executableName, value); 
        }

        [JsonPropertyName("exe_path")]
        public string ExecutablePath
        {
            get => _executablePath;
            set => SetProperty(ref _executablePath, value);
        }

        private int _playCount;

        [JsonPropertyName("play_count")]
        public int PlayCount
        {
            get => _playCount;
            set => SetProperty(ref _playCount, value);
        }

        private string _addedDate = string.Empty;
        private string _releaseDate = string.Empty;
        private DateTime? _lastPlayed;

        [JsonPropertyName("rating")]
        public double Rating
        {
            get => _rating;
            set
            {
                if (SetProperty(ref _rating, value))
                {
                    OnPropertyChanged(nameof(UserRating));
                    OnPropertyChanged(nameof(RatingDescription));
                }
            }
        }

        [JsonIgnore]
        public string RatingDescription
        {
            get
            {
                if (Rating <= 0) return "Unrated ✖";
                if (Rating < 3.0) return "Awful 😠";
                if (Rating < 5.0) return "Poor 😞";
                if (Rating < 6.5) return "Mediocre 😐";
                if (Rating < 7.5) return "Decent 🙂";
                if (Rating < 8.5) return "Good 😊";
                if (Rating < 9.5) return "Great 🤩";
                return "Masterpiece! 👑";
            }
        }

        [JsonIgnore]
        public int UserRating 
        { 
            get => (int)Math.Round(Rating); 
            set => Rating = value; 
        }

        [JsonPropertyName("added_date")]
        public string AddedDate
        {
            get => string.IsNullOrEmpty(_addedDate) ? "N/A" : _addedDate;
            set => SetProperty(ref _addedDate, value);
        }

        [JsonPropertyName("released")]
        public string ReleaseDate
        {
            get => _releaseDate;
            set => SetProperty(ref _releaseDate, value);
        }

        [JsonPropertyName("last_played")]
        public string LastPlayedJson
        {
            get => LastPlayed == null ? "Never" : LastPlayed.Value.ToString("yyyy-MM-dd");
            set
            {
                if (string.IsNullOrEmpty(value) || value.Equals("Never", StringComparison.OrdinalIgnoreCase))
                {
                    LastPlayed = null;
                }
                else if (DateTime.TryParse(value, out var dt))
                {
                    LastPlayed = dt;
                }
                else
                {
                    LastPlayed = null;
                }
            }
        }

        [JsonIgnore]
        public DateTime? LastPlayed
        {
            get => _lastPlayed;
            set
            {
                if (SetProperty(ref _lastPlayed, value))
                {
                    OnPropertyChanged(nameof(LastPlayedJson));
                }
            }
        }

        /// <summary>
        /// "Library"  — added via Discover / search  (shows in My Library tab)
        /// "PCGames"  — manually imported exe       (shows in PC Games tab)
        /// </summary>
        [JsonIgnore]
        public string Source
        {
            get => _source;
            set => SetProperty(ref _source, value);
        }

        private bool _isAdded;
        [JsonIgnore]
        public bool IsAdded
        {
            get => _isAdded;
            set => SetProperty(ref _isAdded, value);
        }

        [JsonPropertyName("genres")]
        public List<string> Genres
        {
            get => _genres;
            set => _genres = value ?? new List<string>();
        }

        [JsonPropertyName("notes")]
        public string Notes
        {
            get => _notes;
            set => _notes = value ?? string.Empty;
        }

        [JsonPropertyName("is_local")]
        public bool IsLocal { get; set; }

        public event PropertyChangedEventHandler? PropertyChanged;

        protected void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }

        protected bool SetProperty<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
        {
            if (EqualityComparer<T>.Default.Equals(field, value)) return false;
            field = value;
            OnPropertyChanged(propertyName);
            return true;
        }
    }
}
