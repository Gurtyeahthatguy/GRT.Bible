//! Reading plans: the canon cut into days of about the same length.

use crate::canon::{book_by_osis, books};
use crate::refs::Ref;

pub struct Plan {
    pub id: &'static str,
    pub title: &'static str,
    pub days: u32,
    books: &'static [&'static str],
}

const NEW_TESTAMENT: &[&str] = &[
    "Matt", "Mark", "Luke", "John", "Acts", "Rom", "1Cor", "2Cor", "Gal", "Eph", "Phil", "Col", "1Thess", "2Thess", "1Tim", "2Tim", "Titus", "Phlm",
    "Heb", "Jas", "1Pet", "2Pet", "1John", "2John", "3John", "Jude", "Rev",
];

pub const PLANS: [Plan; 5] = [
    Plan { id: "bible-year", title: "The whole Bible in a year", days: 365, books: &[] },
    Plan { id: "bible-two-years", title: "The whole Bible in two years", days: 730, books: &[] },
    Plan { id: "new-testament-90", title: "The New Testament in 90 days", days: 90, books: NEW_TESTAMENT },
    Plan { id: "gospels-40", title: "The four Gospels in 40 days", days: 40, books: &["Matt", "Mark", "Luke", "John"] },
    Plan { id: "psalms-30", title: "The Psalms in 30 days", days: 30, books: &["Ps"] },
];

pub fn plan(id: &str) -> Option<&'static Plan> {
    PLANS.iter().find(|p| p.id == id)
}

impl Plan {
    /// The chapters of each day, as ranges of whole chapters, cut where the verses even out.
    pub fn schedule(&self) -> Vec<Vec<(Ref, Ref)>> {
        let chosen: Vec<_> = if self.books.is_empty() {
            books().iter().collect()
        } else {
            self.books.iter().filter_map(|o| book_by_osis(o)).collect()
        };
        let chapters: Vec<(u8, u16, u32)> = chosen
            .iter()
            .flat_map(|b| (1..=b.chapters()).map(move |c| (b.index, c, b.verse_count(c) as u32)))
            .collect();
        let total: u32 = chapters.iter().map(|c| c.2).sum();
        let days = (self.days as usize).min(chapters.len()).max(1);
        let mut out: Vec<Vec<(u8, u16)>> = Vec::with_capacity(days);
        let mut done = 0u32;
        let mut current: Vec<(u8, u16)> = Vec::new();
        for (i, &(book, chapter, verses)) in chapters.iter().enumerate() {
            current.push((book, chapter));
            done += verses;
            let left_days = days - out.len();
            let left_chapters = chapters.len() - i - 1;
            // Close the day when it reaches its share, or when every remaining day needs a chapter.
            let share = total as f64 * (out.len() + 1) as f64 / days as f64;
            if left_days > 1 && (done as f64 >= share || left_chapters < left_days) {
                out.push(std::mem::take(&mut current));
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
        out.into_iter().map(|day| merge(&day)).collect()
    }
}

/// Consecutive chapters of one book as one range.
fn merge(chapters: &[(u8, u16)]) -> Vec<(Ref, Ref)> {
    let mut out: Vec<(Ref, Ref)> = Vec::new();
    for &(book, chapter) in chapters {
        let last_verse = books()[book as usize - 1].verse_count(chapter);
        match out.last_mut() {
            Some((_, end)) if end.book == book && end.chapter + 1 == chapter => *end = Ref::new(book, chapter, last_verse),
            _ => out.push((Ref::new(book, chapter, 1), Ref::new(book, chapter, last_verse))),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_chapter_is_read_once_in_order() {
        for p in &PLANS {
            let schedule = p.schedule();
            assert_eq!(schedule.len() as u32, p.days, "{}", p.id);
            let mut seen: Vec<(u8, u16)> = Vec::new();
            for day in &schedule {
                assert!(!day.is_empty(), "{} has an empty day", p.id);
                for (a, b) in day {
                    for c in a.chapter..=b.chapter {
                        seen.push((a.book, c));
                    }
                }
            }
            let mut sorted = seen.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(seen, sorted, "{} repeats or reorders", p.id);
        }
    }

    #[test]
    fn days_are_about_as_long_as_each_other() {
        let schedule = plan("bible-year").unwrap().schedule();
        let verses = |day: &Vec<(Ref, Ref)>| -> u32 {
            day.iter().map(|(a, b)| (a.chapter..=b.chapter).map(|c| books()[a.book as usize - 1].verse_count(c) as u32).sum::<u32>()).sum()
        };
        let lengths: Vec<u32> = schedule.iter().map(verses).collect();
        let max = *lengths.iter().max().unwrap();
        assert!(max < 200, "the longest day has {max} verses");
        assert_eq!(schedule[0][0].0.osis(), "Gen.1.1");
        assert_eq!(plan("psalms-30").unwrap().schedule()[0][0].0.osis(), "Ps.1.1");
    }
}
