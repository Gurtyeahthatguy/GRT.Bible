//! The Roman calendar with the proper of Italy, and the readings of each day's Mass.
//!
//! The dates are computed here; the saints and the readings come from `data/liturgy.json`.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const DATA: &str = include_str!("../data/liturgy.json");

#[derive(Deserialize)]
struct Data {
    temporale: BTreeMap<String, Proper>,
    sanctorale: Vec<Saint>,
    readings: BTreeMap<String, Vec<MassData>>,
}

#[derive(Deserialize)]
struct Proper {
    grade: u8,
    color: String,
    en: String,
    it: String,
}

#[derive(Deserialize)]
struct Saint {
    key: String,
    month: u32,
    day: u32,
    grade: u8,
    color: String,
    en: String,
    it: String,
    #[serde(default)]
    readings: Option<String>,
}

#[derive(Deserialize)]
struct MassData {
    id: String,
    #[serde(default)]
    label: String,
    readings: Vec<ReadingData>,
}

#[derive(Deserialize)]
struct ReadingData {
    slot: String,
    #[serde(default)]
    cycle: Option<String>,
    #[serde(rename = "ref")]
    reference: String,
    osis: Vec<String>,
    #[serde(default, rename = "or")]
    alternatives: Vec<AlternativeData>,
}

#[derive(Deserialize)]
struct AlternativeData {
    #[serde(rename = "ref")]
    reference: String,
    osis: Vec<String>,
}

fn data() -> &'static Data {
    static DATA_CELL: OnceLock<Data> = OnceLock::new();
    DATA_CELL.get_or_init(|| serde_json::from_str(DATA).expect("liturgy.json is valid"))
}

/// Days since 1970-01-01 of a civil date.
pub fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The civil date of a count of days since 1970-01-01.
pub fn civil_from_days(n: i64) -> (i32, u32, u32) {
    let z = n + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = (yoe + era * 400) as i32 + i32::from(month <= 2);
    (year, month, day)
}

/// 0 for Sunday, 6 for Saturday.
pub fn weekday(n: i64) -> u32 {
    (n + 4).rem_euclid(7) as u32
}

/// Easter Sunday of a Gregorian year.
pub fn easter(year: i32) -> i64 {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    days_from_civil(year, month as u32, day as u32)
}

/// The first Sunday of Advent that begins the year ending in `year + 1`.
fn advent(year: i32) -> i64 {
    let christmas = days_from_civil(year, 12, 25);
    let back = match weekday(christmas) {
        0 => 7,
        w => w as i64,
    };
    christmas - back - 21
}

/// The dates that move with Easter and with Christmas in one liturgical year.
struct Year {
    /// The civil year in which this liturgical year ends.
    civil: i32,
    advent_before: i64,
    advent_after: i64,
    baptism: i64,
    easter: i64,
}

impl Year {
    fn of(day: i64) -> Year {
        let (y, _, _) = civil_from_days(day);
        let civil = if day >= advent(y) { y + 1 } else { y };
        let epiphany = days_from_civil(civil, 1, 6);
        // In Italy the Epiphany keeps 6 January, and the Baptism is the Sunday after it.
        let baptism = epiphany + (7 - weekday(epiphany) as i64);
        Year { civil, advent_before: advent(civil - 1), advent_after: advent(civil), baptism, easter: easter(civil) }
    }

    fn sunday_cycle(&self) -> &'static str {
        ["C", "A", "B"][self.civil.rem_euclid(3) as usize]
    }

    fn weekday_cycle(&self) -> &'static str {
        if self.civil % 2 == 1 {
            "I"
        } else {
            "II"
        }
    }
}

