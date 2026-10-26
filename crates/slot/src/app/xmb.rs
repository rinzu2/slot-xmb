//! The cross-media-bar browser's side of the app: what the bar shows, and how it answers the
//! buttons. Drawing is in `slot_ui::Xmb`; the background in `slot_gfx::Backdrop`.

use std::cell::RefCell;

use super::*;
use slot_power::Charge;
use slot_ui::Field;
use slot_ui::{
    Xmb, XmbCat, XmbEntry, XmbIcon, XmbItem, XmbList, XmbPicker, XmbRow, XmbSize, XmbSizes, XmbText,
};

/// A value strip open on a setting.
#[derive(Copy, Clone, Debug)]
pub(super) struct XmbPick {
    /// The entry shown at the top of the strip.
    first: i32,
    /// Steps taken since the strip opened, so Back can undo them.
    moved: i32,
}

/// The option menu open on a game.
#[derive(Copy, Clone, Debug)]
pub(super) struct XmbOptions {
    at: usize,
    core: Core,
}

/// One choice in a value strip.
enum Choice {
    Text(String),
    Swatch(u32),
}

/// A settings list beside the column, and which line is selected.
type ListPart = Option<(Vec<XmbItem>, usize)>;
/// A value strip: its entries, which is current, and the entry shown at its top.
type StripPart = Option<(Vec<XmbEntry>, usize, i32)>;

/// Everything one frame of the bar shows, with its text looked up as it is built so the same
/// code says which lines of text need rendering.
struct Scene {
    cats: Vec<XmbCat>,
    at: usize,
    rows: Vec<XmbRow>,
    sizes: XmbSizes,
    list: ListPart,
    picker: StripPart,
    options: Option<(Vec<XmbEntry>, usize)>,
    clock_edit: Option<(Vec<Option<XmbText>>, usize)>,
    clock: Option<XmbText>,
    battery_text: Option<XmbText>,
}

/// Looks text up by size and content, remembering every line asked for.
struct Texts<'a> {
    have: &'a HashMap<String, XmbText>,
    want: RefCell<Vec<String>>,
}

impl Texts<'_> {
    fn get(&self, size: XmbSize, text: &str) -> Option<XmbText> {
        if text.is_empty() {
            return None;
        }
        let key = size.key(text);
        let found = self.have.get(&key).copied();
        let mut want = self.want.borrow_mut();
        if !want.contains(&key) {
            want.push(key);
        }
        found
    }
}

const GAMES_ABOVE: i32 = 3;
const GAMES_BELOW: i32 = 4;

impl App {
    /// The shelves the bar shows after Settings, in order: every one with something on it.
    pub(super) fn xmb_shelves(&self) -> Vec<usize> {
        (0..self.shelves.len())
            .filter(|at| !self.shelves[*at].1.carts.is_empty())
            .collect()
    }

    pub(super) fn xmb_at(&self) -> usize {
        self.xmb_shelves()
            .iter()
            .position(|at| *at == self.shelf_at)
            .unwrap_or(0)
    }

    fn xmb_in_settings(&self) -> bool {
        matches!(
            self.phase,
            Phase::QuickMenu { .. } | Phase::About | Phase::SetClock { .. }
        )
    }

    /// The clock's fields in the order the date format reads them, then the time. The zone is
    /// a setting of its own, as on the handheld.
    pub(super) fn xmb_clock_fields(&self) -> [Field; 5] {
        let date = match self.state.xmb_date_format {
            1 => [Field::Day, Field::Month, Field::Year],
            2 => [Field::Year, Field::Month, Field::Day],
            _ => [Field::Month, Field::Day, Field::Year],
        };
        [date[0], date[1], date[2], Field::Hour, Field::Minute]
    }

    /// Left and right move along the fields as they read on screen.
    pub(super) fn xmb_clock_move(&mut self, by: i32) {
        let order = self.xmb_clock_fields();
        let Phase::SetClock { picker, .. } = &mut self.phase else {
            return;
        };
        let at = order
            .iter()
            .position(|f| *f == picker.cursor())
            .unwrap_or(0) as i32;
        let to = (at + by).clamp(0, order.len() as i32 - 1) as usize;
        picker.field(order[to]);
    }

    fn xmb_clock_parts(&self, picker: &ClockPicker, t: &Texts) -> (Vec<Option<XmbText>>, usize) {
        let mut parts = Vec::new();
        let mut at = 0;
        let mut push = |text: String, field: Option<Field>, parts: &mut Vec<Option<XmbText>>| {
            if field.is_some_and(|f| f == picker.cursor()) {
                at = parts.len();
            }
            parts.push(t.get(XmbSize::Small, &text));
        };
        let order = self.xmb_clock_fields();
        for (i, f) in order[..3].iter().enumerate() {
            let text = match f {
                Field::Year => format!("{:04}", picker.year()),
                Field::Month => format!("{:02}", picker.month()),
                _ => format!("{:02}", picker.day()),
            };
            push(text, Some(*f), &mut parts);
            if i < 2 {
                push("/".to_string(), None, &mut parts);
            }
        }
        parts.push(None);
        let h = picker.hour();
        let shown = match self.state.xmb_clock24 {
            true => h,
            false => (h + 11) % 12 + 1,
        };
        push(format!("{shown:02}"), Some(Field::Hour), &mut parts);
        push(":".to_string(), None, &mut parts);
        push(
            format!("{:02}", picker.minute()),
            Some(Field::Minute),
            &mut parts,
        );
        push(":".to_string(), None, &mut parts);
        push(
            format!("{:02}", self.wall_secs().rem_euclid(60)),
            None,
            &mut parts,
        );
        if !self.state.xmb_clock24 {
            parts.push(None);
            push(
                if h < 12 { "AM" } else { "PM" }.to_string(),
                None,
                &mut parts,
            );
        }
        (parts, at)
    }

    /// Where the bar comes to rest: Settings is 0, the shelves follow.
    pub(super) fn xmb_target(&self) -> f32 {
        match self.xmb_in_settings() {
            true => 0.0,
            false => 1.0 + self.xmb_at() as f32,
        }
    }

