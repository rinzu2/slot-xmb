//! The handheld's own background pictures, when the owner has copied them to the card: one small
//! gradient per colour, which the handheld stretches over its screen under the waves. They are
//! never shipped; without them the fitted colours are used.
//!
//! The handheld keeps them in two files of 60 by 34 pictures stored back to back: `13-27.bmp`
//! with the 22 colours before the months, and `01-12.bmp` (or `01-12_03g.bmp` on later models)
//! with the 12 month colours, in the colour list's order.

use crate::CartFace;

/// Where on the card the pictures are looked for, under `System/xmb/`.
pub const XMB_GROUND_OTHERS: &str = "13-27.bmp";
pub const XMB_GROUND_MONTHS: [&str; 2] = ["01-12_03g.bmp", "01-12.bmp"];

/// Every 24-bit picture in `bytes`, one after another.
fn pictures(bytes: &[u8]) -> Vec<CartFace> {
    let u32_at = |i: usize| -> Option<u32> { Some(u32::from_le_bytes(bytes.get(i..i + 4)?.try_into().ok()?)) };
    let mut out = Vec::new();
    let mut at = 0usize;
    while at + 54 <= bytes.len() && &bytes[at..at + 2] == b"BM" {
        let (Some(size), Some(off), Some(w), Some(h)) = (u32_at(at + 2), u32_at(at + 10), u32_at(at + 18), u32_at(at + 22)) else {
            break;
        };
        let (w, h) = (w as i32, h as i32);
        let bpp = u16::from_le_bytes([bytes[at + 28], bytes[at + 29]]);
        if size < 54 || w <= 0 || h == 0 || w > 256 || h.abs() > 256 || bpp != 24 {
            break;
        }
        let (w, hh) = (w as usize, h.unsigned_abs() as usize);
        let stride = (w * 3 + 3) & !3;
        let data = at + off as usize;
        let Some(pixels) = bytes.get(data..data + stride * hh) else {
            break;
        };
        let mut rgba = Vec::with_capacity(w * hh * 4);
        for y in 0..hh {
            let row = if h > 0 { hh - 1 - y } else { y };
            for px in pixels[row * stride..].chunks_exact(3).take(w) {
                rgba.extend_from_slice(&[px[2], px[1], px[0], 255]);
            }
        }
        out.push(CartFace {
            rgba,
            w: w as u32,
            h: hh as u32,
        });
        at += size as usize;
    }
    out
}

/// The 34 colours' pictures, in the colour list's order, from the two files' contents; empty
/// unless both hold the expected number.
pub fn xmb_ground_faces(others: &[u8], months: &[u8]) -> Vec<CartFace> {
    let mut all = pictures(others);
    let months = pictures(months);
    if all.len() != 22 || months.len() != 12 {
        return Vec::new();
    }
    all.extend(months);
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bmp(colour: [u8; 3]) -> Vec<u8> {
        let (w, h) = (60u32, 34u32);
        let stride = (w * 3 + 3) & !3;
        let size = 54 + stride * h;
        let mut b = Vec::new();
        b.extend_from_slice(b"BM");
        b.extend_from_slice(&size.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&54u32.to_le_bytes());
        b.extend_from_slice(&40u32.to_le_bytes());
        b.extend_from_slice(&(w as i32).to_le_bytes());
        b.extend_from_slice(&(h as i32).to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&24u16.to_le_bytes());
        b.extend_from_slice(&[0; 24]);
        for _ in 0..h {
            for _ in 0..w {
                b.extend_from_slice(&[colour[2], colour[1], colour[0]]);
            }
            b.resize(b.len() + (stride - w * 3) as usize, 0);
        }
        b
    }

    #[test]
    fn reads_pictures_stored_back_to_back() {
        let others: Vec<u8> = (0..22u8).flat_map(|i| bmp([i, 0, 0])).collect();
        let months: Vec<u8> = (0..12u8).flat_map(|i| bmp([0, i, 0])).collect();
        let faces = xmb_ground_faces(&others, &months);
        assert_eq!(faces.len(), 34);
        assert_eq!((faces[0].w, faces[0].h), (60, 34));
        assert_eq!(&faces[21].rgba[0..4], &[21, 0, 0, 255]);
        assert_eq!(&faces[33].rgba[0..4], &[0, 11, 0, 255]);
        assert!(xmb_ground_faces(&others, &others).is_empty());
    }
}
