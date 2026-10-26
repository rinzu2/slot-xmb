//! Lines of text for the bar: mixed case, in a wide rounded face, white with a soft shadow.

use std::sync::OnceLock;

use fontdue::{Font, FontSettings};

use crate::CartFace;

const XMB_TTF: &[u8] = include_bytes!("../../assets/xmb.ttf");

/// The sizes text is drawn at. Titles are the names of games and settings; small text is values,
/// the clock and option lists; category names sit between.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum XmbSize {
    Title,
    Category,
    Small,
}

impl XmbSize {
    pub const ALL: [XmbSize; 3] = [XmbSize::Title, XmbSize::Category, XmbSize::Small];

    pub fn px(self) -> f32 {
        match self {
            XmbSize::Title => 25.0,
            XmbSize::Category => 20.0,
            XmbSize::Small => 18.0,
        }
    }

    /// A short tag, so one text cache can hold every size.
    pub fn key(self, text: &str) -> String {
        let tag = match self {
            XmbSize::Title => 'T',
            XmbSize::Category => 'C',
            XmbSize::Small => 'S',
        };
        format!("{tag}{text}")
    }

    pub fn of_key(key: &str) -> Option<(XmbSize, &str)> {
        let mut chars = key.chars();
        let size = match chars.next()? {
            'T' => XmbSize::Title,
            'C' => XmbSize::Category,
            'S' => XmbSize::Small,
            _ => return None,
        };
        Some((size, chars.as_str()))
    }
}

/// Longest line drawn, in pixels; longer text is cut at a word with an ellipsis.
const XMB_TEXT_MAX_W: f32 = 360.0;

fn font() -> Option<&'static Font> {
    static FONT: OnceLock<Option<Font>> = OnceLock::new();
    FONT.get_or_init(|| Font::from_bytes(XMB_TTF, FontSettings::default()).ok())
        .as_ref()
}

fn width(font: &Font, text: &str, px: f32) -> f32 {
    text.chars()
        .map(|c| font.metrics(c, px).advance_width)
        .sum()
}

/// `text`, shortened at a word boundary with an ellipsis until it fits `max`.
fn fitted(font: &Font, text: &str, px: f32, max: f32) -> String {
    if width(font, text, px) <= max {
        return text.to_string();
    }
    let mut words: Vec<&str> = text.split_whitespace().collect();
    while words.len() > 1 {
        words.pop();
        let t = format!("{}…", words.join(" "));
        if width(font, &t, px) <= max {
            return t;
        }
    }
    let mut t: String = text.chars().collect();
    while t.chars().count() > 1 {
        t.pop();
        let c = format!("{t}…");
        if width(font, &c, px) <= max {
            return c;
        }
    }
    t
}

/// Blank texels left of a line's ink, and the extra width beyond its advance in all.
pub(crate) const XMB_TEXT_PAD: f32 = 4.0;
pub(crate) const XMB_TEXT_EXTRA: f32 = 10.0;

/// A white line of text with a soft shadow below and to the right, as wide as its ink.
pub fn xmb_text_face(text: &str, size: XmbSize) -> CartFace {
    let px = size.px();
    let pad = 4u32;
    let h = (px * 1.45).ceil() as u32 + pad * 2;
    let Some(font) = font() else {
        return CartFace {
            rgba: vec![0; (h * 4) as usize],
            w: 1,
            h,
        };
    };
    let text = fitted(font, text, px, XMB_TEXT_MAX_W);
    let w = (width(font, &text, px).ceil() as u32 + pad * 2 + 2).max(1);
    let mut cov = vec![0u8; (w * h) as usize];
    let ascent = font
        .horizontal_line_metrics(px)
        .map_or(px * 0.9, |m| m.ascent);
    let baseline = pad as f32 + (h as f32 - 2.0 * pad as f32 - px) / 2.0 + ascent * 0.92;
    let mut pen = pad as f32;
    for ch in text.chars() {
        let (m, glyph) = font.rasterize(ch, px);
        let gx = (pen + m.xmin as f32).round() as i32;
        let gy = (baseline - m.height as f32 - m.ymin as f32).round() as i32;
        for y in 0..m.height {
            for x in 0..m.width {
                let (dx, dy) = (gx + x as i32, gy + y as i32);
                if dx < 0 || dy < 0 || dx >= w as i32 || dy >= h as i32 {
                    continue;
                }
                let i = (dy as u32 * w + dx as u32) as usize;
                cov[i] = cov[i].max(glyph[y * m.width + x]);
            }
        }
        pen += m.advance_width;
    }
    // The shadow: the ink blurred and nudged down and right, under the ink itself.
    let shadow = blur(&blur(&cov, w, h), w, h);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let a = cov[i] as f32 / 255.0;
            let s = match (x.checked_sub(2), y.checked_sub(2)) {
                (Some(sx), Some(sy)) => shadow[(sy * w + sx) as usize] as f32 / 255.0 * 0.6,
                _ => 0.0,
            };
            let total = a + s * (1.0 - a);
            if total <= 0.0 {
                continue;
            }
            let lum = (255.0 * a / total) as u8;
            rgba[i * 4..i * 4 + 4].copy_from_slice(&[lum, lum, lum, (total * 255.0) as u8]);
        }
    }
    CartFace { rgba, w, h }
}

fn blur(src: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (w, h) = (w as i32, h as i32);
    let mut out = vec![0u8; src.len()];
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0u32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (sx, sy) = ((x + dx).clamp(0, w - 1), (y + dy).clamp(0, h - 1));
                    sum += src[(sy * w + sx) as usize] as u32;
                }
            }
            out[(y * w + x) as usize] = (sum / 9) as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        for size in XmbSize::ALL {
            let k = size.key("Theme Settings");
            assert_eq!(XmbSize::of_key(&k), Some((size, "Theme Settings")));
        }
    }

    #[test]
    fn text_keeps_its_case_and_inks() {
        let f = xmb_text_face("Color", XmbSize::Title);
        assert!(f.rgba.chunks(4).any(|p| p[3] > 200));
        let long = xmb_text_face(&"Background ".repeat(12), XmbSize::Title);
        assert!(long.w as f32 <= XMB_TEXT_MAX_W + 12.0);
    }
}