    /// How far into a settings list the screen should be: 1 inside one, 0 on a column.
    pub(super) fn xmb_nest_target(&self) -> f32 {
        let deep = match (&self.phase, self.in_game_menu) {
            (Phase::Playing { .. }, Some(row)) => !QuickRow::PLAY.contains(&row),
            (Phase::QuickMenu { row }, _) => !QuickRow::MAIN.contains(row),
            (Phase::About, _) | (Phase::SetClock { .. }, _) => true,
            _ => false,
        };
        if deep {
            1.0
        } else {
            0.0
        }
    }

    /// The colour the background is lit with, 1..=34.
    pub fn xmb_colour(&self) -> u32 {
        slot_ui::xmb_colour_index(self.state.xmb_colour, self.wall_secs())
    }

    /// The waves drawn: Classic when chosen, Original otherwise (a wallpaper background still
    /// shows Original's waves behind the in-game menu).
    pub fn xmb_wave_style(&self) -> u8 {
        self.state.xmb_wave.min(1)
    }

    /// Whether the bar is only drifting: on its own screens, left alone a moment, nothing
    /// settling. The waves move slowly enough then to be drawn at a fraction of the rate.
    pub fn xmb_calm(&self) -> bool {
        const QUIET_MS: Millis = 1500;
        XMB && matches!(
            self.phase,
            Phase::Shelf | Phase::QuickMenu { .. } | Phase::About | Phase::SetClock { .. }
        ) && self.now().saturating_sub(self.xmb_touched) > QUIET_MS
            && (self.xmb_col - self.xmb_target()).abs() < 1e-3
            && (self.xmb_nest - self.xmb_nest_target()).abs() < 1e-3
            && !self.hud.visible(self.now())
            && !self.xmb_intro_active()
    }

    /// Whether the home screen shows the wallpaper rather than waves.
    fn xmb_wallpapered(&self) -> bool {
        self.state.xmb_wave == 2
    }

    fn xmb_wallpaper_choices(&self) -> Vec<WallpaperPick> {
        let mut c = vec![
            WallpaperPick::None,
            WallpaperPick::Random,
            WallpaperPick::Default,
            WallpaperPick::Blur,
        ];
        c.extend(self.wallpaper_files().into_iter().map(WallpaperPick::File));
        c
    }

    fn xmb_wallpaper_label(pick: &WallpaperPick) -> String {
        match pick {
            WallpaperPick::Blur => "Default Blur".to_string(),
            other => other.label(),
        }
    }

    fn xmb_battery_text(&self, t: &Texts) -> Option<XmbText> {
        let b = self.battery.filter(|_| self.state.xmb_battery_pct)?;
        t.get(XmbSize::Small, &format!("{}%", b.percent))
    }

    fn xmb_clock_now(&self) -> String {
        slot_ui::xmb_clock_text(
            self.wall_secs(),
            self.state.xmb_date_format,
            self.state.xmb_clock24,
        )
    }

    /// Seconds for the waves and the glow, wrapped every ten hours to keep f32 precise.
    pub fn xmb_time(&self) -> f32 {
        // Shifted to run on from the device's start-up animation without a jump.
        ((self.now() as f64 + self.xmb_time_shift).rem_euclid(36_000_000.0) / 1000.0) as f32
    }

    /// Whether the bar's background is on screen this frame.
    pub fn xmb_visible(&self) -> bool {
        if !XMB {
            return false;
        }
        match &self.phase {
            Phase::Shelf | Phase::QuickMenu { .. } | Phase::About | Phase::SetClock { .. } => true,
            Phase::Inserting { resumed, .. } => !resumed,
            Phase::Ejecting { .. } => !self.shows_game() && self.screen <= 0.0,
            Phase::Playing { .. } => self.in_game_menu_shown().is_some() && self.screen >= 1.0,
            _ => false,
        }
    }

    pub fn set_xmb_art(&mut self, art: slot_ui::XmbArt) {
        self.xmb_art = art;
    }

    pub fn set_xmb_texts(&mut self, texts: HashMap<String, XmbText>) {
        self.xmb_texts = texts;
    }

    /// Every line of text the bar shows this frame, as keys `XmbSize::of_key` reads.
    pub fn xmb_texts_wanted(&self) -> Vec<String> {
        if !XMB {
            return Vec::new();
        }
        let t = Texts {
            have: &self.xmb_texts,
            want: RefCell::new(Vec::new()),
        };
        if let (Some(row), Phase::Playing { .. }) = (self.in_game_menu, &self.phase) {
            self.xmb_in_game_scene(row, &t);
        } else {
            self.xmb_scene(&t);
        }
        t.want.into_inner()
    }

    /// What a settings row shows on its right, if it has a value.
    fn xmb_value_text(&self, row: QuickRow) -> Option<String> {
        if row.binds() {
            return Some(match self.capture_row() == Some(row) {
                true => RECORDING.to_string(),
                false => self
                    .bound_text(row)
                    .trim_start_matches(CLASH_MARK)
                    .to_string(),
            });
        }
        let bezel = |p: Platform| Some(self.resolved_bezel(p).label());
        match row {
            QuickRow::XmbWave => Some(
                match self.state.xmb_wave {
                    0 => "Original",
                    1 => "Classic",
                    _ => "Wallpaper",
                }
                .to_string(),
            ),
            QuickRow::XmbWallpaper => Some(Self::xmb_wallpaper_label(&self.resolved_wallpaper())),
            QuickRow::XmbSetClock => Some(slot_ui::xmb_date_time_text(
                self.wall_secs(),
                self.state.xmb_date_format,
                self.state.xmb_clock24,
            )),
            QuickRow::XmbDateFormat => Some(
                slot_ui::XMB_DATE_FORMATS[usize::from(self.state.xmb_date_format.min(2))]
                    .to_string(),
            ),
            QuickRow::XmbTimeFormat => {
                Some(slot_ui::XMB_TIME_FORMATS[usize::from(self.state.xmb_clock24)].to_string())
            }
            QuickRow::XmbTimeZone => Some(zone_text(self.state.utc_offset_min)),
            QuickRow::XmbBattery => Some(on_off(self.state.xmb_battery_pct)),
            QuickRow::XmbPause => Some(on_off(self.state.xmb_pause_dark)),
            // The handheld shows no value beside its colour; the strip shows the swatches.
            QuickRow::XmbColour => None,
            QuickRow::XmbIcons => Some(
                match self.state.xmb_labels {
                    true => "Label",
                    false => "Cartridge",
                }
                .to_string(),
            ),
            QuickRow::DateTime => Some(slot_ui::date_time_text(self.wall_secs())),
            QuickRow::Wallpaper => Some(self.resolved_wallpaper().label()),
            QuickRow::GbPalette | QuickRow::Palette => Some(self.gb_colors_label().to_string()),
            QuickRow::GbaBezel => bezel(Platform::Gba),
            QuickRow::GbcBezel => bezel(Platform::Gbc),
            QuickRow::GbBezel => bezel(Platform::Gb),
            QuickRow::Bezel => bezel(self.playing_platform()),
            _ => self.quick_value(row).map(|v| v.text().to_string()),
        }
    }

