//! The verse of the day, chosen from a fixed list without any network.

use crate::refs::{parse_range, Ref};

const LIST: &str = include_str!("../data/votd.txt");

pub fn list() -> Vec<(Ref, Ref)> {
    LIST.lines().filter(|l| !l.trim().is_empty()).map(|l| parse_range(l).expect("votd.txt is valid")).collect()
}

/// The passage for a day of the year, counted from 1.
pub fn for_day(day: u32) -> (Ref, Ref) {
    let all = list();
    let step = 211usize;
    all[(day as usize * step) % all.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_passage_is_in_the_canon() {
        for (a, b) in list() {
            assert!(a.in_canon() && b.in_canon(), "{a}");
        }
    }

    #[test]
    fn no_repeats_within_a_year() {
        let days: HashSet<(Ref, Ref)> = (1..=366).map(for_day).collect();
        assert_eq!(days.len(), 366);
    }
}
