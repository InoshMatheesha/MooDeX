// playtime.rs - Human formatting of derived playtime.

// 755.0 -> "12h 35m"; 0.0 -> "0m".
pub fn format_playtime(minutes: f64) -> String {
    if !minutes.is_finite() || minutes <= 0.0 {
        return "0m".to_string();
    }
    let total = minutes.round() as u64;
    let h = total / 60;
    let m = total % 60;
    match (h, m) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_spec() {
        assert_eq!(format_playtime(755.0), "12h 35m");
        assert_eq!(format_playtime(0.0), "0m");
        assert_eq!(format_playtime(-3.0), "0m");
        assert_eq!(format_playtime(f64::NAN), "0m");
        assert_eq!(format_playtime(45.4), "45m");
        assert_eq!(format_playtime(120.0), "2h");
        assert_eq!(format_playtime(59.6), "1h");
    }
}
