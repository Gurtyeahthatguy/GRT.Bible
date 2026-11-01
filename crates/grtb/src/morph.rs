//! Morphology codes of several schemes brought to one shape.
//!
//! A word's features are ten letters, one per slot: part of speech, person,
//! tense or aspect, voice, mood, case, number, gender, degree or state, and the
//! Hebrew stem. A dash means the slot does not apply. A search for "aorist
//! imperative" becomes a pattern over these letters, whatever the source wrote.

pub const SLOTS: usize = 10;
const POS: usize = 0;
const PERSON: usize = 1;
const TENSE: usize = 2;
const VOICE: usize = 3;
const MOOD: usize = 4;
const CASE: usize = 5;
const NUMBER: usize = 6;
const GENDER: usize = 7;
const STATE: usize = 8;
const STEM: usize = 9;

/// What each letter of each slot means, and the words a search may use for it.
/// Greek and Hebrew share a slot where their categories differ, with their own letters.
const VOCABULARY: &[(usize, char, &str, &[&str])] = &[
    (POS, 'N', "noun", &["noun"]),
    (POS, 'V', "verb", &["verb"]),
    (POS, 'A', "adjective", &["adjective"]),
    (POS, 'D', "adverb", &["adverb"]),
    (POS, 'P', "preposition", &["preposition"]),
    (POS, 'C', "conjunction", &["conjunction"]),
    (POS, 'R', "pronoun", &["pronoun"]),
    (POS, 'T', "article", &["article"]),
    (POS, 'X', "particle", &["particle"]),
    (POS, 'I', "interjection", &["interjection"]),
    (POS, 'S', "suffix", &["suffix"]),
    (PERSON, '1', "first person", &["first", "1st"]),
    (PERSON, '2', "second person", &["second", "2nd"]),
    (PERSON, '3', "third person", &["third", "3rd"]),
    (TENSE, 'P', "present", &["present"]),
    (TENSE, 'I', "imperfect", &["imperfect"]),
    (TENSE, 'F', "future", &["future"]),
    (TENSE, 'A', "aorist", &["aorist"]),
    (TENSE, 'X', "perfect", &["perfect"]),
    (TENSE, 'Y', "pluperfect", &["pluperfect"]),
    (TENSE, 'p', "perfect", &["perfect", "qatal"]),
    (TENSE, 'q', "sequential perfect", &["weqatal"]),
    (TENSE, 'i', "imperfect", &["imperfect", "yiqtol"]),
    (TENSE, 'w', "sequential imperfect", &["wayyiqtol"]),
    (TENSE, 'h', "cohortative", &["cohortative"]),
    (TENSE, 'j', "jussive", &["jussive"]),
    (VOICE, 'A', "active", &["active"]),
    (VOICE, 'M', "middle", &["middle"]),
    (VOICE, 'P', "passive", &["passive"]),
    (VOICE, 'E', "middle or passive", &[]),
    (MOOD, 'I', "indicative", &["indicative"]),
    (MOOD, 'M', "imperative", &["imperative"]),
    (MOOD, 'S', "subjunctive", &["subjunctive"]),
    (MOOD, 'O', "optative", &["optative"]),
    (MOOD, 'N', "infinitive", &["infinitive"]),
    (MOOD, 'P', "participle", &["participle"]),
    (CASE, 'N', "nominative", &["nominative"]),
    (CASE, 'G', "genitive", &["genitive"]),
    (CASE, 'D', "dative", &["dative"]),
    (CASE, 'A', "accusative", &["accusative"]),
    (CASE, 'V', "vocative", &["vocative"]),
    (NUMBER, 'S', "singular", &["singular"]),
    (NUMBER, 'P', "plural", &["plural"]),
    (NUMBER, 'D', "dual", &["dual"]),
    (GENDER, 'M', "masculine", &["masculine"]),
    (GENDER, 'F', "feminine", &["feminine"]),
    (GENDER, 'N', "neuter", &["neuter"]),
    (GENDER, 'C', "common", &["common"]),
    (STATE, 'C', "comparative", &["comparative"]),
    (STATE, 'S', "superlative", &["superlative"]),
    (STATE, 'a', "absolute", &["absolute"]),
    (STATE, 'c', "construct", &["construct"]),
    (STATE, 'd', "determined", &["determined"]),
    (STEM, 'q', "qal", &["qal"]),
    (STEM, 'N', "niphal", &["niphal"]),
    (STEM, 'p', "piel", &["piel"]),
    (STEM, 'P', "pual", &["pual"]),
    (STEM, 'h', "hiphil", &["hiphil"]),
    (STEM, 'H', "hophal", &["hophal"]),
    (STEM, 't', "hithpael", &["hithpael"]),
    (STEM, 'o', "polel", &["polel"]),
    (STEM, 'O', "polal", &["polal"]),
    (STEM, 'r', "hithpolel", &["hithpolel"]),
    (STEM, 'm', "poel", &["poel"]),
    (STEM, 'M', "poal", &["poal"]),
    (STEM, 'k', "palel", &["palel"]),
    (STEM, 'K', "pulal", &["pulal"]),
    (STEM, 'Q', "qal passive", &[]),
    (STEM, 'l', "pilpel", &["pilpel"]),
    (STEM, 'L', "polpal", &["polpal"]),
    (STEM, 'f', "hithpalpel", &["hithpalpel"]),
    (STEM, 'D', "nithpael", &["nithpael"]),
    (STEM, 'j', "pealal", &["pealal"]),
    (STEM, 'i', "pilel", &["pilel"]),
    (STEM, 'u', "hothpaal", &["hothpaal"]),
    (STEM, 'c', "tiphil", &["tiphil"]),
    (STEM, 'v', "hishtaphel", &["hishtaphel"]),
    (STEM, 'w', "nithpalel", &["nithpalel"]),
    (STEM, 'y', "nithpoel", &["nithpoel"]),
    (STEM, 'z', "hithpoel", &["hithpoel"]),
];

