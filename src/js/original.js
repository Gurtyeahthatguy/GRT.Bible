/** Greek and Hebrew as the reader wants them: accents, vowel points and cantillation on or off, Greek in Latin letters. */

const GREEK = new Set(['grc', 'el']);
const HEBREW = new Set(['hbo', 'he', 'heb', 'arc']);

export const isGreek = (language) => GREEK.has(language);
export const isHebrew = (language) => HEBREW.has(language);

const CANTILLATION = /[֑-֯]/g;
const POINTS = /[ְ-ׇֽֿׁׂׅׄ]/g;
const COMBINING = /[̀-ͯ]/g;

/** A function that shapes text of a language by the reader's choices. */
export function shaper(language, choices = {}) {
  const { greekAccents = true, hebrewPoints = true, hebrewAccents = true } = choices;
  if (isGreek(language) && !greekAccents) return (text) => text.normalize('NFD').replace(COMBINING, '').normalize('NFC');
  if (isHebrew(language) && (!hebrewPoints || !hebrewAccents)) {
    return (text) => {
      let out = text.replace(CANTILLATION, '');
      if (!hebrewPoints) out = out.replace(POINTS, '');
      return out;
    };
  }
  return (text) => text;
}

const LETTERS = {
  α: 'a', β: 'b', γ: 'g', δ: 'd', ε: 'e', ζ: 'z', η: 'ē', θ: 'th', ι: 'i', κ: 'k', λ: 'l', μ: 'm', ν: 'n',
  ξ: 'x', ο: 'o', π: 'p', ρ: 'r', σ: 's', ς: 's', τ: 't', υ: 'y', φ: 'ph', χ: 'ch', ψ: 'ps', ω: 'ō',
};
const GUTTURAL = new Set(['γ', 'κ', 'ξ', 'χ']);

/** A Greek word in Latin letters, as the SBL Handbook writes them. */
export function transliterateGreek(word) {
  const letters = [];
  for (const ch of word.normalize('NFD')) {
    const lower = ch.toLowerCase();
    if (LETTERS[lower]) letters.push({ base: lower, upper: ch !== lower, rough: false, subscript: false });
    else if (ch === '̔' && letters.length) letters[letters.length - 1].rough = true;
    else if (ch === 'ͅ' && letters.length) letters[letters.length - 1].subscript = true;
  }
  let out = '';
  letters.forEach((l, i) => {
    const prev = letters[i - 1];
    const next = letters[i + 1];
    let t = LETTERS[l.base];
    if (l.base === 'γ' && next && GUTTURAL.has(next.base)) t = 'n';
    if (l.base === 'υ' && prev && ['α', 'ε', 'η', 'ο'].includes(prev.base)) t = 'u';
    if (l.base === 'υ' && next?.base === 'ι') t = 'u';
    if (l.subscript) t += 'i';
    out += t;
  });
  // A rough breathing sounds before the first vowel or diphthong, and after an initial rho.
  if (letters.slice(0, 2).some((l) => l.rough) && out) {
    out = letters[0].base === 'ρ' ? `rh${out.slice(1)}` : `h${out}`;
  }
  if (letters[0]?.upper && out) out = out[0].toUpperCase() + out.slice(1);
  return out;
}

const SHORT = [
  { V: 'verb', N: 'noun', A: 'adj', D: 'adv', P: 'prep', C: 'conj', R: 'pron', T: 'art', X: 'part', I: 'interj', S: 'suff' },
  { 1: '1', 2: '2', 3: '3' },
  { P: 'pres', I: 'impf', F: 'fut', A: 'aor', X: 'perf', Y: 'plup', p: 'perf', q: 'wqtl', i: 'impf', w: 'wyqtl', h: 'coh', j: 'juss' },
  { A: 'act', M: 'mid', P: 'pass', E: 'm/p' },
  { I: 'ind', M: 'impv', S: 'subj', O: 'opt', N: 'inf', P: 'ptc' },
  { N: 'nom', G: 'gen', D: 'dat', A: 'acc', V: 'voc' },
  { S: 's', P: 'p', D: 'd' },
  { M: 'm', F: 'f', N: 'n', C: 'c' },
  { C: 'comp', S: 'sup', a: 'abs', c: 'cstr', d: 'det' },
  { q: 'qal', N: 'niph', p: 'piel', P: 'pual', h: 'hiph', H: 'hoph', t: 'hith' },
];

/** The features in a few letters for the interlinear: "aor act ind 3s". */
export function shortParsing(features, hebrew = false) {
  if (!features || features.length < 10) return '';
  const f = [...features];
  const word = (slot) => (f[slot] === '-' ? '' : SHORT[slot][f[slot]] || '');
  const parts = [word(0), hebrew ? word(9) : '', word(2), word(3), hebrew && f[4] === 'I' ? '' : word(4), word(5)];
  const pgn = `${word(1)}${word(7)}${word(6)}`;
  if (pgn) parts.push(pgn);
  parts.push(word(8));
  return parts.filter(Boolean).join(' ');
}