    /// A row whose value is picked from a strip, rather than one that opens something.
    fn xmb_editable(&self, row: QuickRow) -> bool {
        !row.opens()
            && !row.binds()
            && row.child().is_none()
            && (row == QuickRow::XmbColour || self.xmb_value_text(row).is_some())
    }

    /// The choices a setting's strip lists, and which is current. A setting whose choices
    /// cannot be listed ahead shows its current value alone, still stepped with up and down.
    fn xmb_choices(&self, row: QuickRow) -> (Vec<Choice>, usize) {
        let texts = |items: &[&str], at: usize| {
            (
                items
                    .iter()
                    .map(|s| Choice::Text(s.to_string()))
                    .collect::<Vec<_>>(),
                at.min(items.len().saturating_sub(1)),
            )
        };
        match row {
            QuickRow::XmbColour => {
                let mut c = vec![Choice::Text("By Month".to_string())];
                c.extend((1..=slot_ui::XMB_COLOUR_COUNT as u32).map(Choice::Swatch));
                let at = usize::from(self.state.xmb_colour).min(c.len() - 1);
                return (c, at);
            }
            QuickRow::XmbWave => {
                return texts(
                    &["Original", "Classic", "Wallpaper"],
                    self.state.xmb_wave.into(),
                )
            }
            QuickRow::XmbDateFormat => {
                return texts(
                    &slot_ui::XMB_DATE_FORMATS,
                    self.state.xmb_date_format.into(),
                )
            }
            QuickRow::XmbTimeFormat => {
                return texts(&slot_ui::XMB_TIME_FORMATS, self.state.xmb_clock24.into())
            }
            QuickRow::XmbBattery => {
                return texts(&["Off", "On"], self.state.xmb_battery_pct.into())
            }
            QuickRow::XmbPause => return texts(&["Off", "On"], self.state.xmb_pause_dark.into()),
            QuickRow::XmbTimeZone => {
                let (lo, hi) = (slot_store::UTC_OFFSET_MIN, slot_store::UTC_OFFSET_MAX);
                let names: Vec<String> = (lo..=hi).step_by(30).map(zone_text).collect();
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                let at = ((self.state.utc_offset_min - lo) / 30) as usize;
                return texts(&names, at);
            }
            QuickRow::XmbWallpaper => {
                let list = self.xmb_wallpaper_choices();
                let key = self.resolved_wallpaper().key();
                let at = list.iter().position(|c| c.key() == key).unwrap_or(2);
                let names: Vec<String> = list.iter().map(Self::xmb_wallpaper_label).collect();
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                return texts(&names, at);
            }
            QuickRow::XmbIcons => {
                return texts(&["Cartridge", "Label"], self.state.xmb_labels.into())
            }
            _ => {}
        }
        let current = self.xmb_value_text(row).unwrap_or_default();
        // Shaders list the ones the row's console offers, Off first.
        let shaders = match self.playing_row(row) {
            QuickRow::GbaShader => Some(&slot_store::Shader::GBA[..]),
            QuickRow::GbShader | QuickRow::GbcShader => Some(&slot_store::Shader::GB[..]),
            _ => None,
        };
        if let Some(list) = shaders {
            let names: Vec<&str> = list.iter().map(|s| shader_value(*s).text()).collect();
            if let Some(at) = names.iter().position(|n| *n == current) {
                return texts(&names, at);
            }
        }
        // The list a row picks from follows the row, not its current value: a blue-light row
        // at Off still lists Off then the levels.
        let family: Option<Vec<QuickValue>> = self.quick_value(row).and_then(|v| {
            use QuickValue as V;
            let levels = |from: usize| V::LEVELS[from..].to_vec();
            Some(match self.playing_row(row) {
                QuickRow::Brightness => levels(1),
                QuickRow::BlueLight => {
                    let mut l = vec![V::Off];
                    l.extend(levels(1));
                    l
                }
                QuickRow::FastForward => vec![V::Speed2, V::Speed3, V::Speed4, V::Speed6],
                QuickRow::WallpaperTint => vec![V::Off, V::Low, V::Medium, V::High],
                QuickRow::LidOff => V::LID_OFF.to_vec(),
                QuickRow::Theme => vec![V::Dark, V::Light],
                _ => match v {
                    V::On | V::Off => vec![V::On, V::Off],
                    V::Show | V::Hide => vec![V::Show, V::Hide],
                    _ if V::LEVELS.contains(&v) => levels(0),
                    _ => return None,
                },
            })
        });
        // Off on a blue-light row reads "Off", not "0".
        if let Some(list) = family {
            let names: Vec<&str> = list.iter().map(|v| v.text()).collect();
            if let Some(at) = names.iter().position(|n| *n == current) {
                return texts(&names, at);
            }
        }
        (vec![Choice::Text(current)], 0)
    }