/// Aramaic stems use the same letters with other names.
const ARAMAIC_STEMS: &[(char, &str)] = &[
    ('q', "peal"), ('Q', "peil"), ('u', "hithpeel"), ('p', "pael"), ('P', "ithpaal"), ('M', "hithpaal"),
    ('a', "aphel"), ('h', "haphel"), ('s', "saphel"), ('e', "shaphel"), ('H', "hophal"), ('i', "ithpeel"),
    ('t', "hishtaphel"), ('v', "ishtaphel"), ('w', "hithaphel"), ('o', "polel"), ('z', "ithpoel"),
    ('r', "hithpolel"), ('f', "hithpalpel"), ('b', "hephal"), ('c', "tiphel"), ('m', "poel"), ('l', "palpel"),
    ('L', "ithpalpel"), ('O', "ithpolel"), ('G', "ittaphal"),
];

fn blank() -> [char; SLOTS] {
    ['-'; SLOTS]
}

fn set(slots: &mut [char; SLOTS], slot: usize, letter: Option<char>) {
    if let Some(c) = letter {
        slots[slot] = c;
    }
}

fn pick(c: char, pairs: &[(char, char)]) -> Option<char> {
    pairs.iter().find(|(from, _)| *from == c).map(|(_, to)| *to)
}

const GREEK_CASE: &[(char, char)] = &[('N', 'N'), ('G', 'G'), ('D', 'D'), ('A', 'A'), ('V', 'V')];
const GREEK_NUMBER: &[(char, char)] = &[('S', 'S'), ('P', 'P')];
const GREEK_GENDER: &[(char, char)] = &[('M', 'M'), ('F', 'F'), ('N', 'N')];
const GREEK_PERSON: &[(char, char)] = &[('1', '1'), ('2', '2'), ('3', '3')];

