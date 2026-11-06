// @vitest-environment jsdom

import { describe, it, expect } from 'vitest';

import { copyHtml, copyText } from '../src/js/copy.js';
import { markTerms } from '../src/js/dom.js';
import { renderGuide } from '../src/js/guide.js';
import { libraryTabs, occurrencesCsv, shownCommentaries } from '../src/js/library.js';
import { shaper, shortParsing, transliterateGreek } from '../src/js/original.js';
import { renderMarkup, splitRange } from '../src/js/markup.js';
import { textArea } from '../src/js/print.js';
import { mergeNotes, noteMatches, notesMarkdown, parseNotesMarkdown } from '../src/js/study.js';
import { joinLabels, shiftDate, slotLabel } from '../src/js/liturgy.js';
import { doneInIcs, planDay, planIcs } from '../src/js/plans.js';
import { findPhrase, markPhrase } from '../src/js/phrase.js';
import { anchorsByVerse, topicsOf, videoTouches } from '../src/js/videos.js';
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

  it('shows the chosen commentaries side by side, or the first installed', () => {
    const installed = [{ abbreviation: 'HAYDOCK' }, { abbreviation: 'CATENA' }];
    expect(shownCommentaries(installed, { commentaries: [], commentary: null })).toEqual(['HAYDOCK']);
    expect(shownCommentaries(installed, { commentaries: [], commentary: 'CATENA' })).toEqual(['CATENA']);
    expect(shownCommentaries(installed, { commentaries: ['GONE', 'CATENA', 'HAYDOCK'], commentary: null })).toEqual(['CATENA', 'HAYDOCK']);
    expect(shownCommentaries([], { commentaries: ['CATENA'] })).toEqual([]);
  });

  it('keeps library and print settings within their choices', () => {
    const s = normalise({ library: { tab: 'books', commentary: 'MHC', dictionary: 3 }, print: { paper: 'tabloid', size: 40, myNotes: true } });
    expect(s.library).toEqual({ tab: 'books', commentary: 'MHC', commentaries: [], dictionary: null, book: null });
    expect(normalise({ library: { commentaries: ['HAY', 3, 'CATENA'] } }).library.commentaries).toEqual(['HAY', 'CATENA']);
    expect(s.print).toMatchObject({ paper: 'a4', size: 16, myNotes: true, numbers: true });
    expect(textArea('a5')).toMatchObject({ width: 112, height: 162 });
  });
});

describe('Greek and Hebrew', () => {
  it('transliterates Greek as the SBL Handbook does', () => {
    const words = ['ἠγάπησεν', 'ὁ', 'υἱὸν', 'οὗτος', 'ῥῆμα', 'ἄγγελος', 'Ἰησοῦ', 'τῷ', 'εὐαγγέλιον'];
    expect(words.map(transliterateGreek)).toEqual(['ēgapēsen', 'ho', 'huion', 'houtos', 'rhēma', 'angelos', 'Iēsou', 'tōi', 'euangelion']);
  });

  it('turns accents, points and cantillation off on request', () => {
    expect(shaper('grc', { greekAccents: false })('ἠγάπησεν ὁ θεὸς')).toBe('ηγαπησεν ο θεος');
    expect(shaper('hbo', { hebrewPoints: false })('בְּרֵאשִׁ֖ית בָּרָ֣א')).toBe('בראשית ברא');
    expect(shaper('hbo', { hebrewAccents: false })('בְּרֵאשִׁ֖ית')).toBe('בְּרֵאשִׁית');
    expect(shaper('la', { greekAccents: false })('Dóminus')).toBe('Dóminus');
  });

  it('writes the morphology short', () => {
    expect(shortParsing('V3AAI-S---')).toBe('verb aor act ind 3s');
    expect(shortParsing('V3p-I-SM-q', true)).toBe('verb qal perf 3ms');
    expect(shortParsing('N----ASM--')).toBe('noun acc ms');
    expect(shortParsing('')).toBe('');
  });

  it('exports the occurrences of a word as CSV for Grid', () => {
    const study = { lemma: 'ἀγαπάω', strong: 'G25', occurrences: [{ book: 'John', chapter: 3, verse: 16, text: 'ἠγάπησεν', parsing: 'verb, aorist' }] };
    expect(occurrencesCsv(study, (o) => `Gv ${o.chapter},${o.verse}`)).toBe(
      'Reference,Book,Chapter,Verse,Form,Parsing,Lemma,Strong\n"Gv 3,16",John,3,16,ἠγάπησεν,"verb, aorist",ἀγαπάω,G25\n');
  });

  it('shows the word tab only when a module has words', () => {
    expect(libraryTabs([{ kind: 'bible', words: true }]).map((t) => t.id)).toEqual(['passage', 'word', 'installed']);
  });
});

