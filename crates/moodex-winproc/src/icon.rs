// icon.rs - A program's own icon as RGBA pixels, for the tray's game list.

use camino::Utf8Path;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    HBITMAP, HGDIOBJ,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, PrivateExtractIconsW, HICON, ICONINFO};

// Read one size x size frame of a bitmap as top-down BGRA.
fn read_bits(bitmap: HBITMAP, size: u32) -> Option<Vec<u8>> {
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: u32::try_from(std::mem::size_of::<BITMAPINFOHEADER>()).ok()?,
            biWidth: i32::try_from(size).ok()?,
            biHeight: -i32::try_from(size).ok()?,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut buf = vec![0u8; (size * size * 4) as usize];
    // SAFETY: a memory DC is created and deleted here; `buf` holds exactly size*size 32-bit pixels,
    // matching the top-down 32bpp header, so GetDIBits writes within bounds.
    let lines = unsafe {
        let dc = CreateCompatibleDC(None);
        let n = GetDIBits(dc, bitmap, 0, size, Some(buf.as_mut_ptr().cast()), &mut info, DIB_RGB_COLORS);
        let _ = DeleteDC(dc);
        n
    };
    (lines > 0).then_some(buf)
}

// The icon embedded in an exe (or dll) at `size` px, as straight RGBA; None when it has none.
pub fn exe_icon_rgba(path: &Utf8Path, size: u32) -> Option<Vec<u8>> {
    let mut name = [0u16; 260];
    let wide: Vec<u16> = path.as_str().encode_utf16().collect();
    if wide.len() >= name.len() {
        return None;
    }
    name[..wide.len()].copy_from_slice(&wide);
    let px = i32::try_from(size).ok()?;
    let mut icons = [HICON::default()];
    // SAFETY: `name` is a NUL-terminated MAX_PATH buffer and `icons` has room for the one icon requested.
    let got = unsafe { PrivateExtractIconsW(&name, 0, px, px, Some(&mut icons), None, 0) };
    let hicon = icons[0];
    if got == 0 || got == u32::MAX || hicon.is_invalid() {
        return None;
    }
    let mut info = ICONINFO::default();
    // SAFETY: `hicon` is the valid icon extracted above; GetIconInfo fills `info` with two bitmaps we own.
    let ok = unsafe { GetIconInfo(hicon, &mut info) }.is_ok();
    let color = if ok { read_bits(info.hbmColor, size) } else { None };
    let mask = if ok { read_bits(info.hbmMask, size) } else { None };
    // SAFETY: the bitmaps came from GetIconInfo and the icon from PrivateExtractIconsW; each is freed once.
    unsafe {
        if ok {
            let _ = DeleteObject(HGDIOBJ(info.hbmColor.0));
            let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
        }
        let _ = DestroyIcon(hicon);
    }
    let mut px = color?;
    // Old icons carry no alpha channel; their AND mask says which pixels are transparent.
    let has_alpha = px.chunks_exact(4).any(|p| p[3] != 0);
    for (i, p) in px.chunks_exact_mut(4).enumerate() {
        p.swap(0, 2); // BGRA -> RGBA
        if !has_alpha {
            let opaque = mask.as_ref().is_none_or(|m| m.get(i * 4).copied().unwrap_or(0) == 0);
            p[3] = if opaque { 255 } else { 0 };
        }
    }
    px.chunks_exact(4).any(|p| p[3] != 0).then_some(px)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_exe_has_an_icon() {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let notepad = camino::Utf8PathBuf::from(root).join("notepad.exe");
        let px = exe_icon_rgba(&notepad, 16).expect("notepad has an icon");
        assert_eq!(px.len(), 16 * 16 * 4);
        assert!(px.chunks_exact(4).any(|p| p[3] > 0));
    }

    #[test]
    fn missing_file_has_none() {
        assert!(exe_icon_rgba(Utf8Path::new(r"C:\definitely\not\here.exe"), 16).is_none());
    }
}