    fn xmb_row_icon(row: QuickRow) -> XmbIcon {
        match row {
            QuickRow::General => XmbIcon::General,
            QuickRow::XmbTheme => XmbIcon::Theme,
            QuickRow::XmbClock => XmbIcon::Recents,
            QuickRow::Screen => XmbIcon::Screen,
            QuickRow::Game => XmbIcon::Gameplay,
            QuickRow::Controls => XmbIcon::Controls,
            QuickRow::About => XmbIcon::About,
            QuickRow::Resume => XmbIcon::Resume,
            QuickRow::UndoSave | QuickRow::UndoLoad => XmbIcon::Undo,
            QuickRow::States => XmbIcon::States,
            QuickRow::Screenshots => XmbIcon::Screenshots,
            QuickRow::Settings => XmbIcon::Settings,
            QuickRow::ExitGame => XmbIcon::Exit,
            _ => XmbIcon::Row,
        }
    }

    /// The row's ancestor on `top`, the page shown as the column.
    fn xmb_top_of(row: QuickRow, top: &[QuickRow], in_game: bool) -> QuickRow {
        let mut at = row;
        for _ in 0..8 {
            if top.contains(&at) {
                break;
            }
            let up = match in_game {
                true => at.parent_in_game(),
                false => at.parent(),
            };
            match up {
                Some(p) => at = p,
                None => break,
            }
        }
        at
    }

    fn xmb_icon_rows(&self, column: &[QuickRow], at: usize, t: &Texts) -> Vec<XmbRow> {
        column
            .iter()
            .enumerate()
            .map(|(k, r)| XmbRow {
                pos: k as f32 - at as f32,
                face: self.xmb_art.icons[Self::xmb_row_icon(*r).index()],
                uv: [0.0, 0.0, 1.0, 1.0],
                aspect: 1.0,
                colour: [0xff; 3],
                title: t.get(XmbSize::Title, r.label()),
                halo: None,
            })
            .collect()
    }

    fn xmb_list_items(&self, rows: &[QuickRow], t: &Texts) -> Vec<XmbItem> {
        rows.iter()
            .map(|r| XmbItem {
                icon: XmbIcon::Row,
                label: t.get(XmbSize::Title, r.label()),
                value: self
                    .xmb_value_text(*r)
                    .and_then(|v| t.get(XmbSize::Small, &v)),
            })
            .collect()
    }

    fn xmb_strip(&self, row: QuickRow, t: &Texts) -> (Vec<XmbEntry>, usize) {
        let (choices, at) = self.xmb_choices(row);
        let entries = choices
            .iter()
            .map(|c| match c {
                Choice::Text(s) => XmbEntry::Text(t.get(XmbSize::Small, s)),
                Choice::Swatch(i) => XmbEntry::Swatch(slot_ui::xmb_swatch(*i)),
            })
            .collect();
        (entries, at)
    }

    /// Settings, or a game's menu: the column of `top`, and the list beside it if `row` is
    /// deeper, with its strip if one is open.
    fn xmb_menu_parts(
        &self,
        row: QuickRow,
        top: &[QuickRow],
        hidden: &[QuickRow],
        in_game: bool,
        t: &Texts,
    ) -> (Vec<XmbRow>, ListPart, StripPart) {
        let column: Vec<QuickRow> = top
            .iter()
            .copied()
            .filter(|r| !hidden.contains(r))
            .collect();
        let head = Self::xmb_top_of(row, top, in_game);
        let at = column.iter().position(|r| *r == head).unwrap_or(0);
        let rows = self.xmb_icon_rows(&column, at, t);
        if top.contains(&row) {
            return (rows, None, None);
        }
        let shown = row.shown(hidden);
        let list = (self.xmb_list_items(&shown, t), row.position(hidden));
        let picker = self.xmb_pick.map(|p| {
            let (entries, at) = self.xmb_strip(row, t);
            (entries, at, p.first)
        });
        (rows, Some(list), picker)
    }

    /// About's lines: what each is, and its value.
    pub(super) fn xmb_about_rows(&self) -> Vec<(&'static str, String)> {
        let build = crate::build_info::Build::current();
        let mut rows = vec![
            ("Version", build.version.to_string()),
            ("Serial", build.serial()),
        ];
        if let Some(model) = crate::baseos::var("BASEOS_MODEL").filter(|m| !m.is_empty()) {
            rows.push(("Model", model));
        }
        if let Some(b) = self.battery {
            rows.push(("Battery", format!("{}%", b.percent)));
        }
        rows.push(("Restart", "Press X".to_string()));
        rows
    }

    fn xmb_about_items(&self, t: &Texts) -> Vec<XmbItem> {
        self.xmb_about_rows()
            .into_iter()
            .map(|(l, v)| XmbItem {
                icon: XmbIcon::Row,
                label: t.get(XmbSize::Title, l),
                value: t.get(XmbSize::Small, &v),
            })
            .collect()
    }

    fn xmb_game_title(&self) -> String {
        self.seated()
            .and_then(|stem| self.shelf().find(stem))
            .map(|(cart, _)| slot_ui::label_text(cart))
            .unwrap_or_else(|| self.playing_platform().name().to_string())
    }

    /// The games of the shelf, placed around the animated scroll rather than the selection, so
    /// a fast scroll slides the whole column smoothly instead of leaving it behind.
    fn xmb_game_rows(&self, t: &Texts) -> Vec<XmbRow> {
        let shelf = self.shelf();
        let n = shelf.carts.len() as i32;
        let centre = shelf.index as f32 - (shelf.scroll_target() - shelf.scroll);
        let mut rows = Vec::new();
        let (from, to) = (
            centre.floor() as i32 - GAMES_ABOVE,
            centre.ceil() as i32 + GAMES_BELOW,
        );
        for i in from..=to {
            if i < 0 || i >= n {
                continue;
            }
            let i = i as usize;
            let cart = &shelf.carts[i];
            let (w, h) = slot_ui::cart_box(cart.platform);
            let (w, h) = (w as f32, h.max(1) as f32);
            let face = shelf.face(i);
            let (uv, aspect) = match (self.state.xmb_labels, face) {
                (true, Some(_)) => {
                    let b = shelf.label_box(i);
                    (b, (b[2] * w) / (b[3] * h).max(1.0))
                }
                _ => ([0.0, 0.0, 1.0, 1.0], w / h),
            };
            let title = slot_ui::label_text(cart);
            rows.push(XmbRow {
                pos: i as f32 - centre,
                face,
                uv,
                aspect,
                colour: slot_ui::label_colour(&title),
                title: t.get(XmbSize::Title, &title),
                halo: match self.state.xmb_labels && face.is_some() {
                    true => None,
                    false => Some(slot_ui::xmb_halo_index(slot_ui::gb_shell_of(cart))),
                },
            });
        }
        rows
    }

