//! A cross-media-bar style browser, laid out after the handheld it is modelled on: categories
//! along a bar near the top, the chosen category's items in a column beneath, settings opening as
//! a list beside a collapsed column of icons, and values picked from a strip down the right edge.
//! Everything is drawn from original shapes, an open-licensed font and a procedural background.

mod art;
mod ground;
mod intro;
pub mod modern;
mod palette;
mod text;
mod waves;

pub use art::{
    xmb_cart_halo_face, xmb_frame_glow_face, xmb_glow_face, xmb_halo_index, xmb_icon_face, XmbIcon,
    XMB_HALOS, XMB_ICONS,
};
pub use palette::{
    month_of, xmb_backdrop_colours, xmb_backdrop_edges, xmb_pause_colours, xmb_pause_edges, xmb_clock_text, xmb_colour_index, xmb_date_time_text, civil, XMB_DATE_FORMATS, XMB_TIME_FORMATS, xmb_swatch, XMB_COLOUR_COUNT, XMB_MONTHS,
    XMB_PALETTE,
};
pub use text::{xmb_text_face, XmbSize};
pub use ground::{xmb_ground_faces, XMB_GROUND_MONTHS, XMB_GROUND_OTHERS};
pub use intro::{
    xmb_flare_face, xmb_intro_shape, xmb_pale, xmb_loading_ring, xmb_outro_t, xmb_ring_face, XmbIntro,
    xmb_boot_picture, xmb_foot_face, xmb_title_alpha, xmb_title_face, XMB_INTRO_TITLE, XMB_MODERN_START, XMB_INTRO_BAR, XMB_INTRO_BLOOM, XMB_INTRO_CLOCK, XMB_INTRO_END, XMB_INTRO_WAVES,
    XMB_OUTRO_SECS,
};
use text::{XMB_TEXT_EXTRA, XMB_TEXT_PAD};
pub use waves::{classic_lines, original_edges, xmb_curves, xmb_curves_at};

use slot_gfx::{Backdrop, Canvas, Draw, TexId};

/// Background styles, as stored in settings.
pub const XMB_WAVE_ORIGINAL: u8 = slot_gfx::STYLE_ORIGINAL;
pub const XMB_WAVE_CLASSIC: u8 = slot_gfx::STYLE_CLASSIC;

/// Textures the bar draws with: icons and the glow built once, the wave curves every frame.
#[derive(Copy, Clone, Debug, Default)]
pub struct XmbArt {
    pub icons: [Option<TexId>; XMB_ICONS],
    pub glow: Option<TexId>,
    /// The halo around the highlighted game's cover.
    pub frame: Option<TexId>,
    pub curves: Option<TexId>,
    /// For the start-up: a glow in the theme's colour and one in white, and the loading ring.
    pub flare: Option<TexId>,
    pub glint: Option<TexId>,
    pub ring: Option<TexId>,
    /// The start-up's lettering and where it goes.
    pub title: Option<(TexId, [f32; 4])>,
    /// The glow from the foot of the screen, in the theme's pale colour.
    pub foot: Option<TexId>,
    /// `xmb_cart_halo_face` for each cart outline, with its margins.
    pub halos: [Option<(TexId, [f32; 2])>; XMB_HALOS],
    /// The handheld's background picture for the current colour, when it is on the card.
    pub ground: Option<TexId>,
}

/// A rendered line of text and its size.
pub type XmbText = (TexId, u32, u32);

pub struct XmbCat {
    pub icon: XmbIcon,
    pub label: Option<XmbText>,
}

pub struct XmbRow {
    /// Where the row sits relative to the selection, scroll included: 0 is selected, 1 the item
    /// below it, -1 the item above the bar.
    pub pos: f32,
    pub face: Option<TexId>,
    /// The part of `face` to show, as x, y, w, h in texture space.
    pub uv: [f32; 4],
    /// Width over height of the part shown.
    pub aspect: f32,
    pub colour: [u8; 3],
    pub title: Option<XmbText>,
    /// For a cart drawn whole, which outline's halo it glows with (`xmb_halo_index`).
    pub halo: Option<usize>,
}

