// @vitest-environment jsdom

/** Start the real interface against a backend kept in memory. */

import { describe, it, expect, beforeAll, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const indexHtml = readFileSync(join(here, '..', 'src', 'index.html'), 'utf8');
const wait = (ms = 30) => new Promise((resolve) => setTimeout(resolve, ms));

/** Two translations: AAA numbers psalms like the internal scheme, BBB one lower. */
function fakeBackend() {
  const modules = [
    { abbreviation: 'AAA', title: 'Alpha', language: 'it', direction: 'ltr', versification: 'hebrew', bundled: true,
      books: [{ index: 1, osis: 'Gen', name: 'Genesi', abbrev: 'Gen', chapters: 2 }, { index: 23, osis: 'Ps', name: 'Salmi', abbrev: 'Sal', chapters: 150 }, { index: 50, osis: 'John', name: 'Giovanni', abbrev: 'Gv', chapters: 21 }],
      summary: '', missingBooks: [] },
    { abbreviation: 'BBB', title: 'Beta', language: 'la', direction: 'ltr', versification: 'vulgate', bundled: true,
      books: [{ index: 1, osis: 'Gen', name: 'Genesis', abbrev: 'Gen', chapters: 2 }, { index: 17, osis: 'Tob', name: 'Tobiae', abbrev: 'Tob', chapters: 14 }, { index: 23, osis: 'Ps', name: 'Psalmi', abbrev: 'Ps', chapters: 150 }],
      summary: '', missingBooks: [] },
  ];
  modules.push(
    { kind: 'crossrefs', abbreviation: 'TSK', title: 'Treasury', language: 'en', direction: 'ltr', versification: 'english', bundled: false,
      books: [], summary: '3 groups of references', missingBooks: [], license: 'Public Domain', source: 'SWORD module TSK, version 1.4',
      fileName: 'TSK.grtb', origin: { file: 'TSK.grtb', installed: '2026-10-27' } },
    { kind: 'bible', abbreviation: 'SBL', title: 'Greek', language: 'grc', direction: 'ltr', versification: 'hebrew', bundled: false, words: true,
      books: [{ index: 50, osis: 'John', name: 'Ioannes', abbrev: 'Io', chapters: 21 }], summary: '', missingBooks: [], license: 'CC BY 4.0', source: '', fileName: 'SBL.grtb', origin: null },
    { kind: 'commentary', abbreviation: 'MHC', title: 'Matthew Henry', language: 'en', direction: 'ltr', versification: 'english', bundled: false,
      books: [], summary: '2 entries', missingBooks: [], license: 'Public Domain', source: '', fileName: 'MHC.grtb', origin: null },
    { kind: 'commentary', abbreviation: 'HAY', title: 'Haydock', language: 'en', direction: 'ltr', versification: 'hebrew', bundled: false,
      books: [], summary: '2 entries', missingBooks: [], license: 'Public Domain', source: '', fileName: 'HAY.grtb', origin: null },
    { kind: 'dictionary', abbreviation: 'EAS', title: 'Easton', language: 'en', direction: 'ltr', versification: 'english', bundled: false,
      books: [], summary: '2 entries', missingBooks: [], license: 'Public Domain', source: '', fileName: 'EAS.grtb', origin: null },
    { kind: 'apparatus', abbreviation: 'APP', title: 'Apparatus', language: 'grc', direction: 'ltr', versification: 'hebrew', bundled: false,
      books: [], summary: '2 entries', missingBooks: [], license: 'CC BY 4.0', source: '', fileName: 'APP.grtb', origin: { file: 'APP.grtb', installed: '2026-11-14' } },
    { kind: 'harmony', abbreviation: 'EUS', title: 'Eusebian Canons', language: 'en', direction: 'ltr', versification: 'hebrew', bundled: false,
      books: [], summary: '1 entries', missingBooks: [], license: 'Public domain', source: '', fileName: 'EUS.grtb', origin: null },
    { kind: 'atlas', abbreviation: 'ATL', title: 'Places and events', language: 'en', direction: 'ltr', versification: 'hebrew', bundled: false,
      books: [], summary: '3 entries', missingBooks: [], license: 'CC BY-SA 4.0', source: '', fileName: 'ATL.grtb',
      origin: { file: 'ATL.grtb', installed: '2026-11-14', seal: { state: 'signed', by: 'GRT Project', key: '4f4d552e5af3eacd3e63' } } },
  );
  const offset = (module, book) => (module === 'BBB' && book === 'Ps' ? 1 : 0);
  const calls = [];
  const files = {};
  const answers = [];
  const saved = { user: null, settings: null };
  const user = {
    reading: { current: { module: 'BBB', ref: 'Ps.23.1', offset: 0 }, history: [] },
    notes: [], highlights: [], bookmarks: [], saved_videos: [],
  };

  const chapter = ({ module, book, chapter: c }) => {
    const shift = offset(module, book);
    const verses = [1, 2, 3, 4].map((v) => ({
      verse: v,
      text: `${module} ${book} ${c}:${v} luce perché`,
      paragraph: v === 1,
      canon: `${book}.${c + shift}.${v}`,
      canonEnd: `${book}.${c + shift}.${v}`,
    }));
    const info = modules.find((m) => m.abbreviation === module);
    const own = info.books.find((b) => b.osis === book);
    const at = info.books.indexOf(own);
    return {
      module, book, chapter: c, verses,
      titles: c === 1 ? [{ chapter: c, verse: 1, kind: 'summary', text: 'Summary line' }] : [],
      notes: [{ chapter: c, verse: 2, marker: '', text: 'Edition note' }],
      prev: c > 1 ? { book, chapter: c - 1, verse: 1 } : at > 0 ? { book: info.books[at - 1].osis, chapter: 1, verse: 1 } : null,
      next: c < own.chapters ? { book, chapter: c + 1, verse: 1 } : null,
      canonFirst: verses[0].canon, canonLast: verses[3].canon, videos: [],
    };
  };

  const invoke = vi.fn(async (command, args = {}) => {
    calls.push({ command, args });
    switch (command) {
      case 'startup': return { version: '0.1.0', modules, settings: { videos: false }, user: structuredClone(user), videos: 0, notices: [], schemes: [] };
      case 'chapter': return chapter(args);
      case 'resolve': return args.refs.map((ref) => {
        const [book, c, v] = ref.split('.');
        const info = modules.find((m) => m.abbreviation === args.module);
        if (!info.books.some((b) => b.osis === book)) return null;
        return { book, chapter: Number(c) - offset(args.module, book), verse: Number(v) };
      });
      case 'verse_texts': return args.refs.map((r) => `text of ${r}`);
      case 'parallel': {
        const primary = chapter({ module: args.modules[0], book: args.book, chapter: args.chapter });
        return { modules: args.modules, primary, rows: primary.verses.map((v) => ({ canon: v.canon, cells: [[{ verse: v.verse, chapter: args.chapter, text: v.text }], [{ verse: v.verse, chapter: args.chapter + 1, text: 'other' }]] })) };
      }
      case 'search': return { total: 1, hits: [{ module: 'AAA', book: 'John', chapter: 3, verse: 16, text: 'Perché Dio ha tanto amato' }], terms: ['perche'] };
      case 'votd': return ['Ps.23.1', 'Ps.23.1'];
      case 'liturgy': return {
        date: args.date, season: 'advent', week: 1, sundayCycle: 'B', weekdayCycle: 'I', color: 'purple',
        celebration: { key: 'Advent1', en: 'First Sunday of Advent', it: 'I domenica di Avvento', grade: 'sunday', color: 'purple' },
        optional: [], optionalMasses: [],
        masses: [{ id: 'day', label: 'Mass during the Day', readings: [
          { slot: 'first-reading', ref: 'Is 63:16-17, 19; 64:2-7', osis: ['Isa.63.16-Isa.63.17', 'Isa.63.19-Isa.63.19', 'Isa.64.2-Isa.64.7'], alternatives: [] },
          { slot: 'gospel', ref: 'Mk 13:33-37', osis: ['Mark.13.33-Mark.13.37'], alternatives: [] },
        ] }],
      };
      case 'plans': return [{ id: 'gospels-40', title: 'The four Gospels in 40 days', days: 3 }];
      case 'plan_schedule': return [['Matt.1.1-Matt.2.23'], ['Matt.3.1-Matt.4.25'], ['Matt.5.1-Matt.5.48', 'Mark.1.1-Mark.1.45']];
      case 'read_text': return files[args.path] ?? '';
      case 'videos_list': return {
        channels: [
          { id: 'c1', name: 'First Channel', url: 'https://www.youtube.com/@first', videos: 30 },
          { id: 'c2', name: 'Second Channel', url: 'https://www.youtube.com/@second', videos: 15 },
        ],
        videos: Array.from({ length: 45 }, (_, i) => ({
          id: `v${i}`, title: `Video ${i}`, host: 'www.youtube.com', channel: i < 30 ? 'c1' : 'c2',
          category: i < 30 ? 'Theology' : i < 40 ? 'Doctrine' : 'Debates', source: '', language: 'en',
          durationS: 600, description: '', thumb: null, refs: [], tags: [],
        })),
      };
      case 'crossrefs': return [{ module: 'TSK', title: 'Treasury', groups: [{ label: 'beginning', targets: ['Ps.23.1-Ps.23.2', 'John.1.1'] }] }];
      case 'commentary': return [
        { id: 1, parent: null, key: '', title: '', body: '<p>Intro.</p>', spans: [`${args.at.split('.').slice(0, 2).join('.')}.0`] },
        { id: 2, parent: null, key: '', title: '', body: '<p>On <b>this</b> verse, see <a data-ref="John.1.1">John</a>.</p>', spans: [`${args.at}-${args.at}`] },
      ];
      case 'commentary_step': return 'Gen.2.4';
      case 'topics': return [{ module: 'EAS', title: 'Easton', entries: [{ id: 1, key: 'AARON', title: 'AARON', children: 0 }] }];
      case 'passage': return [{ book: 'Ps', chapter: 22, verse: 1, text: 'Dominus regit me', canon: 'Ps.23.1' }, { book: 'Ps', chapter: 22, verse: 2, text: 'in loco pascuae', canon: 'Ps.23.2' }];
      case 'lookup': {
        // The first, empty lookup answers last, as a slow backend may.
        if (!args.text) await wait(400);
        return [{ id: 1, key: 'AARON', title: 'AARON', children: 0 }, { id: 2, key: 'ABBA', title: 'ABBA', children: 0 }].filter((r) => r.key.startsWith(args.text.toUpperCase()));
      }
      case 'entry': return { id: args.id, parent: null, key: 'AARON', title: 'AARON', body: '<p>Brother of <a data-key="MOSES">Moses</a>.</p>', spans: [] };
      case 'entry_by_key': return { id: 9, parent: null, key: args.key, title: args.key, body: '<p>The lawgiver.</p>', spans: [] };
      case 'guide': return '# Installing modules\n\nStep by step.';
      case 'chapter_words': return [1, 2, 3, 4].flatMap((v) => [
        { verse: v, position: 1, text: 'ἠγάπησεν', lemma: 'ἀγαπάω', strong: 'G25', gloss: 'I love', xlit: '', features: 'V3AAI-S---', parsing: 'verb, aorist', original: false },
        { verse: v, position: 2, text: 'ὁ', lemma: 'ὁ', strong: 'G3588', gloss: 'the', xlit: '', features: 'T----NSM--', parsing: 'article', original: false },
      ]);
      case 'word_study': return {
        module: args.module, lemma: 'ἀγαπάω', strong: args.strong, gloss: 'I love', xlit: '', total: 2,
        books: [['John', 2]], forms: [['ἠγάπησεν', 2]],
        lexicon: [{ module: 'EAS', title: 'Easton', entry: { id: 1, parent: null, key: 'AGAPAO', title: 'AGAPAO', body: '<p>to love</p>', spans: [] } }],
        occurrences: [{ book: 'John', chapter: 3, verse: 16, text: 'ἠγάπησεν', parsing: 'verb, aorist' }],
      };
      case 'apparatus': return [{ module: 'APP', title: 'Apparatus', legend: '<p><i>WH</i> Westcott and Hort</p>', items: [
        { id: 2, parent: null, key: '', title: 'Gen.2.2', body: '<p><b>μονογενὴς θεὸς</b> <i>WH NA28</i> ] ὁ μονογενὴς υἱός <i>RP</i></p>', spans: ['Gen.2.2'] }] }];
      case 'harmony': return [{ module: 'EUS', title: 'Eusebian Canons', legend: '', items: [
        { id: 1, parent: null, key: '1.1', title: 'Canon I · Mt, Mk, Lk, Jn', body: '', spans: ['Matt.3.1-Matt.3.2', 'John.1.1'] }] }];
      case 'atlas_places': return [{ id: 5, key: 'Eden', title: 'Eden', lat: 31.5, lon: 35.5, count: 3 }, { id: 6, key: 'Havilah', title: 'Havilah', lat: 25, lon: 45, count: 1 }];
      case 'atlas_search': return [{ id: 6, key: 'Havilah', title: 'Havilah', lat: 25, lon: 45, count: 1 }];
      case 'atlas_timeline': return [
        { id: 7, title: 'Creation', first: -4003, last: -4003, span: 'Gen.1.1-Gen.2.3' },
        { id: 8, title: 'The Exodus', first: -1446, last: -1446, span: 'Exod.12.1-Exod.12.51' },
        { id: 9, title: 'Birth of Jesus', first: -4, last: -4, span: 'Luke.2.1-Luke.2.20' }];
      case 'atlas_shapes': return [['land', 'M0 0 L4000 0 L4000 3000 Z'], ['river', 'M3900 1800 L3950 1900']];
      case 'module_verify': return { state: 'unknown', key: 'abcdef0123456789abcd' };
      case 'plugin:dialog|save': case 'plugin:dialog|open': return answers.shift() ?? null;
      case 'plugin:dialog|ask': return true;
      case 'save_text': files[args.path] = args.text; return null;
      case 'user_save': saved.user = structuredClone(args.data); return null;
      case 'settings_save': saved.settings = structuredClone(args.settings); return null;
      default: return null;
    }
  });
  return { invoke, calls, saved, files, answers };
}

let backend;
let app;

async function type(input, text) {
  input.value = text;
  input.dispatchEvent(new Event('input', { bubbles: true }));
  await wait();
}

function key(target, name, extra = {}) {
  target.dispatchEvent(new KeyboardEvent('keydown', { key: name, bubbles: true, ...extra }));
}

beforeAll(async () => {
  document.body.innerHTML = /<body>([\s\S]*)<\/body>/.exec(indexHtml)[1].replace(/<script[\s\S]*?<\/script>/g, '');
  backend = fakeBackend();
  window.__GRT_TEST__ = true;
  window.__TAURI__ = { core: { invoke: backend.invoke } };
  Object.defineProperty(navigator, 'clipboard', { value: { writeText: vi.fn(async () => {}) }, configurable: true });
  app = await import('../src/js/main.js');
  await app.boot();
  await wait(50);
});

describe('the interface', () => {
  it('reopens the last chapter in the numbering of its translation', () => {
    expect(document.querySelector('#btn-location').textContent).toBe('Psalmi 22');
    expect(document.querySelector('#module-picker').value).toBe('BBB');
    const article = document.querySelector('#reader article');
    expect(article.getAttribute('lang')).toBe('la');
    expect(article.querySelectorAll('.verse')).toHaveLength(4);
  });

  it('moves between chapters with the arrow keys', async () => {
    key(document.body, 'ArrowRight');
    await wait();
    expect(document.querySelector('#btn-location').textContent).toBe('Psalmi 23');
    key(document.body, 'ArrowLeft');
    await wait();
    expect(document.querySelector('#btn-location').textContent).toBe('Psalmi 22');
  });

  it('goes to a typed reference and selects it', async () => {
    key(document.body, 'k', { ctrlKey: true });
    const input = document.querySelector('.palette-input');
    await type(input, 'Gen 2,3');
    key(input, 'Enter');
    await wait();
    expect(document.querySelector('#overlay').hidden).toBe(true);
    expect(document.querySelector('#btn-location').textContent).toBe('Genesis 2');
    expect(document.querySelector('.selection-label').textContent).toBe('Gen 2,3');
  });

  it('does not choose between two books on its own', async () => {
    key(document.body, 'k', { ctrlKey: true });
    const input = document.querySelector('.palette-input');
    await type(input, 'Gn 1');
    const labels = [...document.querySelectorAll('.palette-list li')].map((li) => li.textContent);
    expect(labels.some((l) => l.includes('Genesis'))).toBe(true);
    expect(labels.some((l) => l.includes('Ion 1') && l.includes('Not in this translation'))).toBe(true);
    key(input, 'Enter');
    await wait();
    expect(document.querySelector('#overlay').hidden).toBe(false);
    expect(document.querySelector('.palette-hint').textContent).toMatch(/Choose one/);
    key(document.body, 'Escape');
    await wait();
  });

  it('copies the selection with its reference', async () => {
    const copy = [...document.querySelectorAll('#selection-bar button')].find((b) => b.textContent === 'Copy');
    copy.click();
    await wait();
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('BBB Gen 2:3 luce perché\nGen 2,3 (BBB)');
  });

  it('saves a highlight under the internal reference', async () => {
    document.querySelector('.swatch.hl-yellow').click();
    await wait(1150);
    expect(backend.saved.user.highlights).toEqual([{ start: 'Gen.2.3', end: 'Gen.2.3', color: 'yellow' }]);
    expect(document.querySelector('.verse[data-verse="3"]').classList.contains('hl-yellow')).toBe(true);
  });

  it('opens a note for the selected verse', async () => {
    const note = [...document.querySelectorAll('#selection-bar button')].find((b) => b.textContent === 'Note');
    note.click();
    await wait(80);
    const area = document.querySelector('.note-text');
    expect(area).not.toBeNull();
    await type(area, 'Light before the sun.');
    await wait(1150);
    expect(backend.saved.user.notes[0]).toMatchObject({ start: 'Gen.2.3', text: 'Light before the sun.' });
    expect(document.querySelector('.verse[data-verse="3"] .marker.note')).not.toBeNull();
  });

  it('anchors a note to words of a verse, with tags and links', async () => {
    const vt = document.querySelector('#reader .verse[data-verse="4"] .vt');
    const range = document.createRange();
    range.setStart(vt.firstChild, 0);
    range.setEnd(vt.firstChild, 5);
    window.getSelection().removeAllRanges();
    window.getSelection().addRange(range);
    document.dispatchEvent(new Event('selectionchange'));
    await wait(400);
    expect(document.querySelector('#selection-bar .selection-label').textContent).toContain('“');
    [...document.querySelectorAll('#selection-bar button')].find((b) => b.textContent === 'Note').click();
    await wait(100);
    window.getSelection().removeAllRanges();
    const made = app.ctx.user().notes.at(-1);
    expect(made.quote).toBe(vt.textContent.slice(0, 5).trim());
    expect(document.querySelector('#reader .verse[data-verse="4"] mark.note-phrase')).not.toBeNull();
    const tags = [...document.querySelectorAll('.note-tags')].at(-1);
    tags.value = '#light creation';
    tags.dispatchEvent(new Event('change'));
    const linker = [...document.querySelectorAll('select[aria-label="Link to another note"]')].at(-1);
    linker.value = linker.options[1].value;
    linker.dispatchEvent(new Event('change'));
    await wait(100);
    const notes = app.ctx.user().notes;
    expect(notes.at(-1).tags).toEqual(['light', 'creation']);
    expect(notes.at(-1).links).toEqual([notes[0].id]);
    expect(document.querySelector(`#note-${notes[0].id}`).textContent).toContain('←');
    const filter = document.querySelector('input[aria-label="Search your notes"]');
    await type(filter, '#light');
    expect(document.querySelectorAll('.note-list .card')).toHaveLength(1);
    await type(filter, '');
  });

  it('shows the liturgy of the day and follows a reading plan', async () => {
    [...document.querySelectorAll('.tabs button')].find((b) => b.textContent === 'Today').click();
    await wait(150);
    const liturgy = document.querySelector('.liturgy');
    expect(liturgy.querySelector('h3').textContent).toBe('First Sunday of Advent');
    const refs = [...liturgy.querySelectorAll('.readings .ref')].map((b) => b.textContent);
    expect(refs[0]).toBe('Is 63,16-17; 63,19; 64,2-7');
    expect(backend.calls.filter((c) => c.command === 'liturgy').pop().args.date).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    liturgy.querySelector('button[aria-label="Next day"]').click();
    await wait(150);
    expect(document.querySelector('.liturgy-nav').textContent).toContain('Today');
    [...document.querySelectorAll('.tabs button')].find((b) => b.textContent === 'Plans').click();
    await wait(150);
    [...document.querySelectorAll('.plan-list button')].find((b) => b.textContent === 'Start today').click();
    await wait(150);
    const card = document.querySelector('.card.plan');
    expect(card.textContent).toContain('Day 1 of 3');
    card.querySelector('input[type="checkbox"]').click();
    await wait(150);
    expect(app.ctx.user().plans[0].done).toEqual([1]);
    expect(document.querySelector('.card.plan').textContent).toContain('1 of 3 days read');
  });

  it('shows the notes of the edition only when asked', async () => {
    expect(document.querySelector('.source-notes')).toBeNull();
    app.ctx.setSetting('sourceNotes', true);
    expect(document.querySelector('.source-notes').textContent).toContain('Edition note');
    app.ctx.setSetting('layout', 'verses');
    expect(document.querySelectorAll('#reader p.line')).toHaveLength(4);
  });

  it('greys out books a translation lacks', async () => {
    document.querySelector('#btn-location').click();
    await wait();
    const tobit = [...document.querySelectorAll('.book-list button')].find((b) => b.textContent === 'Tobiae');
    expect(tobit.classList.contains('missing')).toBe(false);
    key(document.body, 'Escape');
    document.querySelector('#module-picker').value = 'AAA';
    document.querySelector('#module-picker').dispatchEvent(new Event('change'));
    await wait(60);
    document.querySelector('#btn-location').click();
    await wait();
    const tobia = [...document.querySelectorAll('.book-list button')].find((b) => b.textContent === 'Tobia');
    expect(tobia.classList.contains('missing')).toBe(true);
    expect(tobia.getAttribute('aria-disabled')).toBe('true');
    key(document.body, 'Escape');
  });

  it('searches and marks the words found', async () => {
    document.querySelector('#btn-search').click();
    await wait();
    const input = document.querySelector('.search-input');
    input.value = 'perche';
    key(input, 'Enter');
    await wait(60);
    const result = document.querySelector('.result');
    expect(result.querySelector('.result-ref').textContent).toBe('Gv 3,16');
    expect(result.querySelector('mark').textContent).toBe('Perché');
  });

  it('lists videos by category and channel, a page at a time', async () => {
    app.ctx.setSetting('videos', true);
    expect(document.querySelector('#btn-videos').hidden).toBe(false);
    document.querySelector('#btn-videos').click();
    await wait(60);
    const panel = document.querySelector('#panel');
    expect(panel.querySelectorAll('.card.video')).toHaveLength(40);
    expect(panel.querySelector('.search-status').textContent).toBe('45 videos');
    panel.querySelector('.more').click();
    expect(panel.querySelectorAll('.card.video')).toHaveLength(45);
    const category = panel.querySelector('.search-form select[aria-label="Category"]');
    const channel = panel.querySelector('.search-form select[aria-label="Channel"]');
    category.value = 'Debates';
    category.dispatchEvent(new Event('change'));
    expect(panel.querySelector('.search-status').textContent).toBe('5 videos');
    expect(panel.querySelector('.card.video .muted').textContent).toBe('Second Channel · Debates · 10:00');
    category.value = '';
    channel.value = 'c1';
    channel.dispatchEvent(new Event('change'));
    expect(panel.querySelector('.search-status').textContent).toBe('30 videos');
    const openChannel = [...panel.querySelectorAll('.channels button')].find((b) => b.textContent === 'Open channel');
    openChannel.click();
    await wait();
    expect(backend.calls.some((c) => c.command === 'channel_open' && c.args.id === 'c1')).toBe(true);
  });

  it('shows the cross-references and the commentary of the selected verse', async () => {
    document.querySelector('#reader .verse[data-verse="2"]').click();
    await wait();
    const references = [...document.querySelectorAll('#selection-bar button')].find((b) => b.textContent === 'References');
    references.click();
    await wait(120);
    const panel = document.querySelector('#panel');
    expect(panel.querySelector('.panel-head h2').textContent).toBe('Library');
    expect([...panel.querySelectorAll('.tabs button')].map((b) => b.textContent)).toEqual(['This passage', 'Word', 'Dictionary', 'Map', 'Timeline', 'Installed']);
    const group = panel.querySelector('.group');
    expect(group.querySelector('b').textContent).toBe('beginning ');
    const refs = [...group.querySelectorAll('.ref')];
    expect(refs.map((b) => b.textContent)).toEqual(['Sal 23,1-2', 'Gv 1,1']);
    refs[0].click();
    await wait();
    expect(panel.querySelector('.preview').textContent).toContain('Dominus regit me');
    const asked = backend.calls.filter((c) => c.command === 'crossrefs').pop();
    expect(asked.args.at).toBe(backend.calls.filter((c) => c.command === 'commentary').pop().args.at);
    expect(panel.querySelector('details.entry-wrap summary').textContent).toMatch(/^Introduction to /);
    expect(panel.querySelector('.entry-wrap .entry b').textContent).toBe('this');
    expect(panel.querySelector('.topic-list button').textContent).toBe('AARON');
    const add = [...panel.querySelectorAll('select')].find((s) => s.getAttribute('aria-label') === 'Add a commentary beside');
    add.value = 'HAY';
    add.dispatchEvent(new Event('change'));
    await wait(500);
    expect([...panel.querySelectorAll('.commentary-column strong')].map((s) => s.textContent)).toEqual(['MHC', 'HAY']);
    expect(panel.classList.contains('wide')).toBe(true);
    expect(backend.calls.filter((c) => c.command === 'commentary').slice(-2).map((c) => c.args.module)).toEqual(['MHC', 'HAY']);
    panel.querySelector('.commentary-column button[aria-label="Close HAY"]').click();
    await wait(500);
    expect(panel.querySelectorAll('.commentary-column').length).toBe(1);
    expect(panel.classList.contains('wide')).toBe(false);
  });

  it('looks words up in a dictionary and follows its links', async () => {
    const panel = document.querySelector('#panel');
    // A verse selected just before leaving the passage tab must not redraw the dictionary.
    document.querySelector('#reader .verse[data-verse="3"]').click();
    [...panel.querySelectorAll('.tabs button')].find((b) => b.textContent === 'Dictionary').click();
    await wait(60);
    await type(panel.querySelector('input[type="search"]'), 'aa');
    await wait(500);
    expect([...panel.querySelectorAll('.headwords button')].map((b) => b.textContent)).toEqual(['AARON']);
    panel.querySelector('.headwords button').click();
    await wait();
    expect(panel.querySelector('.entry h3').textContent).toBe('AARON');
    panel.querySelector('.entry .key').click();
    await wait(250);
    expect(panel.querySelector('.entry').textContent).toContain('The lawgiver.');
  });

  it('lists what is installed, with licence and where it came from', async () => {
    const panel = document.querySelector('#panel');
    [...panel.querySelectorAll('.tabs button')].find((b) => b.textContent === 'Installed').click();
    await wait(60);
    const heads = [...panel.querySelectorAll('.kind-head')].map((h) => h.textContent);
    expect(heads).toEqual(['Bibles (3)', 'Commentaries (2)', 'Cross-references (1)', 'Dictionaries and lexicons (1)', 'Critical apparatuses (1)', 'Harmonies of the Gospels (1)', 'Atlases (1)']);
    expect(panel.querySelector('.card.module[data-module="ATL"] .seal').textContent).toBe('Signed by GRT Project, key 4f4d 552e 5af3 eacd.');
    const app2 = panel.querySelector('.card.module[data-module="APP"]');
    expect(app2.querySelector('.seal').textContent).toBe('Signature not checked yet.');
    [...app2.querySelectorAll('button')].find((b) => b.textContent === 'Check the signature').click();
    await wait(60);
    expect(app2.querySelector('.seal').textContent).toBe('Signed by a key this program does not know: abcd ef01 2345 6789.');
    const tsk = panel.querySelector('.card.module[data-module="TSK"]');
    expect(tsk.textContent).toContain('Licence: Public Domain');
    expect(tsk.textContent).toContain('Installed from TSK.grtb on 2026-10-27.');
    expect(panel.querySelector('.card.module[data-module="AAA"]').textContent).toContain('Comes with the program.');
    [...panel.querySelectorAll('button')].find((b) => b.textContent === 'How to find and install modules').click();
    await wait();
    expect(document.querySelector('.guide h2').textContent).toBe('Installing modules');
    key(document.body, 'Escape');
  });

  it('shows variants, parallels and places, and sends the passage to Graphs and to Paper', async () => {
    const panel = document.querySelector('#panel');
    [...panel.querySelectorAll('.tabs button')].find((b) => b.textContent === 'This passage').click();
    await wait(250);
    expect(panel.querySelector('.apparatus .entry b').textContent).toBe('μονογενὴς θεὸς');
    expect(panel.querySelector('.apparatus .entry').getAttribute('lang')).toBe('grc');
    expect(panel.querySelector('.apparatus .sigla summary').textContent).toBe('Sigla');
    const columns = panel.querySelectorAll('.parallel-column');
    expect(columns).toHaveLength(2);
    expect(columns[0].textContent).toContain('Dominus regit me');
    const places = [...panel.querySelectorAll('.lib-section')].find((section) => section.querySelector('h3')?.textContent === 'Places in this chapter');
    expect([...places.querySelectorAll('.topic-list button')].map((b) => b.textContent)).toEqual(['Eden', 'Havilah']);
    backend.answers.push('/out/refs.canvas');
    [...panel.querySelectorAll('button')].find((b) => b.textContent === 'Send to Graphs…').click();
    await wait(200);
    const canvas = JSON.parse(backend.files['/out/refs.canvas']);
    expect(canvas.nodes).toHaveLength(3);
    expect(canvas.edges.map((e) => [e.fromNode, e.label])).toEqual([['verse', 'beginning'], ['verse', 'beginning']]);
    [...panel.querySelectorAll('button')].find((b) => b.textContent === 'Study sheet…').click();
    await wait(300);
    const view = document.querySelector('#print-view');
    expect(view.querySelector('select').value).toBe('sheet');
    const pages = view.querySelector('.print-pages').textContent;
    for (const part of ['Gen 2,2-3 · AAA', 'Dominus regit me', 'My notes', 'Light before the sun.', 'Commentary · MHC', 'Variants', 'μονογενὴς θεὸς']) expect(pages).toContain(part);
    backend.answers.push('/out/sheet.fodt');
    [...view.querySelectorAll('button')].find((b) => b.textContent === 'Send to Paper…').click();
    await wait(100);
    const fodt = backend.files['/out/sheet.fodt'];
    expect(fodt).toContain('office:mimetype="application/vnd.oasis.opendocument.text"');
    expect(fodt).toContain('<text:h text:style-name="Heading_20_2" text:outline-level="2">Variants</text:h>');
    expect(fodt).toContain('<text:span text:style-name="Strong">1 </text:span>Dominus regit me');
    key(document.body, 'Escape');
  });

  it('puts the places of the chapter on a map and the events in order of time', async () => {
    const panel = document.querySelector('#panel');
    [...panel.querySelectorAll('.tabs button')].find((b) => b.textContent === 'Map').click();
    await wait(200);
    const map = panel.querySelector('svg.atlas-map');
    expect(map.querySelectorAll('.land')).toHaveLength(1);
    expect([...map.querySelectorAll('.place text')].map((t) => t.textContent)).toEqual(['Eden', 'Havilah']);
    expect(map.getAttribute('viewBox').split(' ').map(Number).every(Number.isFinite)).toBe(true);
    map.querySelector('.place').dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await wait(100);
    expect(panel.querySelector('.place-details h4').textContent).toBe('Eden');
    await type(panel.querySelector('input[aria-label="Find a place"]'), 'hav');
    await wait(60);
    [...panel.querySelectorAll('.headwords button')].find((b) => b.textContent === 'Havilah').click();
    await wait(100);
    expect(map.querySelectorAll('.place')).toHaveLength(1);
    expect(panel.querySelector('.place-details h4').textContent).toBe('Havilah');
    [...panel.querySelectorAll('.tabs button')].find((b) => b.textContent === 'Timeline').click();
    await wait(150);
    expect([...panel.querySelectorAll('.timeline .era')].map((e) => e.textContent)).toEqual(['4000s BC', '1000s BC', 'The first century BC']);
    expect([...panel.querySelectorAll('.timeline .event.here .link')].map((b) => b.textContent)).toEqual(['Creation']);
    expect(panel.querySelector('.timeline .event .when').textContent).toBe('4004 BC');
  });

  it('moves through tabs and verses from the keyboard, and gives every control a name', async () => {
    const panel = document.querySelector('#panel');
    const selected = () => panel.querySelector('[role="tab"][aria-selected="true"]').textContent;
    expect(selected()).toBe('Timeline');
    key(panel.querySelector('[role="tablist"]'), 'ArrowRight');
    await wait(100);
    expect(selected()).toBe('Installed');
    expect(document.activeElement.textContent).toBe('Installed');
    key(panel.querySelector('[role="tablist"]'), 'Home');
    await wait(200);
    expect(selected()).toBe('This passage');
    expect(document.querySelector('#btn-location').textContent).toContain('2');
    while (document.querySelector('#reader .verse.selected')) {
      key(document.body, 'Escape');
      await wait();
    }
    document.querySelector('#reader .verse[data-verse="1"]').click();
    await wait();
    key(document.body, 'ArrowDown');
    await wait();
    expect([...document.querySelectorAll('#reader .verse.selected')].map((v) => v.dataset.verse)).toEqual(['2']);
    key(document.body, 'ArrowDown', { shiftKey: true });
    await wait();
    expect([...document.querySelectorAll('#reader .verse.selected')].map((v) => v.dataset.verse)).toEqual(['2', '3']);
    expect(document.querySelector('.skip-link').getAttribute('href')).toBe('#reader');
    const nameless = [];
    const check = (where) => {
      for (const el of document.querySelectorAll('button, input, select, textarea, [role="button"]')) {
        if (el.closest('[hidden]') || el.type === 'hidden') continue;
        const labelled = el.getAttribute('aria-label') || el.getAttribute('title') || el.labels?.length || el.closest('label');
        const name = ['BUTTON', 'G'].includes(el.tagName.toUpperCase()) || el.getAttribute('role') === 'button' ? el.textContent.trim() || labelled : labelled;
        if (!name) nameless.push(`${where}: ${el.outerHTML.slice(0, 80)}`);
      }
    };
    for (const tab of ['This passage', 'Word', 'Dictionary', 'Map', 'Timeline', 'Installed']) {
      [...panel.querySelectorAll('.tabs button')].find((b) => b.textContent === tab).click();
      await wait(200);
      check(`Library ${tab}`);
    }
    document.querySelector('#btn-study').click();
    await wait(100);
    for (const tab of ['Today', 'Notes', 'Plans', 'Highlights', 'Bookmarks', 'History']) {
      [...document.querySelectorAll('#panel .tabs button')].find((b) => b.textContent === tab).click();
      await wait(150);
      check(`Study ${tab}`);
    }
    expect(nameless).toEqual([]);
  });

  it('prints the chapter, the selection or the notes on fixed pages', async () => {
    key(document.body, 'p', { ctrlKey: true });
    await wait(60);
    const view = document.querySelector('#print-view');
    expect(view).not.toBeNull();
    const what = view.querySelector('select');
    expect([...what.options].map((o) => o.value)).toEqual(['selection', 'chapter', 'sheet', 'notes']);
    expect(view.querySelectorAll('.print-page').length).toBeGreaterThan(0);
    expect(view.querySelector('.print-head').textContent).toBe('Genesi 2AAA');
    what.value = 'notes';
    what.dispatchEvent(new Event('change'));
    await wait(60);
    expect(view.querySelector('.print-page').textContent).toContain('Light before the sun.');
    key(document.body, 'Escape');
    expect(document.querySelector('#print-view')).toBeNull();
  });

  it('links a video to the selected verses and keeps it in a list', async () => {
    const user = app.ctx.user();
    user.video_lists.push({ id: 'l1', name: 'Psalms', videos: [] });
    document.querySelector('#btn-videos').click();
    await wait(80);
    const panel = document.querySelector('#panel');
    const first = panel.querySelector('.card.video');
    const link = [...first.querySelectorAll('button')].find((b) => b.textContent.startsWith('Link to'));
    link.click();
    await wait(80);
    expect(user.video_anchors).toHaveLength(1);
    expect(user.video_anchors[0]).toMatchObject({ video: 'v0' });
    expect(document.querySelector('#reader .marker.video')).not.toBeNull();
    const picker = panel.querySelector('.card.video select') || document.querySelector('#panel .card.video select');
    picker.value = 'l1';
    picker.dispatchEvent(new Event('change'));
    expect(user.video_lists[0].videos).toEqual(['v0']);
  });

  it('opens a translation that lacks the book at its first book', async () => {
    document.querySelector('#module-picker').value = 'SBL';
    document.querySelector('#module-picker').dispatchEvent(new Event('change'));
    await wait(80);
    expect(document.querySelector('#module-picker').value).toBe('SBL');
    expect(document.querySelector('#btn-location').textContent).toBe('Ioannes 1');
    expect(document.querySelector('#toast').textContent).toMatch(/does not contain/);
  });

  it('reads a chapter word by word and studies a word', async () => {
    document.querySelector('#module-picker').value = 'SBL';
    document.querySelector('#module-picker').dispatchEvent(new Event('change'));
    await wait(80);
    const words = document.querySelector('#btn-words');
    expect(words.hidden).toBe(false);
    words.click();
    await wait(80);
    const cell = document.querySelector('#reader .iword');
    expect(cell.querySelector('.w-text').textContent).toBe('ἠγάπησεν');
    expect(cell.querySelector('.w-xlit').textContent).toBe('ēgapēsen');
    expect(cell.querySelector('.w-morph').textContent).toBe('verb aor act ind 3s');
    cell.click();
    await wait(80);
    const panel = document.querySelector('#panel');
    expect(panel.querySelector('.word-lemma').textContent).toBe('ἀγαπάω');
    expect(panel.textContent).toContain('2 occurrences in SBL');
    expect(panel.querySelector('.entry').textContent).toBe('to love');
    [...panel.querySelectorAll('button')].find((b) => b.textContent === 'Find every occurrence').click();
    await wait(80);
    expect(backend.calls.filter((c) => c.command === 'search').pop().args.query).toBe('G25');
    expect(document.querySelectorAll('#reader .iword').length).toBe(8);
    words.click();
    await wait(80);
    expect(document.querySelector('#reader .iword')).toBeNull();
    document.querySelector('#module-picker').value = 'AAA';
    document.querySelector('#module-picker').dispatchEvent(new Event('change'));
    await wait(80);
  });

  it('searches within results and keeps a search', async () => {
    document.querySelector('#btn-search').click();
    await wait();
    if (!document.querySelector('.search-input')) document.querySelector('#btn-search').click();
    await wait();
    const input = document.querySelector('.search-input');
    input.value = 'luce';
    key(input, 'Enter');
    await wait(60);
    const within = [...document.querySelectorAll('.search-form .check')].find((l) => l.textContent.includes('Within'));
    within.querySelector('input').checked = true;
    input.value = 'perche';
    key(input, 'Enter');
    await wait(60);
    expect(backend.calls.filter((c) => c.command === 'search').pop().args.query).toBe('(luce) AND (perche)');
    [...document.querySelectorAll('.search-form button')].find((b) => b.textContent === 'Save').click();
    await wait();
    expect(app.ctx.user().saved_searches.map((x) => x.query)).toEqual(['(luce) AND (perche)']);
    expect(document.querySelector('.saved-searches').textContent).toContain('(luce) AND (perche)');
  });

  it('keeps preferences between sessions', () => {
    return wait(500).then(() => {
      expect(backend.saved.settings.layout).toBe('verses');
      expect(backend.saved.settings.module).toBe('AAA');
    });
  });
});
