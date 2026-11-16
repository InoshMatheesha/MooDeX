// tray_icons.rs - 16 px tray-menu icons: the game's own exe icon, else a rounded cover tile, else a monogram.

use camino::Utf8PathBuf;
use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

// Windows menus draw item bitmaps at 16 x 16 pixels.
pub const SIZE: u32 = 16;
const RADIUS: f32 = 4.0;

// What a game offers to draw from, in order of preference.
pub struct Source {
    pub key: String,
    pub title: String,
    pub exe: Option<Utf8PathBuf>,
    pub cover: Option<Vec<u8>>,
}

// Base icons by source key; the running dot is added per build.
static CACHE: LazyLock<Mutex<HashMap<String, Vec<u8>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

// True when this source's icon was already rendered (its cover need not be loaded again).
pub fn is_cached(key: &str) -> bool {
    CACHE.lock().is_ok_and(|c| c.contains_key(key))
}

pub fn render(src: &Source, running: bool) -> Vec<u8> {
    let cached = CACHE.lock().ok().and_then(|c| c.get(&src.key).cloned());
    let base = match cached {
        Some(b) => b,
        None => {
            let b = src
                .exe
                .as_deref()
                .and_then(|p| moodex_winproc::exe_icon_rgba(p, SIZE))
                .or_else(|| src.cover.as_deref().and_then(cover_tile))
                .unwrap_or_else(|| monogram(&src.title));
            if let Ok(mut c) = CACHE.lock() {
                c.insert(src.key.clone(), b.clone());
            }
            b
        }
    };
    if running {
        with_live_dot(base)
    } else {
        base
    }
}

// The app icon shrunk for the "Show MooDeX" item.
pub fn app_icon(rgba: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let img = RgbaImage::from_raw(width, height, rgba.to_vec())?;
    Some(imageops::resize(&img, SIZE, SIZE, FilterType::Lanczos3).into_raw())
}

// Anti-aliased coverage of a rounded square at pixel (x, y).
fn rounded(x: u32, y: u32) -> f32 {
    let s = SIZE as f32;
    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
    let cx = px.clamp(RADIUS, s - RADIUS);
    let cy = py.clamp(RADIUS, s - RADIUS);
    let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
    (RADIUS + 0.5 - d).clamp(0.0, 1.0)
}

fn mask_rounded(img: &mut RgbaImage) {
    for (x, y, p) in img.enumerate_pixels_mut() {
        p[3] = (f32::from(p[3]) * rounded(x, y)).round() as u8;
    }
}

// Centre square of the cover art, shrunk and given rounded corners plus a hairline edge.
fn cover_tile(bytes: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let side = w.min(h);
    if side == 0 {
        return None;
    }
    let square = imageops::crop_imm(&img, (w - side) / 2, (h - side) / 2, side, side).to_image();
    let mut tile = imageops::resize(&square, SIZE, SIZE, FilterType::Lanczos3);
    // A soft light edge keeps dark covers from vanishing into a dark menu.
    for (x, y, p) in tile.enumerate_pixels_mut() {
        let edge = x == 0 || y == 0 || x == SIZE - 1 || y == SIZE - 1;
        if edge {
            for c in 0..3 {
                p[c] = ((u16::from(p[c]) * 3 + 255) / 4) as u8;
            }
        }
        p[3] = 255;
    }
    mask_rounded(&mut tile);
    Some(tile.into_raw())
}

// 3 x 5 pixel glyphs (bit 2 = left column), drawn at 2x for a crisp letter on the tile.
fn glyph(c: char) -> [u8; 5] {
    match c.to_ascii_uppercase() {
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b010],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b110, 0b101, 0b101, 0b101, 0b101],
        'O' => [0b111, 0b101, 0b101, 0b101, 0b111],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'Q' => [0b010, 0b101, 0b101, 0b110, 0b011],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '0' => [0b010, 0b101, 0b101, 0b101, 0b010],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b110, 0b001, 0b010, 0b001, 0b110],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b110, 0b001, 0b110],
        '6' => [0b011, 0b100, 0b110, 0b101, 0b010],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b010, 0b101, 0b010, 0b101, 0b010],
        '9' => [0b010, 0b101, 0b011, 0b001, 0b110],
        _ => [0b000, 0b010, 0b111, 0b010, 0b000],
    }
}

// Brand-family gradients; each title always gets the same one.
const GRADIENTS: [([u8; 3], [u8; 3]); 6] = [
    ([0x2e, 0x9b, 0xff], [0x8b, 0x5c, 0xf6]),
    ([0x8b, 0x5c, 0xf6], [0xec, 0x48, 0x99]),
    ([0x06, 0xb6, 0xd4], [0x3b, 0x82, 0xf6]),
    ([0x10, 0xb9, 0x81], [0x06, 0xb6, 0xd4]),
    ([0xf5, 0x9e, 0x0b], [0xef, 0x44, 0x44]),
    ([0x63, 0x66, 0xf1], [0xa8, 0x55, 0xf7]),
];