/// How big a column's rows are, in pixels on a 480-line screen.
#[derive(Copy, Clone, Debug)]
pub struct XmbSizes {
    pub small: f32,
    pub big: f32,
    pub pitch: f32,
    pub first_gap: f32,
    /// Games are sized by their height and haloed edge to edge; icons by their longer side
    /// with a round glow.
    pub covers: bool,
    /// Where the highlighted row sits, as a fraction of the screen's height.
    pub sel_y: f32,
}

impl XmbSizes {
    /// Games, as measured off the handheld: covers stacked edge to edge, the highlighted one
    /// nearly twice as tall with a little room either side.
    pub const GAMES: XmbSizes = XmbSizes {
        small: 68.0,
        big: 124.0,
        pitch: 69.0,
        first_gap: 118.0,
        covers: true,
        sel_y: 0.53,
    };
    /// Settings and menus: icons with their names.
    pub const ICONS: XmbSizes = XmbSizes {
        small: 52.0,
        big: 66.0,
        pitch: 66.0,
        first_gap: 82.0,
        covers: false,
        sel_y: SEL_Y,
    };
}

/// A line of a settings list: its badge, name, and value if it shows one.
pub struct XmbItem {
    pub icon: XmbIcon,
    pub label: Option<XmbText>,
    pub value: Option<XmbText>,
}

/// A settings list opened beside the column.
pub struct XmbList<'a> {
    pub items: &'a [XmbItem],
    pub at: usize,
}

pub enum XmbEntry {
    Text(Option<XmbText>),
    Swatch([u8; 3]),
}

/// A strip down the right edge to pick from: a setting's values, or the options on a game.
pub struct XmbPicker<'a> {
    pub entries: &'a [XmbEntry],
    pub at: usize,
    /// The entry shown first at the top of the strip; may be negative to leave space above.
    pub first: i32,
    /// The option menu on a game is wider than a value strip.
    pub wide: bool,
}

/// The date and time being set, in place of the highlighted line's value: its parts in reading
/// order, and which part the arrows change.
pub struct XmbClockEdit<'a> {
    pub parts: &'a [Option<XmbText>],
    pub at: usize,
}

pub struct Xmb<'a> {
    pub canvas: Canvas,
    pub art: XmbArt,
    /// Seconds, for the glow's pulse.
    pub time: f32,
    /// Fades everything but the background, eg while a game starts.
    pub alpha: f32,
    /// The background, unless something else is already drawn underneath.
    pub backdrop: Option<Backdrop>,
    pub cats: &'a [XmbCat],
    /// Where the selection sits along `cats`.
    pub at: usize,
    /// The bar's animated position; equals `at` once it settles.
    pub col: f32,
    pub rows: &'a [XmbRow],
    pub sizes: XmbSizes,
    /// How far into a settings list the screen has moved: 0 at the column, 1 in the list.
    pub nest: f32,
    pub list: Option<XmbList<'a>>,
    pub picker: Option<XmbPicker<'a>>,
    pub options: Option<XmbPicker<'a>>,
    pub clock_edit: Option<XmbClockEdit<'a>>,
    /// The date and time, and the battery as a percentage and whether it is charging.
    pub clock: Option<XmbText>,
    pub battery: Option<(u8, bool)>,
    /// The charge as a number, when shown beside the battery.
    pub battery_text: Option<XmbText>,
    /// The clock and battery fade on their own at start-up, ahead of the rest.
    pub status_alpha: f32,
}