describe('notes that go to GRT Notes and come back', () => {
  const notes = () => [
    { id: 'n1', start: 'John.3.16', end: 'John.3.16', text: 'Love first.', tags: ['love'], links: ['n2'], quote: 'tanto amato', module: 'MAR' },
    { id: 'n2', start: 'Ps.23.1', end: 'Ps.23.2', text: '', tags: [], links: [], quote: '', module: '' },
  ];
  const entries = (list) => list.map((note, i) => ({ note, label: ['Gv 3,16', 'Sal 23,1-2'][i], text: i ? null : 'Dio infatti ha tanto amato' }));

  it('writes the id, the words, the tags and the links of each note', () => {
    const md = notesMarkdown(entries(notes()), { module: 'MAR', date: '2026-11-06' });
    expect(md).toContain('## Gv 3,16 (MAR)\n<!-- grt-bible n1 John.3.16 John.3.16 -->\n\n> Dio infatti ha tanto amato\n\n**“tanto amato”**\n\nLove first.\n\nTags: #love\n\nLinks: [[#Sal 23,1-2 (MAR)]]');
  });

  it('brings back edits, tags, links and notes deleted meanwhile', () => {
    const md = notesMarkdown(entries(notes()), { module: 'MAR', date: '2026-11-06' })
      .replace('Love first.', 'Love first,\nand always.')
      .replace('Tags: #love', 'Tags: #love #gospel')
      .replace('## Sal 23,1-2 (MAR)\n<!-- grt-bible n2 Ps.23.1 Ps.23.2 -->', '## Sal 23,1-2 (MAR)\n<!-- grt-bible n2 Ps.23.1 Ps.23.2 -->\n\nThe shepherd.\n\nLinks: [[#Gv 3,16 (MAR)]]');
    const parsed = parseNotesMarkdown(md);
    expect(parsed.map((b) => b.id)).toEqual(['n1', 'n2']);
    const mine = notes().slice(0, 1);
    expect(mergeNotes(mine, parsed)).toBe(2);
    expect(mine[0]).toMatchObject({ text: 'Love first,\nand always.', tags: ['love', 'gospel'], links: ['n2'], quote: 'tanto amato' });
    expect(mine[1]).toMatchObject({ id: 'n2', start: 'Ps.23.1', end: 'Ps.23.2', text: 'The shepherd.', links: ['n1'] });
    expect(mergeNotes(mine, parseNotesMarkdown(md))).toBe(0);
  });

  it('keeps headings and quotations written inside a note, and tells apart two notes on one verse', () => {
    const list = [
      { id: 'a', start: 'John.3.16', end: 'John.3.16', text: '## My heading\n\n> Augustine said so.', tags: [], links: ['b'], quote: '', module: '' },
      { id: 'b', start: 'John.3.16', end: 'John.3.16', text: 'Second.', tags: [], links: [], quote: '', module: '' },
    ];
    const md = notesMarkdown(list.map((note) => ({ note, label: 'Gv 3,16', text: 'Dio' })), { module: 'MAR', date: '2026-11-06' });
    expect(md).toContain('## Gv 3,16 (MAR) 2\n');
    expect(md).toContain('Links: [[#Gv 3,16 (MAR) 2]]');
    const parsed = parseNotesMarkdown(md);
    expect(parsed.map((b) => [b.id, b.text])).toEqual([['a', '## My heading\n\n> Augustine said so.'], ['b', 'Second.']]);
    const mine = structuredClone(list);
    expect(mergeNotes(mine, parsed)).toBe(0);
  });

  it('leaves out a note whose marker was spoilt by hand', () => {
    const parsed = parseNotesMarkdown('## Gv 3,16 (MAR)\n<!-- grt-bible n9 John.3 John.3.16 -->\n\nText.\n');
    const mine = [];
    expect(mergeNotes(mine, parsed)).toBe(0);
    expect(mine).toEqual([]);
  });

  it('filters by words, by the place and by #tag', () => {
    const [n] = notes();
    expect(noteMatches(n, 'Gv 3,16', 'love')).toBe(true);
    expect(noteMatches(n, 'Gv 3,16', '#love gv')).toBe(true);
    expect(noteMatches(n, 'Gv 3,16', '#hope')).toBe(false);
    expect(noteMatches(n, 'Gv 3,16', 'amato')).toBe(true);
  });
});