/// MorphGNT: two letters of part of speech, then eight positions.
fn morphgnt(code: &str) -> Option<[char; SLOTS]> {
    let chars: Vec<char> = code.chars().collect();
    if chars.len() != 10 {
        return None;
    }
    let mut s = blank();
    s[POS] = match (chars[0], chars[1]) {
        ('N', _) => 'N',
        ('V', _) => 'V',
        ('A', _) => 'A',
        ('D', _) => 'D',
        ('P', _) => 'P',
        ('C', _) => 'C',
        ('R', 'A') => 'T',
        ('R', _) => 'R',
        ('X', _) => 'X',
        ('I', _) => 'I',
        _ => return None,
    };
    let p = &chars[2..];
    set(&mut s, PERSON, pick(p[0], GREEK_PERSON));
    set(&mut s, TENSE, pick(p[1], &[('P', 'P'), ('I', 'I'), ('F', 'F'), ('A', 'A'), ('X', 'X'), ('Y', 'Y')]));
    set(&mut s, VOICE, pick(p[2], &[('A', 'A'), ('M', 'M'), ('P', 'P'), ('D', 'M'), ('O', 'P'), ('N', 'E')]));
    set(&mut s, MOOD, pick(p[3], &[('I', 'I'), ('D', 'M'), ('S', 'S'), ('O', 'O'), ('N', 'N'), ('P', 'P')]));
    set(&mut s, CASE, pick(p[4], GREEK_CASE));
    set(&mut s, NUMBER, pick(p[5], GREEK_NUMBER));
    set(&mut s, GENDER, pick(p[6], GREEK_GENDER));
    set(&mut s, STATE, pick(p[7], &[('C', 'C'), ('S', 'S')]));
    Some(s)
}

/// Robinson's codes, as SWORD modules carry them: `V-AAI-3S`, `N-NSF`, `T-ASM`.
fn robinson(code: &str) -> Option<[char; SLOTS]> {
    let parts: Vec<&str> = code.split('-').collect();
    let head = *parts.first()?;
    let mut s = blank();
    let rest = |i: usize| parts.get(i).copied().unwrap_or("");
    let nominal = |s: &mut [char; SLOTS], cng: &str| {
        let c: Vec<char> = cng.chars().collect();
        // Personal pronouns put the person first: P-1NS.
        let c: Vec<char> = if c.first().map(char::is_ascii_digit).unwrap_or(false) {
            set(s, PERSON, pick(c[0], GREEK_PERSON));
            c[1..].to_vec()
        } else {
            c
        };
        set(s, CASE, c.first().and_then(|x| pick(*x, GREEK_CASE)));
        set(s, NUMBER, c.get(1).and_then(|x| pick(*x, GREEK_NUMBER)));
        set(s, GENDER, c.get(2).and_then(|x| pick(*x, GREEK_GENDER)));
    };
    match head {
        "V" => {
            s[POS] = 'V';
            let tvm: Vec<char> = rest(1).chars().filter(char::is_ascii_alphabetic).collect();
            set(&mut s, TENSE, tvm.first().and_then(|c| pick(*c, &[('P', 'P'), ('I', 'I'), ('F', 'F'), ('A', 'A'), ('R', 'X'), ('L', 'Y')])));
            set(&mut s, VOICE, tvm.get(1).and_then(|c| pick(*c, &[('A', 'A'), ('M', 'M'), ('P', 'P'), ('D', 'M'), ('O', 'P'), ('N', 'E'), ('E', 'E')])));
            set(&mut s, MOOD, tvm.get(2).and_then(|c| pick(*c, &[('I', 'I'), ('M', 'M'), ('S', 'S'), ('O', 'O'), ('N', 'N'), ('P', 'P')])));
            let tail = rest(2);
            if s[MOOD] == 'P' {
                nominal(&mut s, tail);
            } else {
                let c: Vec<char> = tail.chars().collect();
                set(&mut s, PERSON, c.first().and_then(|x| pick(*x, GREEK_PERSON)));
                set(&mut s, NUMBER, c.get(1).and_then(|x| pick(*x, GREEK_NUMBER)));
            }
        }
        "N" | "A" | "T" | "P" | "R" | "C" | "D" | "K" | "I" | "X" | "Q" | "F" | "S" => {
            s[POS] = match head {
                "N" => 'N',
                "A" => 'A',
                "T" => 'T',
                _ => 'R',
            };
            nominal(&mut s, rest(1));
        }
        "ADV" => s[POS] = 'D',
        "CONJ" => s[POS] = 'C',
        "PREP" => s[POS] = 'P',
        "PRT" | "COND" => s[POS] = 'X',
        "INJ" => s[POS] = 'I',
        _ => return None,
    }
    Some(s)
}