// Gradient tile with the title's first letter, for games with no icon or cached cover.
fn monogram(title: &str) -> Vec<u8> {
    let hash = title.bytes().fold(5381u32, |h, b| h.wrapping_mul(33) ^ u32::from(b));
    let (a, b) = GRADIENTS[(hash as usize) % GRADIENTS.len()];
    let mut img = RgbaImage::new(SIZE, SIZE);
    let span = (2 * (SIZE - 1)) as f32;
    for (x, y, p) in img.enumerate_pixels_mut() {
        let t = (x + y) as f32 / span;
        let mut rgb = [0u8; 3];
        for c in 0..3 {
            rgb[c] = (f32::from(a[c]) * (1.0 - t) + f32::from(b[c]) * t).round() as u8;
        }
        // A faint sheen across the top half.
        if y < SIZE / 2 {
            for v in &mut rgb {
                *v = v.saturating_add(14);
            }
        }
        *p = Rgba([rgb[0], rgb[1], rgb[2], 255]);
    }
    let letter = title.chars().find(|c| c.is_ascii_alphanumeric()).unwrap_or('?');
    let rows = glyph(letter);
    let (ox, oy) = (5, 3);
    for (r, bits) in rows.iter().enumerate() {
        for col in 0..3u32 {
            if bits & (0b100 >> col) != 0 {
                for dy in 0..2 {
                    for dx in 0..2 {
                        img.put_pixel(ox + col * 2 + dx, oy + r as u32 * 2 + dy, Rgba([255, 255, 255, 255]));
                    }
                }
            }
        }
    }
    mask_rounded(&mut img);
    img.into_raw()
}

// A green "playing" dot with a dark ring in the bottom-right corner.
fn with_live_dot(mut rgba: Vec<u8>) -> Vec<u8> {
    let (cx, cy, r, ring) = (12.0f32, 12.0f32, 2.6f32, 1.2f32);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let outer = (r + ring + 0.5 - d).clamp(0.0, 1.0);
            if outer <= 0.0 {
                continue;
            }
            let inner = (r + 0.5 - d).clamp(0.0, 1.0);
            let color = [
                (52.0 * inner + 11.0 * (1.0 - inner)),
                (211.0 * inner + 11.0 * (1.0 - inner)),
                (153.0 * inner + 15.0 * (1.0 - inner)),
            ];
            let i = ((y * SIZE + x) * 4) as usize;
            let Some(p) = rgba.get_mut(i..i + 4) else { continue };
            let under = f32::from(p[3]) / 255.0;
            let out_a = outer + under * (1.0 - outer);
            for c in 0..3 {
                let blended = color[c] * outer + f32::from(p[c]) * under * (1.0 - outer);
                p[c] = if out_a > 0.0 { (blended / out_a).round() as u8 } else { 0 };
            }
            p[3] = (out_a * 255.0).round() as u8;
        }
    }
    rgba
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn every_source_gives_a_full_icon() {
        let mono = monogram("Hades");
        assert_eq!(mono.len(), (SIZE * SIZE * 4) as usize);
        // Corners are rounded off, the middle is solid.
        assert_eq!(mono[3], 0);
        assert_eq!(mono[((8 * SIZE + 8) * 4 + 3) as usize], 255);
        let live = with_live_dot(mono.clone());
        assert_ne!(live, mono);
        assert_eq!(monogram("Hades"), monogram("Hades"));
    }

    #[test]
    fn cover_is_cropped_square() {
        let mut png = Vec::new();
        RgbaImage::from_pixel(64, 36, Rgba([200, 30, 30, 255]))
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let tile = cover_tile(&png).unwrap();
        assert_eq!(tile.len(), (SIZE * SIZE * 4) as usize);
        assert_eq!(tile[((8 * SIZE + 8) * 4) as usize], 200);
    }

    // Writes a preview strip when MOODEX_ICON_PREVIEW names an output PNG (manual check).
    #[test]
    fn preview_strip() {
        let Ok(out) = std::env::var("MOODEX_ICON_PREVIEW") else { return };
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let mut srcs: Vec<(Vec<u8>, bool)> = Vec::new();
        for exe in ["notepad.exe", "explorer.exe", "regedit.exe"] {
            let p = Utf8PathBuf::from(&root).join(exe);
            srcs.push((moodex_winproc::exe_icon_rgba(&p, SIZE).unwrap(), false));
        }
        for t in ["Hades", "Onimusha", "Crimson Desert", "Avatar", "Elden Ring", "2077"] {
            srcs.push((monogram(t), false));
        }
        srcs.push((monogram("Playing"), true));
        let scale = 6;
        let mut strip =
            RgbaImage::from_pixel(srcs.len() as u32 * (SIZE + 4) * scale, (SIZE + 8) * scale, Rgba([43, 43, 43, 255]));
        for (i, (px, live)) in srcs.into_iter().enumerate() {
            let px = if live { with_live_dot(px) } else { px };
            let icon = RgbaImage::from_raw(SIZE, SIZE, px).unwrap();
            let big = imageops::resize(&icon, SIZE * scale, SIZE * scale, FilterType::Nearest);
            imageops::overlay(
                &mut strip,
                &big,
                i64::from(i as u32 * (SIZE + 4) * scale + 2 * scale),
                i64::from(4 * scale),
            );
        }
        strip.save(out).unwrap();
    }
}
