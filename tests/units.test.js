// @vitest-environment jsdom

import { describe, it, expect } from 'vitest';

import { copyHtml, copyText } from '../src/js/copy.js';
import { markTerms } from '../src/js/dom.js';
import { renderGuide } from '../src/js/guide.js';
import { libraryTabs } from '../src/js/library.js';
import { renderMarkup, splitRange } from '../src/js/markup.js';
import { textArea } from '../src/js/print.js';
import { notesMarkdown } from '../src/js/study.js';
import { anchorsByVerse, videoTouches } from '../src/js/videos.js';
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

describe('copyHtml', () => {
  it('gives Paper a quotation and its reference', () => {
    const html = copyHtml({ verses, book: 'John', language: 'la', module: 'VUL', options: DEFAULTS.copy });
    expect(html).toBe('<blockquote>Sic enim Deus dilexit mundum. Non enim misit Deus.</blockquote><p><cite>Io 3,16-17 (VUL)</cite></p>');
  });

  it('escapes the text and numbers the verses when asked', () => {
    const options = { refPosition: 'before', includeModule: false, includeNumbers: true };
    const html = copyHtml({ verses: [{ chapter: 1, verse: 1, text: 'a < b & "c"' }], book: 'Gen', language: 'en', module: 'X', options });
    expect(html).toBe('<p><cite>Gen 1:1</cite></p><blockquote><sup>1</sup>a &lt; b &amp; &quot;c&quot;</blockquote>');
  });
});

describe('notesMarkdown', () => {
  it('writes front matter, a heading per note and the verse as a quote', () => {
    const md = notesMarkdown([
      { label: 'Gv 3,16', text: 'Dio infatti ha tanto amato', note: { text: 'Love first.' } },
      { label: 'Sal 23,1', text: null, note: { text: '' } },
    ], { module: 'MAR', date: '2026-10-27' });
    expect(md).toBe([
      '---', 'title: Notes from GRT Bible', 'tags:', '  - bible', 'created: 2026-10-27', '---', '',
      '# Notes from GRT Bible', '', '## Gv 3,16 (MAR)', '', '> Dio infatti ha tanto amato', '', 'Love first.', '',
      '## Sal 23,1 (MAR)', '',
    ].join('\n'));
  });
});

describe('renderMarkup', () => {
  it('keeps the subset and turns links into buttons', () => {
    const seen = [];
    const box = document.createElement('div');
    box.append(renderMarkup('<h4>On</h4><p><b>Ver.</b> 1. See <a data-ref="Luke.3.23-Luke.3.38">Luke</a> and <a data-key="AARON">Aaron</a>.</p>', {
      onRef: (range) => seen.push(range),
      onKey: (key) => seen.push(key),
    }));
    expect(box.innerHTML).toBe('<h4>On</h4><p><b>Ver.</b> 1. See <button class="link ref" type="button" data-start="Luke.3.23" data-end="Luke.3.38">Luke</button> and <button class="link key" type="button" data-key="AARON">Aaron</button>.</p>');
    box.querySelector('.ref').click();
    box.querySelector('.key').click();
    expect(seen).toEqual([{ start: 'Luke.3.23', end: 'Luke.3.38' }, 'AARON']);
  });

  it('drops everything outside the subset, scripts included', () => {
    const box = document.createElement('div');
    box.append(renderMarkup('<p onclick="x()">Hi <script>alert(1)</script><img src=x onerror="y()"><a href="https://example.com" data-ref="javascript:x">there</a><iframe src="x"></iframe></p>'));
    expect(box.innerHTML).toBe('<p>Hi there</p>');
    expect(splitRange('javascript:x')).toBeNull();
    expect(splitRange('Gen.1.1')).toEqual({ start: 'Gen.1.1', end: 'Gen.1.1' });
  });
});

describe('the guide', () => {
  it('draws headings, steps, code and tables without links that leave the program', () => {
    const el = renderGuide('# Modules\n\nA **module** is a file.\n\n1. Open the `Library`.\n2. Choose [Install](https://example.com).\n\n```bash\nls ~/x\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |\n');
    expect(el.querySelector('h2').textContent).toBe('Modules');
    expect(el.querySelector('p strong').textContent).toBe('module');
    expect([...el.querySelectorAll('ol li')].map((li) => li.textContent)).toEqual(['Open the Library.', 'Choose Install (https://example.com).']);
    expect(el.querySelector('a')).toBeNull();
    expect(el.querySelector('pre code').textContent).toBe('ls ~/x');
    expect(el.querySelectorAll('tr')).toHaveLength(2);
  });
});

describe('the library and videos', () => {
  it('shows a tab only for the kinds that are installed', () => {
    const ids = (modules) => libraryTabs(modules).map((t) => t.id);
    expect(ids([{ kind: 'bible' }])).toEqual(['passage', 'installed']);
    expect(ids([{ kind: 'bible' }, { kind: 'dictionary' }, { kind: 'book' }])).toEqual(['passage', 'dictionary', 'books', 'installed']);
  });

  it('puts a linked video on the first verse of its passage in the chapter', () => {
    const chapter = [1, 2, 3].map((v) => ({ verse: v, canon: `Ps.23.${v}`, canonEnd: `Ps.23.${v}` }));
    const anchors = [
      { id: 'a', video: 'v1', start: 'Ps.23.2', end: 'Ps.23.6' },
      { id: 'b', video: 'v2', start: 'Ps.22.20', end: 'Ps.23.1' },
      { id: 'c', video: 'v3', start: 'Ps.24.1', end: 'Ps.24.1' },
    ];
    const map = anchorsByVerse(anchors, chapter);
    expect([...map.entries()]).toEqual([[2, ['v1']], [1, ['v2']]]);
    expect(videoTouches({ id: 'v1', refs: [] }, anchors, 'Ps.23.4', 'Ps.23.4')).toBe(true);
    expect(videoTouches({ id: 'v9', refs: [{ start: 'Ps.23.1', end: 'Ps.23.6' }] }, anchors, 'Ps.23.4', 'Ps.23.4')).toBe(true);
    expect(videoTouches({ id: 'v3', refs: [] }, anchors, 'Ps.23.4', 'Ps.23.4')).toBe(false);
  });

  it('keeps library and print settings within their choices', () => {
    const s = normalise({ library: { tab: 'books', commentary: 'MHC', dictionary: 3 }, print: { paper: 'tabloid', size: 40, myNotes: true } });
    expect(s.library).toEqual({ tab: 'books', commentary: 'MHC', dictionary: null, book: null });
    expect(s.print).toMatchObject({ paper: 'a4', size: 16, myNotes: true, numbers: true });
    expect(textArea('a5')).toMatchObject({ width: 112, height: 162 });
  });
});