// Layout, as fractions of the screen's width (X) or height (Y).
const COL_X: f32 = 0.25;
const NEST_COL_X: f32 = 0.105;
const CAT_Y: f32 = 0.233;
const NEST_CAT_Y: f32 = 0.276;
const SEL_Y: f32 = 0.492;
const ABOVE_Y: f32 = 0.071;
const NEST_SEL_Y: f32 = 0.49;
const NEST_PITCH: f32 = 0.22;
const NEST_ABOVE_Y: f32 = 0.055;
const NEST_ICON: f32 = 0.115;
const BACK_X: f32 = 0.20;
const LIST_ICON_X: f32 = 0.27;
const LIST_LABEL_X: f32 = 0.317;
const LIST_VALUE_X: f32 = 0.79;
const LIST_SEL_Y: f32 = 0.49;
const LIST_PITCH: f32 = 0.202;
const LIST_ICON: f32 = 0.088;
const STRIP_X: f32 = 0.775;
/// Where the wash behind a date being set begins.
const EDIT_X: f32 = 0.50;
const WIDE_STRIP_X: f32 = 0.62;
const STRIP_TOP: f32 = 0.05;
const STRIP_BOTTOM: f32 = 0.95;
const STRIP_PITCH: f32 = 0.072;
// Pixel sizes on a 480-line screen.
const CAT_PITCH: f32 = 112.0;
const CAT_BIG: f32 = 78.0;
const CAT_SMALL: f32 = 54.0;
const TITLE_GAP: f32 = 18.0;
const GLOW_PERIOD: f32 = 1.8;

/// Which entry a strip should show at its top so that `at` is in view: it moves only when the
/// highlight would leave the strip, as the handheld's does.
pub fn xmb_picker_first(first: i32, at: usize, canvas_h: f32) -> i32 {
    let rows = picker_rows(canvas_h);
    let at = at as i32;
    if at < first {
        at
    } else if at >= first + rows {
        at - rows + 1
    } else {
        first
    }
}

/// The first entry that sets a short list in the middle of its strip, as a menu opens.
pub fn xmb_picker_centred(len: usize) -> i32 {
    -((picker_rows(0.0) - len as i32) / 2).max(0)
}

/// The first entry that puts `at` level with a settings line, where a strip opens.
pub fn xmb_picker_open(at: usize, canvas_h: f32) -> i32 {
    let top = STRIP_TOP * canvas_h + STRIP_PITCH * canvas_h / 2.0;
    let steps = ((LIST_SEL_Y * canvas_h - top) / (STRIP_PITCH * canvas_h)).round() as i32;
    at as i32 - steps
}