    fn xmb_cats(&self, t: &Texts) -> Vec<XmbCat> {
        let mut cats = vec![XmbCat {
            icon: XmbIcon::Settings,
            label: t.get(XmbSize::Category, "Settings"),
        }];
        cats.extend(self.xmb_shelves().into_iter().map(|at| {
            let rack = self.shelves[at].0;
            XmbCat {
                icon: rack.xmb_icon(),
                label: t.get(XmbSize::Category, rack.name()),
            }
        }));
        cats
    }

    fn xmb_option_entries(&self, o: XmbOptions, t: &Texts) -> Vec<XmbEntry> {
        let Some(cart) = self.shelf().carts.get(self.shelf().index) else {
            return Vec::new();
        };
        let mut lines = vec![
            "Play".to_string(),
            match self.state.is_favorite(cart.platform, &cart.stem) {
                true => "Remove from Favorites",
                false => "Add to Favorites",
            }
            .to_string(),
        ];
        if cart.platform == Platform::Gba {
            lines.push(format!(
                "Emulator: {}",
                match o.core {
                    Core::Gpsp => "gpSP",
                    _ => "mGBA",
                }
            ));
        }
        lines
            .iter()
            .map(|l| XmbEntry::Text(t.get(XmbSize::Small, l)))
            .collect()
    }

    fn xmb_scene(&self, t: &Texts) -> Scene {
        let clock = t.get(XmbSize::Small, &self.xmb_clock_now());
        let cats = self.xmb_cats(t);
        let at = self.xmb_target() as usize;
        let mut scene = Scene {
            cats,
            at,
            rows: Vec::new(),
            sizes: XmbSizes::ICONS,
            list: None,
            picker: None,
            options: None,
            clock_edit: None,
            clock,
            battery_text: self.xmb_battery_text(t),
        };
        match &self.phase {
            Phase::QuickMenu { row } => {
                let hidden = self.hidden_rows();
                let (rows, list, picker) =
                    self.xmb_menu_parts(*row, &QuickRow::MAIN, &hidden, false, t);
                scene.rows = rows;
                scene.list = list;
                scene.picker = picker;
            }
            Phase::SetClock { picker, .. } => {
                let hidden = self.hidden_rows();
                let row = QuickRow::XmbSetClock;
                let (rows, list, _) = self.xmb_menu_parts(row, &QuickRow::MAIN, &hidden, false, t);
                scene.rows = rows;
                scene.list = list;
                scene.clock_edit = Some(self.xmb_clock_parts(picker, t));
            }
            Phase::About => {
                let hidden = self.hidden_rows();
                let column: Vec<QuickRow> = QuickRow::MAIN
                    .iter()
                    .copied()
                    .filter(|r| !hidden.contains(r))
                    .collect();
                let at = column
                    .iter()
                    .position(|r| *r == QuickRow::About)
                    .unwrap_or(0);
                scene.rows = self.xmb_icon_rows(&column, at, t);
                let items = self.xmb_about_items(t);
                let at = self.xmb_about.min(items.len().saturating_sub(1));
                scene.list = Some((items, at));
            }
            _ => {
                scene.rows = self.xmb_game_rows(t);
                scene.sizes = XmbSizes::GAMES;
                scene.options = self
                    .xmb_options
                    .map(|o| (self.xmb_option_entries(o, t), o.at));
            }
        }
        scene
    }

    fn xmb_in_game_scene(&self, row: QuickRow, t: &Texts) -> Scene {
        let clock = t.get(XmbSize::Small, &self.xmb_clock_now());
        let hidden = self.in_game_hidden();
        let (rows, list, picker) = self.xmb_menu_parts(row, &QuickRow::PLAY, &hidden, true, t);
        let cats = vec![XmbCat {
            icon: match self.playing_platform() {
                Platform::Gba => XmbIcon::Gba,
                Platform::Gbc => XmbIcon::Gbc,
                Platform::Gb => XmbIcon::Gb,
            },
            label: t.get(XmbSize::Category, &self.xmb_game_title()),
        }];
        Scene {
            cats,
            at: 0,
            rows,
            sizes: XmbSizes::ICONS,
            list,
            picker,
            options: None,
            clock_edit: None,
            clock,
            battery_text: self.xmb_battery_text(t),
        }
    }

    fn xmb_backdrop(&self, alpha: f32) -> Option<slot_gfx::Backdrop> {
        Some(slot_gfx::Backdrop {
            curves: self.xmb_art.curves?,
            style: self.xmb_wave_style(),
            colours: slot_ui::xmb_backdrop_colours(self.xmb_colour()),
            edges: slot_ui::xmb_backdrop_edges(self.xmb_colour()),
            shape: slot_gfx::BACKDROP_PLAIN,
            alpha,
            ground: self.xmb_art.ground,
        })
    }

    fn xmb_draw_scene(
        &self,
        scene: &Scene,
        backdrop: Option<slot_gfx::Backdrop>,
        col: f32,
        alpha: f32,
        out: &mut Vec<Draw>,
    ) {
        self.xmb_draw_scene_with(scene, backdrop, col, alpha, alpha, out);
    }

