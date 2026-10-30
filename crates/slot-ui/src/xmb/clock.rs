//! The bar's clock editor: the date and the time as fields read along a line, as the handheld
//! sets its clock, in the city the clock keeps (`slot_store::Zone`, which minds summer time).

use slot_store::{civil_from_days, days_from_civil, days_in_month, Zone};

const DAY: i64 = 86_400;
const YEAR_MIN: i64 = 2000;
const YEAR_MAX: i64 = 2099;

/// A part of the date or the time.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum XmbClockField {
    Year,
    Month,
    Day,
    Hour,
    Minute,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct XmbClockPicker {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    zone: Zone,
    cursor: XmbClockField,
}

fn wrap(v: i64, lo: i64, hi: i64) -> i64 {
    lo + (v - lo).rem_euclid(hi - lo + 1)
}

impl XmbClockPicker {
    /// The clock at `utc`, as it reads in `zone`.
    pub fn new(utc: i64, zone: Zone) -> Self {
        let local = zone.local(utc);
        let (year, month, day) = civil_from_days(local.div_euclid(DAY));
        let rem = local.rem_euclid(DAY);
        XmbClockPicker {
            year: year.clamp(YEAR_MIN, YEAR_MAX),
            month,
            day,
            hour: rem / 3600,
            minute: rem / 60 % 60,
            zone,
            cursor: XmbClockField::Month,
        }
    }

    /// The moment the fields read, in UTC.
    pub fn secs(&self) -> i64 {
        let local = days_from_civil(self.year, self.month, self.day) * DAY + self.hour * 3600 + self.minute * 60;
        self.zone.utc(local)
    }

    pub fn year(&self) -> i64 {
        self.year
    }

    pub fn month(&self) -> i64 {
        self.month
    }

    pub fn day(&self) -> i64 {
        self.day
    }

    pub fn hour(&self) -> i64 {
        self.hour
    }

    pub fn minute(&self) -> i64 {
        self.minute
    }

    pub fn cursor(&self) -> XmbClockField {
        self.cursor
    }

    pub fn field(&mut self, field: XmbClockField) {
        self.cursor = field;
    }

    pub fn up(&mut self) {
        self.step(1);
    }

    pub fn down(&mut self) {
        self.step(-1);
    }

    fn step(&mut self, by: i64) {
        match self.cursor {
            XmbClockField::Year => self.year = wrap(self.year + by, YEAR_MIN, YEAR_MAX),
            XmbClockField::Month => self.month = wrap(self.month + by, 1, 12),
            XmbClockField::Day => self.day = wrap(self.day + by, 1, days_in_month(self.year, self.month)),
            XmbClockField::Hour => self.hour = wrap(self.hour + by, 0, 23),
            XmbClockField::Minute => self.minute = wrap(self.minute + by, 0, 59),
        }
        self.day = self.day.min(days_in_month(self.year, self.month));
    }
}
