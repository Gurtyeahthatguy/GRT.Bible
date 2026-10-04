/** Reading and writing references such as "Gv 3,16-18" or "John 3:16". */

import { ALIASES, BOOKS, foldKey, nameIn } from './books.js';

const ROMAN = [['iii', '3'], ['ii', '2'], ['i', '1']];

function bookCandidates(raw) {
  const key = foldKey(raw);
  if (!key) return { exact: [], partial: [] };
  const keys = [key];
  for (const [roman, digit] of ROMAN) {
    if (key.startsWith(roman) && key.length > roman.length + 1) {
      keys.push(digit + key.slice(roman.length));
      break;
    }
  }
  const exact = [];
  for (const k of keys) {
    for (const osis of ALIASES.get(k) || []) {
      if (!exact.includes(osis)) exact.push(osis);
    }
  }
  if (exact.length) return { exact, partial: [] };
  const partial = [];
  if (key.length >= 3) {
    for (const k of keys) {
      for (const [alias, list] of ALIASES) {
        if (alias.length > k.length && alias.startsWith(k)) {
          for (const osis of list) if (!partial.includes(osis)) partial.push(osis);
        }
      }
    }
  }
  partial.sort((a, b) => BOOKS.findIndex((x) => x.osis === a) - BOOKS.findIndex((x) => x.osis === b));
  return { exact: [], partial };
}

const NUMBERS = /^(\d+)(?:[\s,.:;]+(\d+))?(?:\s*-\s*(\d+)(?:[\s,.:;]+(\d+))?)?\s*$/;

/** Parses what a person typed, listing every book it could mean. */
export function parseReference(input) {
  const text = String(input || '')
    .replace(/[–—]/g, '-')
    .trim();
  if (!text) return null;
  const match = /^((?:[1-3]|i{1,3})?\s*\.?\s*[^\d]+?)\s*(\d.*)?$/iu.exec(text);
  if (!match) return null;
  const { exact, partial } = bookCandidates(match[1]);
  const books = exact.length ? exact : partial;
  if (!books.length) return null;
  const result = { books, exact: exact.length > 0, chapter: 1, verse: null, chapterEnd: null, verseEnd: null };
  if (match[2] !== undefined) {
    const numbers = NUMBERS.exec(match[2]);
    if (!numbers) return null;
    const [, c, v, x, y] = numbers.map((n) => (n === undefined ? null : Number(n)));
    result.chapter = c;
    if (v !== null) result.verse = v;
    if (v !== null && x !== null && y === null) result.verseEnd = x;
    if (v === null && x !== null) result.chapterEnd = x;
    if (y !== null) {
      result.chapterEnd = x;
      result.verseEnd = y;
    }
    if (result.chapter < 1 || result.verse === 0) return null;
    if (result.chapterEnd !== null && result.chapterEnd < result.chapter) return null;
    if (result.verseEnd !== null && result.chapterEnd === null && result.verseEnd < result.verse) return null;
  }
  return result;
}

/** "Gv 3,16-17", "John 3:16-17" or "Io 3,16-4,2", in the conventions of a language. */
export function formatReference(language, osis, chapter, verse, chapterEnd = chapter, verseEnd = verse, abbrev = null) {
  const short = abbrev || nameIn(osis, language)[1];
  const sep = language === 'en' ? ':' : ',';
  if (verse === null || verse === undefined) {
    return chapterEnd && chapterEnd !== chapter ? `${short} ${chapter}-${chapterEnd}` : `${short} ${chapter}`;
  }
  if (chapterEnd === chapter || chapterEnd === null || chapterEnd === undefined) {
    return verseEnd && verseEnd !== verse ? `${short} ${chapter}${sep}${verse}-${verseEnd}` : `${short} ${chapter}${sep}${verse}`;
  }
  return `${short} ${chapter}${sep}${verse}-${chapterEnd}${sep}${verseEnd}`;
}

/** "Ps.23.1" into its parts. */
export function splitOsis(ref) {
  const m = /^([1-3]?[A-Za-z]+)\.(\d+)\.(\d+)$/.exec(ref || '');
  return m ? { book: m[1], chapter: Number(m[2]), verse: Number(m[3]) } : null;
}

export function osis(book, chapter, verse) {
  return `${book}.${chapter}.${verse}`;
}

/** Orders internal references in canon order. */
export function compareOsis(a, b) {
  const x = splitOsis(a);
  const y = splitOsis(b);
  if (!x || !y) return 0;
  const bx = BOOKS.findIndex((k) => k.osis === x.book);
  const by = BOOKS.findIndex((k) => k.osis === y.book);
  return bx - by || x.chapter - y.chapter || x.verse - y.verse;
}