/// One morpheme of the Open Scriptures Hebrew Bible, without its language letter.
fn oshb_morpheme(code: &str) -> Option<[char; SLOTS]> {
    let c: Vec<char> = code.chars().collect();
    let at = |i: usize| c.get(i).copied();
    let gender = |x: Option<char>| x.and_then(|g| pick(g, &[('m', 'M'), ('f', 'F'), ('b', 'C'), ('c', 'C')]));
    let number = |x: Option<char>| x.and_then(|n| pick(n, &[('s', 'S'), ('p', 'P'), ('d', 'D')]));
    let person = |x: Option<char>| x.and_then(|p| pick(p, &[('1', '1'), ('2', '2'), ('3', '3')]));
    let state = |x: Option<char>| x.and_then(|t| pick(t, &[('a', 'a'), ('c', 'c'), ('d', 'd')]));
    let mut s = blank();
    match at(0)? {
        'V' => {
            s[POS] = 'V';
            s[STEM] = at(1)?;
            match at(2)? {
                conj @ ('p' | 'q' | 'i' | 'w' | 'h' | 'j') => {
                    s[TENSE] = conj;
                    s[MOOD] = 'I';
                    set(&mut s, PERSON, person(at(3)));
                    set(&mut s, GENDER, gender(at(4)));
                    set(&mut s, NUMBER, number(at(5)));
                }
                'v' => {
                    s[MOOD] = 'M';
                    set(&mut s, PERSON, person(at(3)));
                    set(&mut s, GENDER, gender(at(4)));
                    set(&mut s, NUMBER, number(at(5)));
                }
                p @ ('r' | 's') => {
                    s[MOOD] = 'P';
                    s[VOICE] = if p == 'r' { 'A' } else { 'P' };
                    set(&mut s, GENDER, gender(at(3)));
                    set(&mut s, NUMBER, number(at(4)));
                    set(&mut s, STATE, state(at(5)));
                }
                i @ ('a' | 'c') => {
                    s[MOOD] = 'N';
                    s[STATE] = i;
                }
                _ => {}
            }
        }
        'N' => {
            s[POS] = 'N';
            set(&mut s, GENDER, gender(at(2)));
            set(&mut s, NUMBER, number(at(3)));
            set(&mut s, STATE, state(at(4)));
        }
        'A' => {
            s[POS] = 'A';
            set(&mut s, GENDER, gender(at(2)));
            set(&mut s, NUMBER, number(at(3)));
            set(&mut s, STATE, state(at(4)));
        }
        'P' => {
            s[POS] = 'R';
            set(&mut s, PERSON, person(at(2)));
            set(&mut s, GENDER, gender(at(3)));
            set(&mut s, NUMBER, number(at(4)));
        }
        'S' => {
            s[POS] = 'S';
            set(&mut s, PERSON, person(at(2)));
            set(&mut s, GENDER, gender(at(3)));
            set(&mut s, NUMBER, number(at(4)));
        }
        'T' => s[POS] = if at(1) == Some('d') { 'T' } else { 'X' },
        'R' => s[POS] = 'P',
        'C' => s[POS] = 'C',
        'D' => s[POS] = 'D',
        _ => return None,
    }
    Some(s)
}

/// Whether an OSHB code is Aramaic, and its morphemes: `HC/Vqw3ms`.
pub fn oshb_parts(code: &str) -> Option<(bool, Vec<&str>)> {
    let aramaic = code.starts_with('A');
    let body = code.strip_prefix('H').or_else(|| code.strip_prefix('A'))?;
    Some((aramaic, body.split('/').collect()))
}

/// The features of a code written `scheme:code`; `main` picks the OSHB morpheme of the word's lemma.
pub fn features(code: &str, main: usize) -> Option<String> {
    let (scheme, body) = code.split_once(':')?;
    let slots = match scheme {
        "mgnt" => morphgnt(body)?,
        "robinson" => robinson(body)?,
        "oshb" => {
            let (_, parts) = oshb_parts(body)?;
            oshb_morpheme(parts.get(main).or(parts.last())?)?
        }
        _ => return None,
    };
    Some(slots.iter().collect())
}

