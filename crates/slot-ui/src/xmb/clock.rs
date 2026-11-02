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
    Second,
    /// Morning or afternoon, on a 12-hour clock: either way moves the hour by twelve.
    Meridiem,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct XmbClockPicker {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    /// The seconds, once they have been set: until then they run on with the clock.
    second: Option<i64>,
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
            second: None,
            zone,
            cursor: XmbClockField::Month,
        }
    }

    /// The moment the fields read, in UTC: at the start of the minute until the seconds are set.
    pub fn secs(&self) -> i64 {
        let local = days_from_civil(self.year, self.month, self.day) * DAY
            + self.hour * 3600
            + self.minute * 60
            + self.second.unwrap_or(0);
        self.zone.utc(local)
    }

    /// The seconds, if they have been set.
    pub fn second(&self) -> Option<i64> {
        self.second
    }

    /// Takes the seconds the clock reads now, `live`, as where the seconds start from, unless
    /// they have been set already: called before they are first stepped.
    pub fn hold_second(&mut self, live: i64) {
        self.second.get_or_insert(live.rem_euclid(60));
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
            XmbClockField::Second => self.second = Some(wrap(self.second.unwrap_or(0) + by, 0, 59)),
            XmbClockField::Meridiem => self.hour = (self.hour + 12) % 24,
        }
        self.day = self.day.min(days_in_month(self.year, self.month));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seconds_and_the_half_of_the_day_can_be_set() {
        // 2026-11-02 09:30:42 UTC.
        let utc = 1_793_611_842;
        let mut p = XmbClockPicker::new(utc, Zone::default());
        assert_eq!((p.hour(), p.minute(), p.second()), (9, 30, None));
        assert_eq!(p.secs(), utc - 42);
        p.field(XmbClockField::Meridiem);
        p.up();
        assert_eq!(p.hour(), 21);
        p.down();
        assert_eq!(p.hour(), 9);
        p.field(XmbClockField::Second);
        p.hold_second(42);
        p.up();
        assert_eq!(p.second(), Some(43));
        assert_eq!(p.secs(), utc + 1);
        p.hold_second(10);
        assert_eq!(p.second(), Some(43), "seconds once set stay set");
        for _ in 0..20 {
            p.up();
        }
        assert_eq!(p.second(), Some(3));
    }
}
