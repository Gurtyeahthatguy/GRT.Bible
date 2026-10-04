//! Conversion between a module's numbering and the internal scheme.

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use crate::refs::Ref;

/// The schemes shipped with the program, as `(name, mapping lines)`.
const SCHEMES: [(&str, &str); 7] = [
    ("hebrew", include_str!("../data/versification/hebrew.map")),
    ("vulgate", include_str!("../data/versification/vulgate.map")),
    ("english", include_str!("../data/versification/english.map")),
    ("lxx", include_str!("../data/versification/lxx.map")),
    ("martini", include_str!("../data/versification/martini.map")),
    ("douay", include_str!("../data/versification/douay.map")),
    ("webc", include_str!("../data/versification/webc.map")),
];

/// Schemes offered when importing a module.
pub const GENERAL_SCHEMES: [&str; 4] = ["hebrew", "vulgate", "english", "lxx"];

#[derive(Debug, Default)]
pub struct Scheme {
    pub name: String,
    forward: HashMap<Ref, (Ref, Ref)>,
    reverse: HashMap<Ref, Ref>,
}

fn parse_side(text: &str) -> Result<(Ref, u16), String> {
    let text = text.trim();
    let (head, last) = match text.rsplit_once('-') {
        Some((h, l)) => (h, Some(l)),
        None => (text, None),
    };
    let start = Ref::parse(head).ok_or_else(|| format!("bad reference {text}"))?;
    let end = match last {
        Some(l) => l.parse::<u16>().map_err(|_| format!("bad range {text}"))?,
        None => start.verse,
    };
    if end < start.verse {
        return Err(format!("backwards range {text}"));
    }
    Ok((start, end - start.verse + 1))
}

impl Scheme {
    pub fn identity(name: &str) -> Scheme {
        Scheme { name: name.to_string(), ..Default::default() }
    }

    pub fn parse(name: &str, text: &str) -> Result<Scheme, String> {
        let mut forward = HashMap::new();
        for (number, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let (a, b) = line.split_once('=').ok_or_else(|| format!("{name}:{}: no '='", number + 1))?;
            let (src, n) = parse_side(a).map_err(|e| format!("{name}:{}: {e}", number + 1))?;
            let (dst, m) = parse_side(b).map_err(|e| format!("{name}:{}: {e}", number + 1))?;
            let at = |r: Ref, i: u16| Ref { verse: r.verse + i, ..r };
            if n == m {
                for i in 0..n {
                    forward.insert(at(src, i), (at(dst, i), at(dst, i)));
                }
            } else if n == 1 {
                forward.insert(src, (dst, at(dst, m - 1)));
            } else if m == 1 {
                for i in 0..n {
                    forward.insert(at(src, i), (dst, dst));
                }
            } else {
                return Err(format!("{name}:{}: ranges of different lengths", number + 1));
            }
        }
        let ordered: BTreeMap<Ref, (Ref, Ref)> = forward.iter().map(|(k, v)| (*k, *v)).collect();
        let mut reverse = HashMap::new();
        for (src, (a, b)) in ordered {
            for verse in a.verse..=b.verse {
                reverse.entry(Ref { verse, ..a }).or_insert(src);
            }
        }
        Ok(Scheme { name: name.to_string(), forward, reverse })
    }

    /// The internal verse, or range of verses, a module verse stands for.
    pub fn to_canon(&self, r: Ref) -> (Ref, Ref) {
        self.forward.get(&r).copied().unwrap_or((r, r))
    }

    fn direct(&self, canon: Ref, exists: &dyn Fn(Ref) -> bool) -> Option<Ref> {
        if let Some(r) = self.reverse.get(&canon) {
            return Some(*r);
        }
        if !self.forward.contains_key(&canon) && exists(canon) {
            return Some(canon);
        }
        None
    }

    /// The module verse showing an internal verse, or the nearest one in the same chapter.
    pub fn from_canon(&self, canon: Ref, exists: &dyn Fn(Ref) -> bool) -> Option<Ref> {
        if let Some(r) = self.direct(canon, exists) {
            return Some(r);
        }
        let limit = crate::canon::book(canon.book).map(|b| b.verse_count(canon.chapter)).unwrap_or(0).max(8);
        for distance in 1..=limit {
            let ahead = Ref { verse: canon.verse + distance, ..canon };
            if ahead.in_canon() {
                if let Some(r) = self.direct(ahead, exists) {
                    return Some(r);
                }
            }
            if canon.verse > distance {
                let behind = Ref { verse: canon.verse - distance, ..canon };
                if let Some(r) = self.direct(behind, exists) {
                    return Some(r);
                }
            }
        }
        None
    }