    fn xmb_draw_scene_with(
        &self,
        scene: &Scene,
        backdrop: Option<slot_gfx::Backdrop>,
        col: f32,
        alpha: f32,
        status_alpha: f32,
        out: &mut Vec<Draw>,
    ) {
        let battery = self
            .battery
            .map(|b| (b.percent, b.charge != Charge::Discharging));
        Xmb {
            canvas: self.canvas(),
            art: self.xmb_art,
            time: self.xmb_time(),
            alpha,
            backdrop,
            cats: &scene.cats,
            at: scene.at,
            col,
            rows: &scene.rows,
            sizes: scene.sizes,
            nest: self.xmb_nest,
            list: scene
                .list
                .as_ref()
                .map(|(items, at)| XmbList { items, at: *at }),
            picker: scene.picker.as_ref().map(|(entries, at, first)| XmbPicker {
                entries,
                at: *at,
                first: *first,
                wide: false,
            }),
            options: scene.options.as_ref().map(|(entries, at)| XmbPicker {
                entries,
                at: *at,
                first: slot_ui::xmb_picker_centred(entries.len()),
                wide: true,
            }),
            clock_edit: scene
                .clock_edit
                .as_ref()
                .map(|(parts, at)| slot_ui::XmbClockEdit { parts, at: *at }),
            clock: scene.clock,
            battery,
            battery_text: scene.battery_text,
            status_alpha,
        }
        .draw(out);
    }

    fn xmb_textures(&self) -> Texts<'_> {
        Texts {
            have: &self.xmb_texts,
            want: RefCell::new(Vec::new()),
        }
    }

    /// The home screen: shelves, Settings, About and the option menu.
    pub(super) fn draw_xmb(&self, alpha: f32, out: &mut Vec<Draw>) {
        let wallpapered = self.xmb_wallpapered();
        if wallpapered {
            self.draw_background(out);
        }
        let t = self.xmb_textures();
        let scene = self.xmb_scene(&t);
        let backdrop = if wallpapered {
            None
        } else {
            self.xmb_backdrop(1.0)
        };
        self.xmb_draw_scene(&scene, backdrop, self.xmb_col, alpha, out);
    }

    /// The menu over a paused game: the bar's background laid over the picture, mostly opaque.
    pub(super) fn draw_xmb_in_game(&self, row: QuickRow, out: &mut Vec<Draw>) {
        let t = self.xmb_textures();
        let scene = self.xmb_in_game_scene(row, &t);
        // The game shows only faintly through the bar, which goes dark while a game is paused
        // whatever its colour, as the PS3's does.
        let dark = self.state.xmb_pause_dark;
        let backdrop = self.xmb_backdrop(0.96).map(|b| match dark {
            true => slot_gfx::Backdrop {
                colours: slot_ui::xmb_pause_colours(),
                edges: slot_ui::xmb_pause_edges(),
                ground: None,
                ..b
            },
            false => b,
        });
        self.xmb_draw_scene(&scene, backdrop, 0.0, 1.0, out);
    }

    pub(super) fn xmb_black(&self, alpha: f32, out: &mut Vec<Draw>) {
        if alpha <= 0.0 {
            return;
        }
        let canvas = self.canvas();
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: canvas.w,
            h: canvas.h,
            colour: [0.0, 0.0, 0.0, alpha.min(1.0)],
        });
    }

    /// Steps the bar one category, without wrapping: left of the first shelf is Settings.
    pub(super) fn xmb_step(&mut self, by: i32) {
        let at = self.xmb_at() as i32 + by;
        if at < 0 {
            return self.open_quick_menu();
        }
        if (at as usize) < self.xmb_shelves().len() {
            self.switch_shelf(by);
        }
    }

    pub(super) fn xmb_leave_settings(&mut self) {
        self.xmb_pick = None;
        if self.shelf().carts.is_empty() {
            if let Some(at) = self.xmb_shelves().first() {
                self.shelf_at = *at;
            }
        }
        self.phase = Phase::Shelf;
    }

    fn xmb_open_pick(&mut self, row: QuickRow) {
        let (_, at) = self.xmb_choices(row);
        self.xmb_pick = Some(XmbPick {
            first: slot_ui::xmb_picker_open(at, self.canvas().h),
            moved: 0,
        });
    }

    /// Up and down step the value live, as the handheld previews a colour the moment it is
    /// highlighted; A keeps it, B or Left puts back what was there.
    fn xmb_pick_input(&mut self, row: QuickRow, mut pick: XmbPick, action: Action) -> bool {
        // The strip stops at its ends rather than wrapping round, as the handheld's does; a
        // value that cannot be listed ahead (one entry) still steps.
        let (list, at) = self.xmb_choices(row);
        let listed = list.len() > 1;
        match action {
            Action::GbaDown(Btn::Up) if listed && at == 0 => return true,
            Action::GbaDown(Btn::Down) if listed && at + 1 >= list.len() => return true,
            Action::GbaDown(Btn::Up) => {
                self.change_setting(row, false);
                pick.moved -= 1;
            }
            Action::GbaDown(Btn::Down) => {
                self.change_setting(row, true);
                pick.moved += 1;
            }
            Action::GbaDown(Btn::A | Btn::Right) => {
                self.xmb_pick = None;
                return true;
            }
            Action::GbaDown(Btn::B | Btn::Left) => {
                let back = pick.moved < 0;
                for _ in 0..pick.moved.unsigned_abs() {
                    self.change_setting(row, back);
                }
                self.xmb_pick = None;
                return true;
            }
            Action::QuickMenu => {
                self.xmb_pick = None;
                return false;
            }
            _ => return true,
        }
        let (_, at) = self.xmb_choices(row);
        pick.first = slot_ui::xmb_picker_first(pick.first, at, self.canvas().h);
        self.xmb_pick = Some(pick);
        true
    }

    pub(super) fn xmb_quick_input(&mut self, row: QuickRow, action: Action) {
        if let Some(pick) = self.xmb_pick {
            if self.xmb_pick_input(row, pick, action) {
                return;
            }
        }
        if action == Action::QuickMenu {
            return self.xmb_leave_settings();
        }
        if row.binds() {
            match action {
                Action::GbaDown(Btn::A) => return self.start_capture(row),
                Action::GbaDown(Btn::X) => return self.set_binding(row, None),
                Action::GbaDown(Btn::Y) if !self.page_is_default(row) => {
                    self.reset_prompt = Some(row);
                    return;
                }
                _ => {}
            }
        }
        let top = QuickRow::MAIN.contains(&row);
        let hidden = self.hidden_rows();
        match action {
            Action::GbaDown(Btn::Up) => {
                self.phase = Phase::QuickMenu {
                    row: row.up(&hidden),
                }
            }
            Action::GbaDown(Btn::Down) => {
                self.phase = Phase::QuickMenu {
                    row: row.down(&hidden),
                }
            }
            // Settings is the leftmost category: right from its column moves on to the games.
            Action::GbaDown(Btn::Right) if top => self.xmb_leave_settings(),
            Action::GbaDown(Btn::A | Btn::Right) => {
                if self.xmb_editable(row) {
                    self.xmb_open_pick(row);
                } else {
                    self.open_quick_row(row);
                }
            }
            Action::GbaDown(Btn::B | Btn::Left) => {
                if let Some(parent) = row.parent() {
                    self.phase = Phase::QuickMenu { row: parent };
                }
            }
            _ => {}
        }
    }

    pub(super) fn xmb_in_game_input(&mut self, row: QuickRow, action: Action) {
        if let Some(pick) = self.xmb_pick {
            if self.xmb_pick_input(row, pick, action) {
                return;
            }
            return self.tap_in_game_menu(None);
        }
        if row.binds() {
            match action {
                Action::GbaDown(Btn::A) => return self.start_capture(row),
                Action::GbaDown(Btn::X) => return self.set_binding(row, None),
                Action::GbaDown(Btn::Y) if !self.page_is_default(row) => {
                    self.reset_prompt = Some(row);
                    return;
                }
                _ => {}
            }
        }
        let hidden = self.in_game_hidden();
        match action {
            Action::GbaDown(Btn::Up) => self.in_game_menu = Some(row.up(&hidden)),
            Action::GbaDown(Btn::Down) => self.in_game_menu = Some(row.down(&hidden)),
            // Right steps into lists and values; it never fires an action like Exit Game.
            Action::GbaDown(Btn::Right) if row.child().is_none() && !self.xmb_editable(row) => {}
            Action::GbaDown(Btn::A | Btn::Right) => {
                if self.xmb_editable(row) {
                    self.xmb_open_pick(row);
                } else {
                    self.open_in_game_row(row);
                }
            }
            Action::GbaDown(Btn::B | Btn::Left) => match row.parent_in_game() {
                Some(parent) => self.in_game_menu = Some(parent),
                None if action == Action::GbaDown(Btn::B) => self.tap_in_game_menu(None),
                None => {}
            },
            Action::QuickMenu => self.tap_in_game_menu(None),
            Action::Polaroids => self.double_tap_states(),
            _ => {}
        }
    }

    pub(super) fn xmb_open_options(&mut self) {
        let Some(cart) = self.shelf().carts.get(self.shelf().index) else {
            return;
        };
        let core = match (&self.root, cart.platform) {
            (Some(root), Platform::Gba) => slot_store::core_for(root, &cart.stem),
            _ => Core::default(),
        };
        self.shelf_mut().release_hold();
        self.play_held = None;
        self.xmb_options = Some(XmbOptions { at: 0, core });
    }

    pub(super) fn xmb_options_input(&mut self, action: Action) {
        let Some(mut o) = self.xmb_options else {
            return;
        };
        let gba = self
            .shelf()
            .carts
            .get(self.shelf().index)
            .is_some_and(|c| c.platform == Platform::Gba);
        let n = if gba { 3 } else { 2 };
        match action {
            Action::GbaDown(Btn::Up) => o.at = (o.at + n - 1) % n,
            Action::GbaDown(Btn::Down) => o.at = (o.at + 1) % n,
            Action::GbaDown(Btn::B | Btn::Start) | Action::QuickMenu => {
                self.xmb_options = None;
                return;
            }
            Action::GbaDown(Btn::A | Btn::Left | Btn::Right) => match o.at {
                0 if action == Action::GbaDown(Btn::A) => {
                    self.xmb_options = None;
                    return self.insert(!self.state.auto_load);
                }
                1 if action == Action::GbaDown(Btn::A) => self.toggle_favorite(),
                2 => {
                    o.core = match o.core {
                        Core::Gpsp => Core::Mgba,
                        _ => Core::Gpsp,
                    };
                    self.write_core(o.core);
                }
                _ => {}
            },
            _ => {}
        }
        self.xmb_options = Some(o);
    }
}