describe('words a note is about', () => {
  it('finds them whatever the case, accents and spaces', () => {
    const text = 'Dio infatti ha tanto  amato il mondo';
    expect(findPhrase(text, 'TANTO amato')).toEqual([15, 27]);
    expect(findPhrase('Perché Dio', 'perche')).toEqual([0, 6]);
    expect(findPhrase(text, 'odiato')).toBeNull();
  });

  it('marks them across elements', () => {
    const el = document.createElement('span');
    el.innerHTML = 'In principio<br>era il Verbo';
    expect(markPhrase(el, 'principio era')).toBe(true);
    expect([...el.querySelectorAll('mark')].map((m) => m.textContent)).toEqual(['principio', 'era']);
    expect(markPhrase(el, 'Logos')).toBe(false);
  });
});

describe('the liturgy and the reading plans', () => {
  it('names the readings and writes each reference once per book', () => {
    expect(slotLabel('first-reading')).toBe('First reading');
    expect(slotLabel('reading-3')).toBe('Reading 3');
    expect(slotLabel('psalm-2')).toBe('Psalm 2');
    expect(joinLabels(['Is 63,16-17', 'Is 63,19', 'Is 64,2-7', 'Sal 80,2'])).toBe('Is 63,16-17; 63,19; 64,2-7; Sal 80,2');
    expect(joinLabels(['1 Sam 1,24-28'])).toBe('1 Sam 1,24-28');
    expect(shiftDate('2026-12-31', 1)).toBe('2027-01-01');
    expect(shiftDate('2028-03-01', -1)).toBe('2028-02-29');
  });

  it('counts the days of a plan and sends them to Dates and back', () => {
    expect(planDay('2026-11-06', '2026-11-06')).toBe(1);
    expect(planDay('2026-11-06', '2027-11-05')).toBe(365);
    expect(planDay('2026-11-06', '2026-11-05')).toBe(0);
    const progress = { plan: 'gospels-40', start: '2026-12-31', done: [2] };
    const ics = planIcs('The four Gospels in 40 days', progress, ['Mt 1-2', 'Mt 3-4', 'Mt 5; Mc 1']);
    expect(ics).toContain('UID:grt-bible-gospels-40-2026-12-31-day3@grt\r\n');
    expect(ics).toContain('DUE;VALUE=DATE:20270102\r\n');
    expect(ics).toContain('SUMMARY:Read Mt 5\\; Mc 1\r\n');
    expect(ics.split('\r\n').every((line) => new TextEncoder().encode(line).length <= 75)).toBe(true);
    expect(doneInIcs(ics, progress)).toEqual([2]);
    // GRT Dates writes a ticked task with STATUS and PERCENT-COMPLETE, and may fold lines.
    const ticked = ics.replace('UID:grt-bible-gospels-40-2026-12-31-day3@grt\r\n', 'UID:grt-bible-gospels-40-2026-12-31-d\r\n ay3@grt\r\n')
      .replace(/(ay3@grt\r\n[\s\S]*?)STATUS:NEEDS-ACTION/, '$1STATUS:COMPLETED');
    expect(doneInIcs(ticked, progress)).toEqual([2, 3]);
    expect(doneInIcs(ics, { ...progress, start: '2026-01-01' })).toEqual([]);
  });
});

describe('video topics', () => {
  it('finds finer topics in the title', () => {
    expect(topicsOf({ title: 'Do Catholics Pray to the Saints?', tags: [] })).toEqual(['Mary and the saints', 'Prayer and spiritual life']);
    expect(topicsOf({ title: 'Why Liberals Love Ugly Art', tags: [] })).toEqual([]);
  });
});