    pub fn explicit_lines(&self) -> usize {
        self.forward.len()
    }

    pub fn targets(&self) -> impl Iterator<Item = (Ref, (Ref, Ref))> + '_ {
        self.forward.iter().map(|(k, v)| (*k, *v))
    }
}

fn registry() -> &'static HashMap<&'static str, Scheme> {
    static REGISTRY: OnceLock<HashMap<&'static str, Scheme>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        SCHEMES
            .iter()
            .map(|(name, text)| (*name, Scheme::parse(name, text).expect("shipped versification data is valid")))
            .collect()
    })
}

/// A shipped scheme; unknown names get the internal numbering unchanged.
pub fn scheme(name: &str) -> &'static Scheme {
    static IDENTITY: OnceLock<Scheme> = OnceLock::new();
    registry().get(name).unwrap_or_else(|| IDENTITY.get_or_init(|| Scheme::identity("hebrew")))
}

pub fn is_known(name: &str) -> bool {
    registry().contains_key(name)
}

pub fn names() -> Vec<&'static str> {
    SCHEMES.iter().map(|(n, _)| *n).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(text: &str) -> Ref {
        Ref::parse(text).unwrap()
    }

    fn to(scheme_name: &str, text: &str) -> String {
        scheme(scheme_name).to_canon(r(text)).0.osis()
    }

    #[test]
    fn known_divergences() {
        assert_eq!(to("vulgate", "Ps.22.1"), "Ps.23.1");
        assert_eq!(to("vulgate", "Ps.50.3"), "Ps.51.3");
        assert_eq!(to("vulgate", "Ps.113.9"), "Ps.115.1");
        assert_eq!(to("english", "Ps.51.1"), "Ps.51.3");
        assert_eq!(to("english", "Mal.4.1"), "Mal.3.19");
        assert_eq!(to("english", "Joel.2.28"), "Joel.3.1");
        assert_eq!(to("english", "Dan.4.1"), "Dan.3.98");
        assert_eq!(to("english", "Dan.3.24"), "Dan.3.91");
        assert_eq!(to("hebrew", "Dan.3.24"), "Dan.3.91");
        assert_eq!(to("hebrew", "Gen.1.1"), "Gen.1.1");
        assert_eq!(to("lxx", "Ps.9.22"), "Ps.10.1");
        assert_eq!(to("martini", "Ps.50.2"), "Ps.51.3");
        assert_eq!(to("martini", "Deut.13.1"), "Deut.13.1");
        assert_eq!(to("vulgate", "Deut.13.1"), "Deut.13.2");
        assert_eq!(to("webc", "Esth.4.18"), "Esth.13.8");
        assert_eq!(to("webc", "Rom.14.24"), "Rom.16.25");
        assert_eq!(to("webc", "Bar.6.2"), "Bar.6.1");
    }

    #[test]
    fn back_to_the_module() {
        let all = |_: Ref| true;
        assert_eq!(scheme("vulgate").from_canon(r("Ps.23.1"), &all).unwrap().osis(), "Ps.22.1");
        assert_eq!(scheme("english").from_canon(r("Mal.3.19"), &all).unwrap().osis(), "Mal.4.1");
        // A superscription the English numbering leaves out lands on the first verse.
        assert_eq!(scheme("english").from_canon(r("Ps.51.1"), &all).unwrap().osis(), "Ps.51.1");
        assert_eq!(scheme("hebrew").from_canon(r("Dan.3.95"), &all).unwrap().osis(), "Dan.3.28");
    }

    #[test]
    fn every_target_is_in_the_canon() {
        for name in names() {
            for (src, (a, b)) in scheme(name).targets() {
                assert!(a.in_canon() && b.in_canon(), "{name}: {src} maps outside the canon");
            }
        }
    }

    #[test]
    fn a_note_survives_the_round_trip() {
        let all = |_: Ref| true;
        for name in names() {
            let s = scheme(name);
            for (src, _) in s.targets() {
                let (canon, _) = s.to_canon(src);
                let back = s.from_canon(canon, &all).unwrap();
                assert_eq!(s.to_canon(back).0, canon, "{name}: {src}");
            }
        }
    }
}
