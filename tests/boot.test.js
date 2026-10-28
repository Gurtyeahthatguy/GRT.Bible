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
    { kind: 'commentary', abbreviation: 'MHC', title: 'Matthew Henry', language: 'en', direction: 'ltr', versification: 'english', bundled: false,
      books: [], summary: '2 entries', missingBooks: [], license: 'Public Domain', source: '', fileName: 'MHC.grtb', origin: null },
    { kind: 'dictionary', abbreviation: 'EAS', title: 'Easton', language: 'en', direction: 'ltr', versification: 'english', bundled: false,
      books: [], summary: '2 entries', missingBooks: [], license: 'Public Domain', source: '', fileName: 'EAS.grtb', origin: null },
  );
  const offset = (module, book) => (module === 'BBB' && book === 'Ps' ? 1 : 0);
  const calls = [];
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
      case 'passage': return [{ book: 'Ps', chapter: 22, verse: 1, text: 'Dominus regit me', canon: 'Ps.23.1' }, { book: 'Ps', chapter: 22, verse: 2, text: 'in loco pascuae', canon: 'Ps.23.2' }];
      case 'lookup': return [{ id: 1, key: 'AARON', title: 'AARON', children: 0 }, { id: 2, key: 'ABBA', title: 'ABBA', children: 0 }].filter((r) => r.key.startsWith(args.text.toUpperCase()));
      case 'entry': return { id: args.id, parent: null, key: 'AARON', title: 'AARON', body: '<p>Brother of <a data-key="MOSES">Moses</a>.</p>', spans: [] };
      case 'entry_by_key': return { id: 9, parent: null, key: args.key, title: args.key, body: '<p>The lawgiver.</p>', spans: [] };
      case 'guide': return '# Installing modules\n\nStep by step.';
      case 'user_save': saved.user = structuredClone(args.data); return null;
      case 'settings_save': saved.settings = structuredClone(args.settings); return null;
      default: return null;
    }
  });
  return { invoke, calls, saved };
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
    const [category, channel] = panel.querySelectorAll('.search-form select');
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
    expect([...panel.querySelectorAll('.tabs button')].map((b) => b.textContent)).toEqual(['This passage', 'Dictionary', 'Installed']);
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
  });

  it('looks words up in a dictionary and follows its links', async () => {
    const panel = document.querySelector('#panel');
    [...panel.querySelectorAll('.tabs button')].find((b) => b.textContent === 'Dictionary').click();
    await wait(60);
    await type(panel.querySelector('input[type="search"]'), 'aa');
    await wait(200);
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
    expect(heads).toEqual(['Bibles (2)', 'Commentaries (1)', 'Cross-references (1)', 'Dictionaries and lexicons (1)']);
    const tsk = panel.querySelector('.card.module[data-module="TSK"]');
    expect(tsk.textContent).toContain('Licence: Public Domain');
    expect(tsk.textContent).toContain('Installed from TSK.grtb on 2026-10-27.');
    expect(panel.querySelector('.card.module[data-module="AAA"]').textContent).toContain('Comes with the program.');
    [...panel.querySelectorAll('button')].find((b) => b.textContent === 'How to find and install modules').click();
    await wait();
    expect(document.querySelector('.guide h2').textContent).toBe('Installing modules');
    key(document.body, 'Escape');
  });

  it('prints the chapter, the selection or the notes on fixed pages', async () => {
    key(document.body, 'p', { ctrlKey: true });
    await wait(60);
    const view = document.querySelector('#print-view');
    expect(view).not.toBeNull();
    const what = view.querySelector('select');
    expect([...what.options].map((o) => o.value)).toEqual(['selection', 'chapter', 'notes']);
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

  it('keeps preferences between sessions', () => {
    return wait(500).then(() => {
      expect(backend.saved.settings.layout).toBe('verses');
      expect(backend.saved.settings.module).toBe('AAA');
    });
  });
});