/// The features in words: "verb, aorist, active, imperative, second person, singular".
pub fn describe(features: &str, hebrew: bool, aramaic: bool) -> String {
    let mut out = Vec::new();
    for (slot, letter) in features.chars().enumerate() {
        if letter == '-' {
            continue;
        }
        if slot == STEM && aramaic {
            if let Some((_, name)) = ARAMAIC_STEMS.iter().find(|(c, _)| *c == letter) {
                out.push(name.to_string());
                continue;
            }
        }
        let found = VOCABULARY.iter().find(|(s, c, _, _)| *s == slot && *c == letter);
        if let Some((_, _, name, _)) = found {
            // The mood of a finite Hebrew verb is implied by its aspect.
            if hebrew && slot == MOOD && letter == 'I' {
                continue;
            }
            out.push(name.to_string());
        }
    }
    out.join(", ")
}

/// A LIKE pattern over the features for words such as "aorist imperative" or "qal perfect".
/// Words that are not morphology are returned apart; `hebrew` chooses between the tenses.
pub fn pattern(words: &[&str], hebrew: bool) -> (String, Vec<String>) {
    let mut slots = ['_'; SLOTS];
    let mut unknown = Vec::new();
    for word in words {
        let w = word.to_lowercase();
        let found = VOCABULARY.iter().find(|(slot, c, _, names)| {
            names.contains(&w.as_str()) && (*slot != TENSE || c.is_ascii_lowercase() == hebrew)
        });
        match found {
            Some((slot, c, _, _)) => slots[*slot] = *c,
            None => unknown.push(word.to_string()),
        }
    }
    (slots.iter().collect(), unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morphgnt_codes() {
        assert_eq!(features("mgnt:V-2AAD-S--", 0).unwrap(), "V2AAM-S---");
        assert_eq!(features("mgnt:N-----NSF-", 0).unwrap(), "N----NSF--");
        assert_eq!(features("mgnt:RA----GSM-", 0).unwrap(), "T----GSM--");
        assert_eq!(describe("V2AAM-S---", false, false), "verb, second person, aorist, active, imperative, singular");
    }

    #[test]
    fn robinson_codes() {
        assert_eq!(features("robinson:V-AAI-3S", 0).unwrap(), "V3AAI-S---");
        assert_eq!(features("robinson:V-PAP-NSM", 0).unwrap(), "V-PAPNSM--");
        assert_eq!(features("robinson:N-NSF", 0).unwrap(), "N----NSF--");
        assert_eq!(features("robinson:P-1NS", 0).unwrap(), "R1---NS---");
        assert_eq!(features("robinson:CONJ", 0).unwrap(), "C---------");
        assert_eq!(features("robinson:V-2AAI-3S", 0).unwrap(), "V3AAI-S---");
    }

    #[test]
    fn oshb_codes() {
        assert_eq!(features("oshb:HVqp3ms", 0).unwrap(), "V3p-I-SM-q");
        assert_eq!(features("oshb:HR/Ncfsa", 1).unwrap(), "N-----SFa-");
        assert_eq!(features("oshb:HC/Vqw3ms", 1).unwrap(), "V3w-I-SM-q");
        assert_eq!(features("oshb:HVhrmsa", 0).unwrap(), "V--AP-SMah");
        assert_eq!(describe("V3p-I-SM-q", true, false), "verb, third person, perfect, singular, masculine, qal");
        assert_eq!(describe("V3p-I-SM-q", true, true), "verb, third person, perfect, singular, masculine, peal");
        assert!(features("strongMorph:TH8804", 0).is_none());
    }

    #[test]
    fn search_words() {
        assert_eq!(pattern(&["aorist", "imperative"], false), ("__A_M_____".into(), vec![]));
        assert_eq!(pattern(&["qal", "perfect", "3rd"], true), ("_3p______q".into(), vec![]));
        assert_eq!(pattern(&["perfect"], false).0, "__X_______");
        assert_eq!(pattern(&["love", "plural"], false), ("______P___".into(), vec!["love".into()]));
    }
}