/// Precedence in the Table of Liturgical Days: the smaller wins.
mod rank {
    pub const TRIDUUM: u8 = 1;
    pub const PRINCIPAL: u8 = 2;
    pub const SOLEMNITY: u8 = 3;
    pub const LORD: u8 = 5;
    pub const SUNDAY: u8 = 6;
    pub const FEAST: u8 = 7;
    pub const PRIVILEGED: u8 = 9;
    pub const MEMORIAL: u8 = 10;
    pub const OPTIONAL: u8 = 12;
    pub const WEEKDAY: u8 = 13;
}

fn rank_of_grade(grade: u8) -> u8 {
    match grade {
        7 => rank::PRINCIPAL,
        6 => rank::SOLEMNITY,
        5 => rank::LORD,
        4 => rank::FEAST,
        3 => rank::MEMORIAL,
        _ => rank::OPTIONAL,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reading {
    pub slot: String,
    #[serde(rename = "ref")]
    pub reference: String,
    pub osis: Vec<String>,
    pub alternatives: Vec<Alternative>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Alternative {
    #[serde(rename = "ref")]
    pub reference: String,
    pub osis: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Mass {
    pub id: String,
    pub label: String,
    pub readings: Vec<Reading>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Celebration {
    pub key: String,
    pub en: String,
    pub it: String,
    /// solemnity, feast, memorial, optional or weekday.
    pub grade: String,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    pub date: String,
    /// advent, christmas, lent, triduum, easter or ordinary.
    pub season: String,
    pub week: u32,
    pub sunday_cycle: String,
    pub weekday_cycle: String,
    pub color: String,
    pub celebration: Celebration,
    /// Optional memorials, and memorials that a privileged weekday turns into commemorations.
    pub optional: Vec<Celebration>,
    pub masses: Vec<Mass>,
    /// Masses of an optional memorial with readings of its own.
    pub optional_masses: Vec<Mass>,
}

fn grade_name(grade: u8) -> &'static str {
    match grade {
        6 | 7 => "solemnity",
        4 | 5 => "feast",
        3 => "memorial",
        2 => "optional",
        _ => "weekday",
    }
}

const WEEKDAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];
const WEEKDAY_NAMES: [&str; 7] = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];

/// The day of the proper of time, before the saints are weighed against it.
struct Temporal {
    season: &'static str,
    week: u32,
    rank: u8,
    color: String,
    /// The key in the proper of time for names, and in the lectionary for readings.
    proper: Option<String>,
    readings: String,
    en: String,
    it: String,
    grade: u8,
}

fn ordinal_en(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

fn roman(n: u32) -> String {
    let table = [(10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")];
    let mut n = n;
    let mut out = String::new();
    for (value, letters) in table {
        while n >= value {
            out.push_str(letters);
            n -= value;
        }
    }
    out
}

const IT_WEEKDAYS: [&str; 7] = ["Domenica", "Lunedì", "Martedì", "Mercoledì", "Giovedì", "Venerdì", "Sabato"];
const EN_WEEKDAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

fn weekday_of(season_en: &str, season_it: &str, week: u32, wd: u32) -> (String, String) {
    (
        format!("{} of the {} Week of {season_en}", EN_WEEKDAYS[wd as usize], ordinal_en(week)),
        format!("{} della {} settimana {season_it}", IT_WEEKDAYS[wd as usize], roman(week)),
    )
}

fn temporal(day: i64, year: &Year) -> Temporal {
    let d = data();
    let wd = weekday(day);
    let (_, month, date) = civil_from_days(day);
    let e = year.easter;
    let proper = |key: &str, rank: u8, readings: &str, season: &'static str, week: u32| {
        let p = &d.temporale[key];
        Temporal { season, week, rank, color: p.color.clone(), proper: Some(key.into()), readings: readings.into(), en: p.en.clone(), it: p.it.clone(), grade: p.grade }
    };
    let weekday_day = |season: &'static str, week: u32, rank: u8, color: &str, readings: String, names: (String, String)| Temporal {
        season,
        week,
        rank,
        color: color.into(),
        proper: None,
        readings,
        en: names.0,
        it: names.1,
        grade: 0,
    };
    // Advent.
    if day >= year.advent_before && day < days_from_civil(year.civil - 1, 12, 25) {
        let week = ((day - year.advent_before) / 7 + 1) as u32;
        if wd == 0 {
            return proper(&format!("Advent{week}"), rank::PRINCIPAL, &format!("advent-sunday-{week}"), "advent", week);
        }
        if month == 12 && date >= 17 {
            let names = (format!("{date} December"), format!("{date} dicembre"));
            return weekday_day("advent", week, rank::PRIVILEGED, "purple", format!("advent-december-{date}"), names);
        }
        return weekday_day("advent", week, rank::WEEKDAY, "purple", format!("advent-weekday-{week}-{}", WEEKDAYS[wd as usize]), weekday_of("Advent", "di Avvento", week, wd));
    }
    // Christmas time.
    if day <= year.baptism || (month == 12 && date >= 25) {
        let christmas = days_from_civil(year.civil - 1, 12, 25);
        if day == christmas {
            return proper("Christmas", rank::PRINCIPAL, "nativity-of-the-lord", "christmas", 1);
        }
        if day == holy_family(year) {
            return proper("HolyFamily", rank::LORD, "holy-family-of-jesus-mary-and-joseph", "christmas", 1);
        }
        if month == 12 {
            let names = (format!("{date} December, in the Octave of Christmas"), format!("{date} dicembre, ottava di Natale"));
            let readings = if date >= 29 { format!("christmas-octave-day-{}", date - 24) } else { String::new() };
            return weekday_day("christmas", 1, rank::PRIVILEGED, "white", readings, names);
        }
        if date == 1 {
            return proper("MaryMotherOfGod", rank::SOLEMNITY, "mary-mother-of-god", "christmas", 1);
        }
        if date == 6 {
            return proper("Epiphany", rank::PRINCIPAL, "epiphany-of-the-lord", "christmas", 2);
        }
        if day == year.baptism {
            return proper("BaptismLord", rank::LORD, "baptism-of-the-lord", "christmas", 2);
        }
        if wd == 0 {
            return proper("Christmas2", rank::SUNDAY, "second-sunday-after-christmas", "christmas", 2);
        }
        if date < 6 {
            let names = (format!("{date} January, Christmas Time"), format!("{date} gennaio, tempo di Natale"));
            return weekday_day("christmas", 2, rank::WEEKDAY, "white", format!("christmas-time-january-{date}"), names);
        }
        // With the Epiphany on 6 January, the days after it read by date: the 7th as Monday, the 12th as Saturday.
        let names = (format!("{date} January, Christmas Time"), format!("{date} gennaio, tempo di Natale"));
        let by_date = WEEKDAY_NAMES[((date - 6) % 7) as usize];
        return weekday_day("christmas", 2, rank::WEEKDAY, "white", format!("{by_date}-after-epiphany"), names);
    }
    let ash = e - 46;
    // Ordinary Time after the Baptism.
    if day < ash {
        let week = ((day - year.baptism) / 7 + 1) as u32;
        return ordinary(day, week);
    }
    // Lent and Holy Week.
    if day < e - 3 {
        if day == ash {
            return proper("AshWednesday", rank::PRINCIPAL, "lent-weekday-0-wed", "lent", 0);
        }
        let week = ((day - (ash + 4)) / 7 + 1) as u32;
        let week = if day < ash + 4 { 0 } else { week };
        if wd == 0 {
            if week == 6 {
                return proper("PalmSun", rank::PRINCIPAL, "palm-sunday-of-the-passion-of-the-lord", "lent", 6);
            }
            return proper(&format!("Lent{week}"), rank::PRINCIPAL, &format!("lent-sunday-{week}"), "lent", week);
        }
        if week == 6 {
            let key = ["", "MonHolyWeek", "TueHolyWeek", "WedHolyWeek"][wd as usize];
            return proper(key, rank::PRINCIPAL, &format!("lent-weekday-6-{}", WEEKDAYS[wd as usize]), "lent", 6);
        }
        let names = if week == 0 {
            (format!("{} after Ash Wednesday", EN_WEEKDAYS[wd as usize]), format!("{} dopo le Ceneri", IT_WEEKDAYS[wd as usize]))
        } else {
            weekday_of("Lent", "di Quaresima", week, wd)
        };
        return weekday_day("lent", week, rank::PRIVILEGED, "purple", format!("lent-weekday-{week}-{}", WEEKDAYS[wd as usize]), names);
    }
    // The Triduum.
    if day < e {
        let (key, readings) = match e - day {
            3 => ("HolyThurs", "thursday-of-the-lords-supper"),
            2 => ("GoodFri", "friday-of-the-passion-of-the-lord"),
            _ => ("EasterVigil", "easter-sunday"),
        };
        return proper(key, rank::TRIDUUM, readings, "triduum", 0);
    }
    // Easter time.
    if day < e + 50 {
        if day == e {
            return proper("Easter", rank::TRIDUUM, "easter-sunday", "easter", 1);
        }
        if day < e + 7 {
            let key = ["", "MonOctaveEaster", "TueOctaveEaster", "WedOctaveEaster", "ThuOctaveEaster", "FriOctaveEaster", "SatOctaveEaster"][wd as usize];
            return proper(key, rank::PRINCIPAL, &format!("easter-{}", WEEKDAY_NAMES[wd as usize]), "easter", 1);
        }
        let week = ((day - e) / 7 + 1) as u32;
        if day == e + 42 {
            return proper("Ascension", rank::PRINCIPAL, "ascension-of-the-lord", "easter", 7);
        }
        if day == e + 49 {
            return proper("Pentecost", rank::PRINCIPAL, "pentecost-sunday", "easter", 8);
        }
        if wd == 0 {
            return proper(&format!("Easter{week}"), rank::PRINCIPAL, &format!("easter-sunday-{week}"), "easter", week);
        }
        return weekday_day("easter", week, rank::WEEKDAY, "white", format!("easter-weekday-{week}-{}", WEEKDAYS[wd as usize]), weekday_of("Easter", "di Pasqua", week, wd));
    }
    // Ordinary Time after Pentecost, counted back from Christ the King, the 34th Sunday.
    let king = year.advent_after - 7;
    let sunday = day - wd as i64;
    let week = (34 - (king - sunday) / 7) as u32;
    if day == e + 56 {
        return proper("Trinity", rank::SOLEMNITY, "most-holy-trinity", "ordinary", week);
    }
    if day == e + 63 {
        return proper("CorpusChristi", rank::SOLEMNITY, "most-holy-body-and-blood-of-christ", "ordinary", week);
    }
    if day == e + 68 {
        return proper("SacredHeart", rank::SOLEMNITY, "most-sacred-heart-of-jesus", "ordinary", week);
    }
    if day == king {
        return proper("ChristKing", rank::SOLEMNITY, "our-lord-jesus-christ-king-of-the-universe", "ordinary", 34);
    }
    ordinary(day, week)
}

fn ordinary(day: i64, week: u32) -> Temporal {
    let wd = weekday(day);
    if wd == 0 {
        let p = &data().temporale[&format!("OrdSunday{week}")];
        return Temporal {
            season: "ordinary",
            week,
            rank: rank::SUNDAY,
            color: p.color.clone(),
            proper: Some(format!("OrdSunday{week}")),
            readings: format!("ot-sunday-{week}"),
            en: p.en.clone(),
            it: p.it.clone(),
            grade: p.grade,
        };
    }
    let (en, it) = weekday_of("Ordinary Time", "del Tempo Ordinario", week, wd);
    Temporal {
        season: "ordinary",
        week,
        rank: rank::WEEKDAY,
        color: "green".into(),
        proper: None,
        readings: format!("ot-weekday-{week}-{}", WEEKDAYS[wd as usize]),
        en,
        it,
        grade: 0,
    }
}

/// The Sunday within the octave of Christmas, or 30 December when Christmas is a Sunday.
fn holy_family(year: &Year) -> i64 {
    let christmas = days_from_civil(year.civil - 1, 12, 25);
    match weekday(christmas) {
        0 => christmas + 5,
        w => christmas + (7 - w as i64),
    }
}

/// Where a solemnity of the saints falls this year, after any transfer.
fn solemnity_date(saint: &Saint, civil: i32) -> i64 {
    let natural = days_from_civil(civil, saint.month, saint.day);
    if saint.grade < 6 {
        return natural;
    }
    let e = easter(civil);
    let palm = e - 7;
    let is_sunday = weekday(natural) == 0;
    match saint.key.as_str() {
        "StJoseph" if natural >= palm && natural < e => palm - 1,
        "Annunciation" if natural >= palm && natural <= e + 7 => e + 8,
        "StJoseph" | "Annunciation" if is_sunday && natural < palm => natural + 1,
        "ImmaculateConception" if is_sunday => natural + 1,
        "NativityJohnBaptist" if natural == e + 68 => natural - 1,
        "NativityJohnBaptist" | "StsPeterPaulAp" if natural == e + 56 || natural == e + 63 || natural == e + 68 => natural + 1,
        _ => natural,
    }
}

fn masses_of(key: &str, sunday: &str, weekday_cycle: &str) -> Vec<Mass> {
    let Some(masses) = data().readings.get(key) else { return Vec::new() };
    masses
        .iter()
        .map(|m| Mass {
            id: m.id.clone(),
            label: m.label.clone(),
            readings: m
                .readings
                .iter()
                .filter(|r| match r.cycle.as_deref() {
                    None | Some("") => true,
                    Some(c @ ("I" | "II")) => c == weekday_cycle,
                    Some(c) => c == sunday,
                })
                .map(|r| Reading {
                    slot: r.slot.clone(),
                    reference: r.reference.clone(),
                    osis: r.osis.clone(),
                    alternatives: r.alternatives.iter().map(|a| Alternative { reference: a.reference.clone(), osis: a.osis.clone() }).collect(),
                })
                .collect(),
        })
        .filter(|m| !m.readings.is_empty())
        .collect()
}

fn celebration_of(saint: &Saint) -> Celebration {
    Celebration { key: saint.key.clone(), en: saint.en.clone(), it: saint.it.clone(), grade: grade_name(saint.grade).into(), color: saint.color.clone() }
}

/// The saints whose celebration falls on a day, transfers and the mobile memorials included.
fn saints_on(day: i64) -> Vec<(&'static Saint, u8)> {
    let d = data();
    let (_, month, date) = civil_from_days(day);
    let mut out: Vec<(&Saint, u8)> = Vec::new();
    for saint in &d.sanctorale {
        // Solemnities may move; everything else stays on its date or is lost.
        let on = if saint.grade >= 6 {
            let civil = civil_from_days(day).0;
            solemnity_date(saint, civil) == day
        } else {
            saint.month == month && saint.day == date
        };
        if on {
            out.push((saint, rank_of_grade(saint.grade)));
        }
    }
    out.sort_by_key(|(s, r)| (*r, s.key.clone()));
    out
}

/// Mobile memorials that the data keeps among the proper of time.
fn mobile_memorials(day: i64, year: &Year) -> Vec<(String, u8, &'static str)> {
    let e = year.easter;
    let mut out = Vec::new();
    if day == e + 50 {
        out.push(("MaryMotherChurch".to_string(), rank::MEMORIAL, "mary-mother-of-the-church"));
    }
    if day == e + 69 {
        out.push(("ImmaculateHeart".to_string(), rank::MEMORIAL, "immaculate-heart-of-mary"));
    }
    out
}

fn iso(day: i64) -> String {
    let (y, m, d) = civil_from_days(day);
    format!("{y:04}-{m:02}-{d:02}")
}

fn mass_label(id: &str) -> &'static str {
    match id {
        "vigil" => "Evening Mass of the Vigil",
        "night" => "Mass during the Night",
        "dawn" => "Mass at Dawn",
        "day" => "Mass during the Day",
        "evening" => "Evening Mass",
        "procession" => "Procession with palms",
        "easter-vigil" => "Easter Vigil in the Holy Night",
        _ => "",
    }
}

/// The liturgical day of a civil date, by the calendar of Italy, with the evening Mass of
/// tomorrow's vigil when tomorrow has one.
pub fn day(year: i32, month: u32, date: u32) -> Day {
    let n = days_from_civil(year, month, date);
    let (mut today, _) = compute(n);
    let (tomorrow, vigil) = compute(n + 1);
    let vigil = vigil.or_else(|| (tomorrow.celebration.key == "Epiphany").then(|| tomorrow.masses.first().cloned()).flatten());
    if let Some(mut mass) = vigil {
        if today.celebration.key != "EasterVigil" {
            mass.id = "vigil".into();
            mass.label = format!("{}: {}", mass_label("vigil"), tomorrow.celebration.en);
            today.masses.push(mass);
        }
    }
    for mass in &mut today.masses {
        if mass.label.is_empty() {
            mass.label = mass_label(&mass.id).into();
        }
    }
    today
}

/// The day count of `YYYY-MM-DD` from 1600 to 4000; None for text that is not a date, or a day the month lacks.
pub fn parse_date(iso: &str) -> Option<i64> {
    let mut parts = iso.split('-').map(|p| p.parse::<u32>().ok());
    let (Some(Some(y)), Some(Some(m)), Some(Some(d)), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else { return None };
    let y = i32::try_from(y).ok().filter(|y| (1600..=4000).contains(y))?;
    let n = days_from_civil(y, m, d);
    (civil_from_days(n) == (y, m, d)).then_some(n)
}

/// The liturgical day of `YYYY-MM-DD`, if it is a date.
pub fn day_of(iso: &str) -> Option<Day> {
    let (y, m, d) = civil_from_days(parse_date(iso)?);
    Some(day(y, m, d))
}

/// A day without the vigil of the next, and its own vigil Mass, which belongs to the evening before.
fn compute(n: i64) -> (Day, Option<Mass>) {
    let y = Year::of(n);
    let t = temporal(n, &y);
    let sunday = y.sunday_cycle();
    let weekly = y.weekday_cycle();
    let temporal_celebration = Celebration {
        key: t.proper.clone().unwrap_or_else(|| t.readings.clone()),
        en: t.en.clone(),
        it: t.it.clone(),
        grade: if t.proper.is_some() && t.grade >= 4 { grade_name(t.grade) } else if t.rank == rank::SUNDAY || (t.rank == rank::PRINCIPAL && weekday(n) == 0) { "sunday" } else { "weekday" }.into(),
        color: t.color.clone(),
    };
    let mut candidates: Vec<(u8, Celebration, String)> = saints_on(n)
        .into_iter()
        .map(|(s, r)| (r, celebration_of(s), s.readings.clone().unwrap_or_default()))
        .collect();
    for (key, r, readings) in mobile_memorials(n, &y) {
        let p = &data().temporale[&key];
        candidates.push((r, Celebration { key: key.clone(), en: p.en.clone(), it: p.it.clone(), grade: "memorial".into(), color: p.color.clone() }, readings.into()));
    }
    candidates.sort_by_key(|(r, c, _)| (*r, c.key.clone()));
    let mut optional = Vec::new();
    let mut optional_masses = Vec::new();
    let (celebration, readings, color) = match candidates.first() {
        // A solemnity or feast that outranks the day takes it, with its own readings.
        Some((r, c, key)) if *r < t.rank && *r <= rank::FEAST => {
            let masses = if key.is_empty() { Vec::new() } else { masses_of(key, sunday, weekly) };
            let masses = if masses.is_empty() { masses_of(&t.readings, sunday, weekly) } else { masses };
            (c.clone(), masses, c.color.clone())
        }
        // An obligatory memorial on an ordinary weekday: its readings if it has its own, else the weekday's.
        Some((r, c, key)) if *r == rank::MEMORIAL && t.rank == rank::WEEKDAY => {
            let own = if key.is_empty() { Vec::new() } else { masses_of(key, sunday, weekly) };
            let masses = if own.is_empty() { masses_of(&t.readings, sunday, weekly) } else { own };
            for (r2, c2, k2) in candidates.iter().skip(1) {
                if *r2 >= rank::MEMORIAL {
                    optional.push(c2.clone());
                    if !k2.is_empty() {
                        optional_masses.extend(masses_of(k2, sunday, weekly));
                    }
                }
            }
            (c.clone(), masses, c.color.clone())
        }
        _ => {
            // Optional memorials, and memorials on privileged weekdays, are offered beside the day.
            if t.rank >= rank::PRIVILEGED {
                for (r, c, k) in &candidates {
                    if *r >= rank::MEMORIAL {
                        optional.push(c.clone());
                        if !k.is_empty() {
                            optional_masses.extend(masses_of(k, sunday, weekly));
                        }
                    }
                }
            }
            (temporal_celebration.clone(), masses_of(&t.readings, sunday, weekly), t.color.clone())
        }
    };
    let color = match color.as_str() {
        "" => "white".into(),
        c => c.to_string(),
    };
    let mut readings = readings;
    let vigil = readings.iter().position(|m| m.id == "vigil").map(|i| readings.remove(i));
    match celebration.key.as_str() {
        "EasterVigil" => readings.retain(|m| m.id == "easter-vigil"),
        "Easter" => readings.retain(|m| m.id != "easter-vigil"),
        _ => {}
    }
    let day = Day {
        date: iso(n),
        season: t.season.into(),
        week: t.week,
        sunday_cycle: sunday.into(),
        weekday_cycle: weekly.into(),
        color,
        celebration,
        optional,
        masses: readings,
        optional_masses,
    };
    (day, vigil)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(n: i64) -> String {
        iso(n)
    }

    #[test]
    fn civil_dates_round_trip_and_know_their_weekday() {
        for n in [-1000, 0, 19_000, 20_760, 40_000] {
            let (y, m, d) = civil_from_days(n);
            assert_eq!(days_from_civil(y, m, d), n);
        }
        assert_eq!(weekday(days_from_civil(2026, 11, 8)), 0);
        assert_eq!(weekday(days_from_civil(1970, 1, 1)), 4);
    }

    #[test]
    fn easter_falls_where_the_tables_put_it() {
        assert_eq!(date(easter(2026)), "2026-04-05");
        assert_eq!(date(easter(2027)), "2027-03-28");
        assert_eq!(date(easter(2038)), "2038-04-25");
        assert_eq!(date(easter(2285)), "2285-03-22");
        assert_eq!(date(advent(2026)), "2026-11-29");
        assert_eq!(date(advent(2027)), "2027-11-28");
    }

    #[test]
    fn the_cycles_follow_the_year_that_ends() {
        assert_eq!(day(2026, 11, 8).sunday_cycle, "A");
        assert_eq!(day(2026, 11, 29).sunday_cycle, "B");
        assert_eq!(day(2027, 2, 1).weekday_cycle, "I");
        assert_eq!(day(2028, 2, 1).weekday_cycle, "II");
    }

    #[test]
    fn sundays_and_seasons() {
        let d = day(2026, 11, 8);
        assert_eq!((d.season.as_str(), d.week, d.color.as_str()), ("ordinary", 32, "green"));
        assert_eq!(d.masses[0].readings.iter().map(|r| r.reference.as_str()).collect::<Vec<_>>()[0], "Wis 6:12-16");
        let advent = day(2026, 11, 29);
        assert_eq!((advent.season.as_str(), advent.celebration.key.as_str()), ("advent", "Advent1"));
        assert_eq!(day(2027, 3, 28).celebration.key, "Easter");
        assert_eq!(day(2027, 2, 10).celebration.key, "AshWednesday");
        assert_eq!(day(2027, 5, 9).celebration.key, "Ascension");
        assert_eq!(day(2027, 11, 21).celebration.key, "ChristKing");
        assert_eq!(day(2027, 1, 10).celebration.key, "BaptismLord");
    }

    #[test]
    fn vigils_belong_to_the_evening_before() {
        let eve = day(2026, 12, 24);
        assert_eq!(eve.masses.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["day", "vigil"]);
        assert_eq!(eve.masses[1].readings[0].reference, "Is 62:1-5");
        let christmas = day(2026, 12, 25);
        assert_eq!(christmas.masses.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["night", "dawn", "day"]);
        assert_eq!(day(2027, 1, 5).masses.last().unwrap().readings[0].reference, "Is 60:1-6");
        assert_eq!(day(2027, 3, 27).masses.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["easter-vigil"]);
        assert_eq!(day(2027, 3, 28).masses.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["day", "evening"]);
        // After the Epiphany the days read by date.
        assert_eq!(day(2027, 1, 8).masses[0].readings[0].reference, "1 Jn 4:7-10");
    }

    #[test]
    fn saints_take_their_days_or_give_way() {
        assert_eq!(day(2026, 11, 30).celebration.key, "StAndrewAp");
        // The Immaculate Conception on the second Sunday of Advent moves to Monday.
        assert_eq!(day(2024, 12, 9).celebration.key, "ImmaculateConception");
        assert_eq!(day(2024, 12, 8).celebration.key, "Advent2");
        // Saint Joseph in Holy Week goes to the Saturday before Palm Sunday.
        assert_eq!(day(2035, 3, 17).celebration.key, "StJoseph");
        // The Annunciation in Holy Week goes to the Monday after the second Sunday of Easter.
        assert_eq!(day(2024, 4, 8).celebration.key, "Annunciation");
        // Patrons of Italy are feasts here.
        assert_eq!(day(2027, 10, 4).celebration.grade, "feast");
        // A memorial in Lent is only offered.
        let lent = day(2027, 3, 4);
        assert_eq!(lent.celebration.grade, "weekday");
        assert!(lent.optional.iter().any(|c| c.key == "StCasimir"));
    }

    #[test]
    fn only_real_dates_are_days() {
        assert_eq!(day_of("2026-12-25").unwrap().celebration.key, "Christmas");
        assert_eq!(day_of("2028-02-29").unwrap().date, "2028-02-29");
        for bad in ["2026-02-29", "2026-02-31", "2026-13-01", "2026-00-10", "2026-01-00", "2026-1", "2026-01-01-01", "1200-01-01", "Christmas"] {
            assert!(day_of(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn the_italian_lectionary_gives_some_days_their_own_readings() {
        let first = |d: &Day| d.masses[0].readings[0].reference.clone();
        assert_eq!(first(&day(2026, 10, 7)), "Acts 1:12-14");
        assert_eq!(first(&day(2026, 11, 21)), "Zec 2:14-17");
        assert_eq!(first(&day(2026, 8, 22)), "Is 9:1-6");
        let souls = day(2026, 11, 2);
        assert_eq!(souls.masses.iter().map(|m| m.label.as_str()).collect::<Vec<_>>(), ["First Mass", "Second Mass", "Third Mass"]);
        assert_eq!(souls.masses[2].readings[0].reference, "Wis 3:1-9");
    }
}