/// Entries a strip shows at once. Its rows scale with the screen, so the count does not.
fn picker_rows(_canvas_h: f32) -> i32 {
    (((STRIP_BOTTOM - STRIP_TOP) / STRIP_PITCH).floor() as i32).max(1)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn rect(x: f32, y: f32, w: f32, h: f32, colour: [f32; 4]) -> Draw {
    Draw::Rect { x, y, w, h, colour }
}

impl Xmb<'_> {
    fn s(&self) -> f32 {
        self.canvas.h / 480.0
    }

    fn icon(&self, icon: XmbIcon) -> Option<TexId> {
        self.art.icons[icon.index()]
    }

    fn pulse(&self) -> f32 {
        0.55 + 0.30 * (self.time * std::f32::consts::TAU / GLOW_PERIOD).sin()
    }

    pub fn draw(&self, out: &mut Vec<Draw>) {
        if let Some(b) = self.backdrop {
            out.push(Draw::Backdrop(b));
        }
        let a = self.alpha.clamp(0.0, 1.0);
        if a <= 0.0 {
            if self.status_alpha > 0.0 && self.picker.is_none() && self.options.is_none() {
                self.draw_status(self.status_alpha.clamp(0.0, 1.0), out);
            }
            return;
        }
        let nest = self.nest.clamp(0.0, 1.0);
        let col_x = (self.canvas.w * lerp(COL_X, NEST_COL_X, nest)).round();
        self.draw_rows(col_x, nest, a, out);
        self.draw_cats(col_x, nest, a, out);
        if let Some(list) = &self.list {
            self.draw_list(list, col_x, nest, a, out);
        }
        if let Some(p) = &self.picker {
            self.draw_strip(p, a, out);
        }
        if let Some(p) = &self.options {
            self.draw_strip(p, a, out);
        }
        // A strip, the option menu or a date being set takes the top bar's place.
        if self.picker.is_none() && self.options.is_none() && self.clock_edit.is_none() {
            self.draw_status(self.status_alpha.clamp(0.0, 1.0), out);
        }
    }

    fn draw_cats(&self, col_x: f32, nest: f32, a: f32, out: &mut Vec<Draw>) {
        let s = self.s();
        let y = self.canvas.h * lerp(CAT_Y, NEST_CAT_Y, nest);
        for (k, cat) in self.cats.iter().enumerate() {
            let rel = k as f32 - self.col;
            let near = 1.0 - rel.abs().min(1.0);
            // Deeper in, only the category the list belongs to stays on the bar.
            let keep = if k == self.at { 1.0 } else { 1.0 - nest };
            if keep <= 0.0 {
                continue;
            }
            let x = col_x + rel * CAT_PITCH * s;
            let size = lerp(CAT_SMALL, CAT_BIG, near) * s;
            if x + size < 0.0 || x - size > self.canvas.w {
                continue;
            }
            let alpha = lerp(0.55, 1.0, near) * a * keep;
            if let Some(tex) = self.icon(cat.icon) {
                out.push(Draw::Tex {
                    x: (x - size / 2.0).round(),
                    y: (y - size / 2.0).round(),
                    w: size,
                    h: size,
                    tex,
                    alpha,
                });
            }
            if let (Some((tex, w, h)), true) = (cat.label, near > 0.0) {
                let (w, h) = (w as f32 * s, h as f32 * s);
                out.push(Draw::Tex {
                    x: (x - w / 2.0).round(),
                    y: (y + CAT_BIG * s / 2.0 - 4.0 * s).round(),
                    w,
                    h,
                    tex,
                    alpha: near * near * alpha,
                });
            }
        }
    }

    fn top_row_y(&self, pos: f32) -> f32 {
        let (h, s, z) = (self.canvas.h, self.s(), self.sizes);
        let sel = z.sel_y * h;
        // Above the selection, the gap past the bar mirrors the one below.
        let above = ABOVE_Y * h;
        if pos >= 1.0 {
            sel + z.first_gap * s + (pos - 1.0) * z.pitch * s
        } else if pos >= 0.0 {
            sel + pos * z.first_gap * s
        } else if pos >= -1.0 {
            sel + pos * (sel - above)
        } else {
            above + (pos + 1.0) * z.pitch * s
        }
    }

    fn nest_row_y(&self, pos: f32) -> f32 {
        let h = self.canvas.h;
        if pos >= 0.0 {
            (NEST_SEL_Y + pos * NEST_PITCH) * h
        } else if pos >= -1.0 {
            lerp(NEST_SEL_Y, NEST_ABOVE_Y, -pos) * h
        } else {
            (NEST_ABOVE_Y + (pos + 1.0) * NEST_PITCH) * h
        }
    }

    fn draw_rows(&self, col_x: f32, nest: f32, a: f32, out: &mut Vec<Draw>) {
        let settle = 1.0 - (self.col - self.at as f32).abs().min(1.0);
        let a = a * settle;
        if a <= 0.0 {
            return;
        }
        let s = self.s();
        // The column slides in from the right as the bar settles on a new category.
        let x = col_x + (1.0 - settle) * 40.0 * s;
        for row in self.rows {
            let pos = row.pos;
            let y = lerp(self.top_row_y(pos), self.nest_row_y(pos), nest);
            if y < -120.0 * s || y > self.canvas.h + 120.0 * s {
                continue;
            }
            let near = 1.0 - pos.abs().min(1.0);
            let top_size = lerp(self.sizes.small, self.sizes.big, near) * s;
            let size = lerp(top_size, NEST_ICON * self.canvas.h, nest);
            let base = if pos < -0.5 {
                0.5
            } else {
                lerp(0.72, 1.0, near)
            };
            let alpha = base * a * lerp(1.0, if near > 0.5 { 0.9 } else { 0.55 }, nest);
            let (w, h) = if self.sizes.covers {
                (size * row.aspect, size)
            } else if row.aspect >= 1.0 {
                (size, size / row.aspect)
            } else {
                (size * row.aspect, size)
            };
            let (ix, iy) = ((x - w / 2.0).round(), (y - h / 2.0).round());
            // The highlighted item glows, breathing slowly: a cover all round its edge.
            if self.sizes.covers && near > 0.5 {
                let g = (near - 0.5) * 2.0 * (1.0 - nest) * (self.pulse() * 1.35).min(1.0) * a;
                match row.halo.and_then(|k| self.art.halos.get(k).copied().flatten()) {
                    // A whole cart: the glow follows its outline exactly.
                    Some((tex, [mx, my])) if g > 0.0 => out.push(Draw::Tex {
                        x: ix - w * mx,
                        y: iy - h * my,
                        w: w * (1.0 + 2.0 * mx),
                        h: h * (1.0 + 2.0 * my),
                        tex,
                        alpha: g,
                    }),
                    Some(_) => {}
                    // A label: a rectangle, so the square frame fits it.
                    None => {
                        if let Some(tex) = self.art.frame {
                            nine_slice(tex, ix, iy, w, h, 22.0 * s, g, out);
                        }
                    }
                }
            } else if near > 0.5 && nest < 1.0 {
                if let Some(tex) = self.art.glow {
                    let g = (near - 0.5) * 2.0 * (1.0 - nest);
                    let (gw, gh) = (w + 34.0 * s, h + 34.0 * s);
                    out.push(Draw::Tex {
                        x: x - gw / 2.0,
                        y: y - gh / 2.0,
                        w: gw,
                        h: gh,
                        tex,
                        alpha: self.pulse() * g * a,
                    });
                }
            }
            out.push(match row.face {
                Some(tex) => Draw::Part {
                    x: ix,
                    y: iy,
                    w,
                    h,
                    tex,
                    alpha,
                    uv: row.uv,
                },
                None => rect(
                    ix,
                    iy,
                    w,
                    h,
                    [
                        row.colour[0] as f32 / 255.0,
                        row.colour[1] as f32 / 255.0,
                        row.colour[2] as f32 / 255.0,
                        alpha,
                    ],
                ),
            });
            if near > 0.5 && nest > 0.0 {
                if let Some(tex) = self.icon(XmbIcon::Back) {
                    let b = 14.0 * s;
                    out.push(Draw::Tex {
                        x: (self.canvas.w * BACK_X + col_x - self.canvas.w * NEST_COL_X).round(),
                        y: (y - b / 2.0).round(),
                        w: b,
                        h: b,
                        tex,
                        alpha: a * nest,
                    });
                }
            }
            let mut title_alpha = alpha * (1.0 - nest);
            // Games show only the highlighted one's name; settings name every icon.
            if self.sizes.covers {
                title_alpha *= ((near - 0.5) * 2.0).clamp(0.0, 1.0);
            }
            if title_alpha <= 0.0 {
                continue;
            }
            if let Some((tex, tw, th)) = row.title {
                let scale = lerp(0.8, 1.0, near) * s;
                let half = if self.sizes.covers { w / 2.0 } else { size / 2.0 };
                let tx = x + half + TITLE_GAP * s;
                text(tex, tw, th, scale, tx, y, self.canvas.w - 10.0, title_alpha, out);
            }
        }
    }

    fn draw_list(&self, list: &XmbList, col_x: f32, nest: f32, a: f32, out: &mut Vec<Draw>) {
        let (w, h, s) = (self.canvas.w, self.canvas.h, self.s());
        let a = a * nest;
        if a <= 0.0 {
            return;
        }
        let shift = col_x - w * NEST_COL_X;
        let editing = self.clock_edit.is_some();
        let picking = self.picker.is_some() || editing;
        let right = match (editing, picking) {
            (true, _) => w * EDIT_X,
            (_, true) => w * STRIP_X,
            _ => w - 10.0,
        };
        if editing {
            // The handheld washes the right of the screen behind the date being set.
            out.push(rect((w * EDIT_X).round(), 0.0, w - (w * EDIT_X).round(), h, [0.5, 0.5, 0.5, 0.26 * a]));
        }
        for (k, item) in list.items.iter().enumerate() {
            let rel = k as f32 - list.at as f32;
            let y = (LIST_SEL_Y + rel * LIST_PITCH) * h;
            if y < -40.0 * s || y > h + 40.0 * s {
                continue;
            }
            let lit = k == list.at;
            let alpha = if lit { 1.0 } else { 0.5 } * a;
            let size = LIST_ICON * h;
            let ix = w * LIST_ICON_X + shift;
            if lit {
                if let Some(tex) = self.art.glow {
                    let g = size * 1.9;
                    out.push(Draw::Tex {
                        x: ix - g / 2.0,
                        y: y - g / 2.0,
                        w: g,
                        h: g,
                        tex,
                        alpha: self.pulse() * a,
                    });
                }
            }
            if let Some(tex) = self.icon(item.icon) {
                out.push(Draw::Tex {
                    x: (ix - size / 2.0).round(),
                    y: (y - size / 2.0).round(),
                    w: size,
                    h: size,
                    tex,
                    alpha: if lit { a } else { 0.8 * a },
                });
            }
            // With a strip open, the other lines keep only their badges.
            if picking && !lit {
                continue;
            }
            let lx = w * LIST_LABEL_X + shift;
            if let Some((tex, tw, th)) = item.label {
                let room = if lit || !picking { right } else { w * LIST_VALUE_X - 12.0 };
                text(tex, tw, th, s, lx, y, room, alpha, out);
                if lit && picking {
                    // The opened line is underlined across to the strip, or across the screen
                    // under a date being set.
                    let end = if editing { w * 0.98 } else { right };
                    let uy = (y + th as f32 * s * 0.36).round();
                    out.push(rect(lx, uy, end - lx, (1.5 * s).max(1.0), [1.0, 1.0, 1.0, 0.85 * a]));
                }
            }
            if lit {
                if let Some(edit) = &self.clock_edit {
                    self.draw_clock_edit(edit, y, a, out);
                    continue;
                }
            }
            if !picking {
                if let Some((tex, tw, th)) = item.value {
                    // A long value moves left to fit rather than being cut off.
                    let vx = (w * LIST_VALUE_X).min(w - 10.0 * s - tw as f32 * s) + shift;
                    text(tex, tw, th, s, vx, y, w - 6.0, alpha, out);
                }
            }
        }
    }

    /// The date and time being set, on the line it belongs to: one run of text, the field being
    /// changed marked by a lit box with arrows above and below.
    fn draw_clock_edit(&self, edit: &XmbClockEdit, y: f32, a: f32, out: &mut Vec<Draw>) {
        let (w, s) = (self.canvas.w, self.s());
        let space = 6.0 * s;
        // Each part's texture carries padding around its ink; lay the ink edge to edge.
        let ink = |tw: u32| (tw as f32 - XMB_TEXT_EXTRA) * s;
        let total: f32 = edit
            .parts
            .iter()
            .map(|p| p.map_or(space, |(_, tw, _)| ink(tw)))
            .sum();
        let mut x = (w * (EDIT_X + 0.012)).min(w * 0.975 - total).round();
        for (k, part) in edit.parts.iter().enumerate() {
            let Some((tex, tw, th)) = *part else {
                x += space;
                continue;
            };
            let pw = ink(tw);
            let ph = th as f32 * s;
            if k == edit.at {
                let pad = 2.0 * s;
                out.push(rect(
                    (x - pad).round(),
                    (y - ph * 0.30).round(),
                    (pw + 2.0 * pad).round(),
                    (ph * 0.60).round(),
                    [1.0, 1.0, 1.0, 0.28 * a],
                ));
                let b = 11.0 * s;
                for (icon, ay) in [(XmbIcon::Up, y - ph * 0.30 - b - 1.0 * s), (XmbIcon::Down, y + ph * 0.30 + 1.0 * s)] {
                    if let Some(tex) = self.icon(icon) {
                        out.push(Draw::Tex {
                            x: (x + pw / 2.0 - b / 2.0).round(),
                            y: ay.round(),
                            w: b,
                            h: b,
                            tex,
                            alpha: a,
                        });
                    }
                }
            }
            text(tex, tw, th, s, x - XMB_TEXT_PAD * s, y, w, a, out);
            x += pw;
        }
    }

    fn draw_strip(&self, p: &XmbPicker, a: f32, out: &mut Vec<Draw>) {
        let (w, h, s) = (self.canvas.w, self.canvas.h, self.s());
        let x0 = (w * if p.wide { WIDE_STRIP_X } else { STRIP_X }).round();
        // A neutral wash: it lifts a dark colour and sinks a bright one, as the handheld's does.
        out.push(rect(x0, 0.0, w - x0, h, [0.5, 0.5, 0.5, 0.26 * a]));
        let pitch = STRIP_PITCH * h;
        let top = STRIP_TOP * h + pitch / 2.0;
        let rows = picker_rows(h);
        for (k, entry) in p.entries.iter().enumerate() {
            let rel = k as i32 - p.first;
            if rel < 0 || rel >= rows {
                continue;
            }
            let y = top + rel as f32 * pitch;
            let lit = k == p.at;
            if lit {
                out.push(rect(x0, y - pitch * 0.42, w - x0, pitch * 0.84, [1.0, 1.0, 1.0, 0.38 * a]));
            }
            let alpha = a;
            match entry {
                XmbEntry::Text(Some((tex, tw, th))) => {
                    text(*tex, *tw, *th, s, x0 + 12.0 * s, y, w - 6.0, alpha, out);
                }
                XmbEntry::Text(None) => {}
                XmbEntry::Swatch(c) => {
                    let q = 0.040 * h;
                    out.push(rect(
                        (x0 + 14.0 * s).round(),
                        (y - q / 2.0).round(),
                        q,
                        q,
                        [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, alpha],
                    ));
                }
            }
        }
        // Arrows say there is more above or below.
        let more_above = p.first > 0;
        let more_below = (p.entries.len() as i32) > p.first + rows;
        let b = 12.0 * s;
        for (icon, on, y) in [
            (XmbIcon::Up, more_above, STRIP_TOP * h * 0.5),
            (XmbIcon::Down, more_below, h - STRIP_TOP * h * 0.5),
        ] {
            if let (true, Some(tex)) = (on, self.icon(icon)) {
                out.push(Draw::Tex {
                    x: (x0 + (w - x0) / 2.0 - b / 2.0).round(),
                    y: (y - b / 2.0).round(),
                    w: b,
                    h: b,
                    tex,
                    alpha: a,
                });
            }
        }
    }

    fn draw_status(&self, a: f32, out: &mut Vec<Draw>) {
        let (w, h, s) = (self.canvas.w, self.canvas.h, self.s());
        let y = 0.045 * h;
        let mut right = w - 10.0 * s;
        if let Some((percent, charging)) = self.battery {
            // Charging, the bars fill one by one and start again.
            let bars = match charging {
                true => (self.time / 0.6) as u32 % 4,
                false => (u32::from(percent) * 3).div_ceil(100).min(3),
            };
            let icon = [XmbIcon::Battery0, XmbIcon::Battery1, XmbIcon::Battery2, XmbIcon::Battery3]
                [bars as usize];
            let b = 34.0 * s;
            if let Some(tex) = self.icon(icon) {
                // The cell sits a touch low in its square so it lines up with the text's ink.
                out.push(Draw::Tex {
                    x: (right - b * 0.93).round(),
                    y: (y - b / 2.0 + 2.5 * s).round(),
                    w: b,
                    h: b,
                    tex,
                    alpha: a,
                });
            }
            right -= b * 0.93 + 6.0 * s;
            if let Some((tex, tw, th)) = self.battery_text {
                let tw_s = (tw as f32 - XMB_TEXT_EXTRA) * s;
                text(tex, tw, th, s, right - tw_s - XMB_TEXT_PAD * s, y, w, a, out);
                right -= tw_s + 10.0 * s;
            }
        }
        if let Some((tex, tw, th)) = self.clock {
            let tw_s = (tw as f32 - XMB_TEXT_EXTRA) * s;
            text(tex, tw, th, s, right - tw_s - XMB_TEXT_PAD * s, y, w, a, out);
        }
    }
}