/// A zone as an offset from UTC: "UTC+9", "UTC-3:30".
fn zone_text(offset_min: i16) -> String {
    let sign = if offset_min < 0 { '-' } else { '+' };
    let off = offset_min.unsigned_abs();
    match off % 60 {
        0 => format!("UTC{sign}{}", off / 60),
        m => format!("UTC{sign}{}:{m:02}", off / 60),
    }
}

fn on_off(on: bool) -> String {
    if on { "On" } else { "Off" }.to_string()
}

fn ramp(x: f32, from: f32, span: f32) -> f32 {
    ((x - from) / span).clamp(0.0, 1.0)
}

fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

impl App {
    pub(super) fn shutdown_show_ms(&self) -> Millis {
        match XMB {
            true => (slot_ui::XMB_OUTRO_SECS * 1000.0) as Millis,
            false => 250,
        }
    }

    /// The device draws the start of the animation itself while this app is still being
    /// loaded; `played` is how far it got, so the animation carries on from there.
    pub fn xmb_intro_played(&mut self, played_ms: Millis) {
        if !XMB {
            return;
        }
        if !matches!(self.phase, Phase::Shelf) {
            self.xmb_intro_done = true;
            return;
        }
        self.xmb_intro = Some(self.now() as f64 - played_ms as f64);
        self.xmb_handover = played_ms as f32 / 1000.0;
        self.xmb_time_shift = played_ms as f64 - self.now() as f64;
    }

    /// Starts the animation when the bar first comes up, unless the device already began it;
    /// a start that goes straight into a game, or to setting the clock, has none.
    pub(super) fn xmb_intro_tick(&mut self) {
        if !XMB || self.xmb_intro_done || self.xmb_intro.is_some() {
            return;
        }
        match self.phase {
            Phase::Shelf => {
                self.xmb_intro = Some(self.now() as f64);
                self.xmb_time_shift = -(self.now() as f64);
                self.xmb_sound = Some(crate::audio::XmbSound::Opening);
            }
            _ => self.xmb_intro_done = true,
        }
    }

