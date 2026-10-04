// @vitest-environment jsdom

import { describe, it, expect } from 'vitest';

import { copyText } from '../src/js/copy.js';
import { markTerms } from '../src/js/dom.js';
import { nextSelection, normaliseUser, swipeDirection } from '../src/js/main.js';
import { annotationsFor } from '../src/js/reader.js';
import { DEFAULTS, normalise } from '../src/js/settings.js';

const verses = [
  { chapter: 3, verse: 16, text: 'Sic enim Deus dilexit mundum.' },
  { chapter: 3, verse: 17, text: 'Non enim misit Deus.' },
];

describe('copyText', () => {
  it('puts the reference after the text by default', () => {
    const text = copyText({ verses, book: 'John', language: 'la', module: 'VUL', options: DEFAULTS.copy });
    expect(text).toBe('Sic enim Deus dilexit mundum. Non enim misit Deus.\nIo 3,16-17 (VUL)');
  });

  it('can lead with the reference, drop the module and number the verses', () => {
    const options = { refPosition: 'before', includeModule: false, includeNumbers: true };
    const text = copyText({ verses, book: 'John', language: 'it', module: 'MAR', options });
    expect(text).toBe('Gv 3,16-17\n16 Sic enim Deus dilexit mundum. 17 Non enim misit Deus.');
  });

  it('keeps an exact selection of words', () => {
    const text = copyText({ verses: verses.slice(0, 1), excerpt: 'dilexit mundum', book: 'John', language: 'en', module: 'WEBC', options: DEFAULTS.copy });
    expect(text).toBe('dilexit mundum\nJohn 3:16 (WEBC)');
  });
});

describe('settings', () => {
  it('fills in defaults and keeps values in range', () => {
    const s = normalise({ theme: 'sepia', fontSize: 99, lineWidth: 40, layout: 'nonsense', copy: { refPosition: 'before' } });
    expect(s.theme).toBe('sepia');
    expect(s.fontSize).toBe(36);
    expect(s.lineWidth).toBe(60);
    expect(s.layout).toBe('paragraphs');
    expect(s.copy).toEqual({ refPosition: 'before', includeModule: true, includeNumbers: false });
    expect(s.sourceNotes).toBe(false);
    expect(s.videos).toBeNull();
  });
});

describe('selection', () => {
  const order = [1, 2, 3, 4, 5, 6];

  it('grows next to the run and restarts elsewhere', () => {
    expect(nextSelection([], 3, null, order)).toEqual([3]);
    expect(nextSelection([3], 4, null, order)).toEqual([3, 4]);
    expect(nextSelection([3, 4], 2, null, order)).toEqual([2, 3, 4]);
    expect(nextSelection([3, 4], 6, null, order)).toEqual([6]);
  });

  it('shrinks from the ends and extends with Shift', () => {
    expect(nextSelection([2, 3, 4], 2, null, order)).toEqual([3, 4]);
    expect(nextSelection([2, 3, 4], 3, null, order)).toEqual([3]);
    expect(nextSelection([3], 3, null, order)).toEqual([]);
    expect(nextSelection([2], 5, 2, order)).toEqual([2, 3, 4, 5]);
  });
});

describe('annotations', () => {
  it('find highlights, notes and bookmarks by internal reference', () => {
    const chapter = [
      { verse: 1, canon: 'Ps.51.3', canonEnd: 'Ps.51.3' },
      { verse: 2, canon: 'Ps.51.4', canonEnd: 'Ps.51.4' },
    ];
    const user = normaliseUser({
      highlights: [{ start: 'Ps.51.4', end: 'Ps.51.6', color: 'blue' }],
      notes: [{ id: 'n1', start: 'Ps.51.3', end: 'Ps.51.4', text: 'x' }],
      bookmarks: [{ id: 'b1', ref: 'Ps.51.4', label: '' }],
    });
    const marks = annotationsFor(chapter, user);
    expect(marks.get(1)).toMatchObject({ color: null, bookmark: null });
    expect(marks.get(1).notes).toHaveLength(1);
    expect(marks.get(2).color).toBe('blue');
    expect(marks.get(2).notes).toHaveLength(0);
    expect(marks.get(2).bookmark.id).toBe('b1');
  });
});

describe('markTerms', () => {
  it('finds folded terms in accented text', () => {
    const parts = markTerms('Perché Iddio è luce', ['perche', 'luce'], 'it');
    const marked = parts.filter((p) => p instanceof HTMLElement).map((p) => p.textContent);
    expect(marked).toEqual(['Perché', 'luce']);
  });

  it('folds Latin spellings', () => {
    const parts = markTerms('Vivit ejus anima', ['eius'], 'la');
    expect(parts.filter((p) => p instanceof HTMLElement).map((p) => p.textContent)).toEqual(['ejus']);
  });
});

describe('swipeDirection', () => {
  const stroke = (dx, dy, ms) => swipeDirection({ x: 200, y: 400, at: 0 }, { x: 200 + dx, y: 400 + dy, at: ms });

  it('turns the page on a quick sideways stroke', () => {
    expect(stroke(-120, 10, 200)).toBe('next');
    expect(stroke(120, -10, 200)).toBe('prev');
  });

  it('leaves scrolling, short strokes and slow drags alone', () => {
    expect(stroke(-120, 90, 200)).toBeNull();
    expect(stroke(-40, 0, 200)).toBeNull();
    expect(stroke(-120, 0, 900)).toBeNull();
  });
});