/// A halo `m` wide around the rectangle `x`, `y`, `w`, `h`, cut from the frame glow's nine
/// slices so its width holds whatever the rectangle's shape.
#[allow(clippy::too_many_arguments)]
fn nine_slice(tex: TexId, x: f32, y: f32, w: f32, h: f32, m: f32, alpha: f32, out: &mut Vec<Draw>) {
    if alpha <= 0.0 {
        return;
    }
    let e = art::FRAME_INSET as f32 / 64.0;
    let mid = 1.0 - 2.0 * e;
    let xs = [(x - m, m, 0.0, e), (x, w, e, mid), (x + w, m, 1.0 - e, e)];
    let ys = [(y - m, m, 0.0, e), (y, h, e, mid), (y + h, m, 1.0 - e, e)];
    for (i, (px, pw, u, uw)) in xs.into_iter().enumerate() {
        for (j, (py, ph, v, vh)) in ys.into_iter().enumerate() {
            // The middle is under the cover.
            if i == 1 && j == 1 {
                continue;
            }
            out.push(Draw::Part {
                x: px,
                y: py,
                w: pw,
                h: ph,
                tex,
                alpha,
                uv: [u, v, uw, vh],
            });
        }
    }
}

/// A line of text with its left edge at `x`, centred on `y`, cut off at `right`.
#[allow(clippy::too_many_arguments)]
fn text(tex: TexId, w: u32, h: u32, scale: f32, x: f32, y: f32, right: f32, alpha: f32, out: &mut Vec<Draw>) {
    let (tw, th) = (w as f32 * scale, h as f32 * scale);
    let shown = tw.min(right - x);
    if shown <= 0.0 || tw <= 0.0 {
        return;
    }
    out.push(Draw::Part {
        x: x.round(),
        y: (y - th / 2.0).round(),
        w: shown,
        h: th,
        tex,
        alpha,
        uv: [0.0, 0.0, shown / tw, 1.0],
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn art_renders() {
        for icon in XmbIcon::ALL {
            let f = xmb_icon_face(icon);
            assert!(f.rgba.chunks(4).any(|px| px[3] > 0), "{icon:?} is empty");
        }
        assert!(xmb_glow_face().rgba.chunks(4).any(|p| p[3] > 200));
    }

    #[test]
    fn a_strip_scrolls_only_at_its_ends() {
        let h = 480.0;
        let rows = picker_rows(h);
        assert_eq!(xmb_picker_first(0, 3, h), 0);
        assert_eq!(xmb_picker_first(0, rows as usize, h), 1);
        assert_eq!(xmb_picker_first(5, 2, h), 2);
        // A strip opens with the current value level with its line.
        let first = xmb_picker_open(10, h);
        let top = STRIP_TOP * h + STRIP_PITCH * h / 2.0;
        let y = top + (10 - first) as f32 * STRIP_PITCH * h;
        assert!((y - LIST_SEL_Y * h).abs() < STRIP_PITCH * h / 2.0);
    }
}