    pub(super) fn xmb_intro_secs(&self) -> Option<f32> {
        if self.xmb_intro_done {
            return None;
        }
        self.xmb_intro
            .map(|at| ((self.now() as f64 - at) / 1000.0) as f32)
    }

    /// Whether the bar is still hidden by the start-up animation, so nothing need wait for it.
    pub fn xmb_holds_faces(&self) -> bool {
        self.xmb_intro_secs()
            .is_some_and(|s| s < slot_ui::XMB_INTRO_BAR - 0.3)
    }

    pub(super) fn xmb_intro_active(&self) -> bool {
        self.xmb_intro_secs()
            .is_some_and(|s| s < slot_ui::XMB_INTRO_END)
    }

    /// Seconds since the waves' fast start-up run began; it stays given once there was a
    /// start-up, even a skipped one, so the waves never jump back.
    pub fn xmb_intro_waves(&self) -> Option<f32> {
        let at = self.xmb_intro?;
        Some(((self.now() as f64 - at) / 1000.0) as f32 - slot_ui::XMB_INTRO_WAVES)
    }

    /// Ends the start-up animation where it is, eg for a test that wants the bar at once.
    pub fn skip_xmb_intro(&mut self) {
        self.xmb_intro_done = true;
    }

    /// A button press during the animation ends it.
    pub(super) fn xmb_intro_skip(&mut self, action: Action) -> bool {
        if !self.xmb_intro_active() {
            return false;
        }
        if matches!(action, Action::GbaDown(_) | Action::QuickMenu) {
            self.xmb_intro_done = true;
            return true;
        }
        false
    }

    pub fn take_xmb_sound(&mut self) -> Option<crate::audio::XmbSound> {
        self.xmb_sound.take()
    }

    /// Everything a button can change on the bar, to tell whether a press did anything.
    pub(super) fn xmb_signature(&self) -> String {
        let shelf = self.shelf();
        format!(
            "{:?}|{:?}|{}|{}|{:?}|{:?}|{:?}|{}|{:?}|{}",
            std::mem::discriminant(&self.phase),
            self.quick_menu(),
            self.shelf_at,
            shelf.index,
            self.xmb_pick,
            self.xmb_options,
            self.in_game_menu,
            self.xmb_about,
            self.picker().map(|p| (p.text(), p.cursor())),
            self.state.xmb_colour,
        )
    }

    /// The handheld's sounds: a tick for moving, a chime for choosing, a softer one for
    /// backing out. Only for presses that changed something.
    pub(super) fn xmb_sound_for(&mut self, action: Action, before: String) {
        use crate::audio::XmbSound as S;
        if self.xmb_sound.is_some() || self.xmb_signature() == before {
            return;
        }
        let in_game = matches!(self.phase, Phase::Playing { .. });
        let sound = match action {
            // A game starts on the release of A, which counts as choosing it.
            Action::GbaDown(Btn::A) | Action::GbaUp(Btn::A) | Action::Insert => S::Confirm,
            Action::GbaDown(Btn::Start) => S::Confirm,
            Action::GbaDown(Btn::B) => S::Cancel,
            Action::QuickMenu if in_game || self.in_game_menu.is_some() => S::Confirm,
            Action::QuickMenu => S::Cursor,
            Action::GbaDown(Btn::Up | Btn::Down | Btn::Left | Btn::Right)
            | Action::ShelfLeft
            | Action::ShelfRight
            | Action::PreviousShelf
            | Action::NextShelf => S::Cursor,
            _ => return,
        };
        // In a game the buttons are the game's, unless its menu is open.
        if in_game && self.in_game_menu.is_none() && !matches!(action, Action::QuickMenu) {
            return;
        }
        self.xmb_sound = Some(sound);
    }

    pub fn xmb_theme_rgb(&self) -> [u8; 3] {
        slot_ui::xmb_swatch(self.xmb_colour())
    }

    fn xmb_intro_art(&self, t: f32) -> slot_ui::XmbIntro {
        slot_ui::XmbIntro {
            canvas: self.canvas(),
            t,
            backdrop: self.xmb_backdrop(1.0),
            flare: self.xmb_art.flare,
            glint: self.xmb_art.glint,
            theme: self.xmb_theme_rgb().map(|v| v as f32 / 255.0),
            title: self.xmb_art.title,
            foot: self.xmb_art.foot,
        }
    }

    pub(super) fn xmb_draw_intro(&self, s: f32, out: &mut Vec<Draw>) {
        self.xmb_intro_art(s).draw(out);
        // The clock, then the bar, fade in once the animation and the loading are both done.
        let from = |at: f32| at.max(self.xmb_handover);
        let status = ease(ramp(s, from(slot_ui::XMB_INTRO_CLOCK), 0.4));
        let bar = ease(ramp(s, from(slot_ui::XMB_INTRO_BAR), 0.5));
        let t = self.xmb_textures();
        let scene = self.xmb_scene(&t);
        self.xmb_draw_scene_with(&scene, None, self.xmb_col, bar, status, out);
    }

    pub(super) fn xmb_draw_outro(&self, out: &mut Vec<Draw>) {
        let left = self.act_at.saturating_sub(self.now()) as f32 / 1000.0;
        let p = 1.0 - (left / slot_ui::XMB_OUTRO_SECS).clamp(0.0, 1.0);
        self.xmb_intro_art(slot_ui::xmb_outro_t(p)).draw(out);
        let ui = 1.0 - ease(ramp(p, 0.0, 0.15));
        if ui > 0.0 && !matches!(self.phase, Phase::Playing { .. }) {
            let t = self.xmb_textures();
            let scene = self.xmb_scene(&t);
            self.xmb_draw_scene_with(&scene, None, self.xmb_col, ui, ui, out);
        }
        // The last of the band goes out.
        let dark = ease(ramp(p, 0.72, 0.28));
        if dark > 0.0 {
            let c = self.canvas();
            out.push(Draw::Rect {
                x: 0.0,
                y: 0.0,
                w: c.w,
                h: c.h,
                colour: [0.0, 0.0, 0.0, dark],
            });
        }
    }
}
