/** The chapter as semantic HTML: headings, paragraphs or one verse per line, notes at the end. */

import { h, withBreaks } from './dom.js';
import { compareOsis } from './reference.js';

function overlaps(start, end, a, b) {
  return compareOsis(start, b) <= 0 && compareOsis(end, a) >= 0;
}

/** Highlight colour, note and bookmark markers for each verse of the chapter. */
export function annotationsFor(verses, user) {
  const out = new Map();
  for (const v of verses) {
    const entry = { color: null, notes: [], bookmark: null };
    for (const hl of user.highlights) {
      if (overlaps(v.canon, v.canonEnd, hl.start, hl.end)) entry.color = hl.color;
    }
    for (const note of user.notes) {
      if (overlaps(v.canon, v.canonEnd, note.start, note.start)) entry.notes.push(note);
    }
    for (const mark of user.bookmarks) {
      if (overlaps(v.canon, v.canonEnd, mark.ref, mark.ref)) entry.bookmark = mark;
    }
    out.set(v.verse, entry);
  }
  return out;
}

function verseNode(v, settings, marks, noteNumbers) {
  const mark = marks.get(v.verse) || { color: null, notes: [], bookmark: null };
  const cls = ['verse'];
  if (mark.color) cls.push(`hl-${mark.color}`);
  const node = h(
    'span',
    { class: cls.join(' '), id: `v${v.verse}`, dataset: { verse: v.verse, canon: v.canon } },
    settings.verseNumbers ? h('sup', { class: 'vn' }, String(v.verse)) : null,
    h('span', { class: 'vt' }, withBreaks(v.text)),
  );
  for (const n of noteNumbers.get(v.verse) || []) {
    node.append(h('a', { class: 'fn-ref', href: `#fn${n}`, id: `fnref${n}`, role: 'doc-noteref', 'aria-label': `Note ${n}` }, String(n)));
  }
  if (mark.bookmark) {
    node.append(h('span', { class: 'marker bookmark', title: 'Bookmarked', 'aria-label': 'Bookmarked' }));
  }
  for (const note of mark.notes) {
    node.append(h('button', { class: 'marker note', title: 'Your note', 'aria-label': 'Open your note', dataset: { note: note.id } }));
  }
  return node;
}

function titleNode(t) {
  if (t.kind === 'summary') return h('p', { class: 'summary' }, t.text);
  if (t.kind === 'psalm') return h('p', { class: 'psalm-title' }, t.text);
  if (t.kind === 'major') return h('h2', { class: 'major' }, t.text);
  return h('h2', { class: 'section' }, t.text);
}

/** The chapter article, with the reader's annotations marked. */
export function renderChapter({ data, module, bookName, settings, user, videos = [] }) {
  const lang = module.language;
  const article = h('article', { class: `chapter layout-${settings.layout}`, lang, dir: module.direction || 'ltr' });
  const header = h('header', { class: 'chapter-head' }, h('h1', {}, `${bookName} ${data.chapter}`));
  if (videos.length) {
    header.append(
      h('button', { class: 'video-link', id: 'chapter-videos', title: 'Videos about this chapter', dataset: { count: videos.length } },
        `${videos.length} ${videos.length === 1 ? 'video' : 'videos'}`),
    );
  }
  article.append(header);

  const marks = annotationsFor(data.verses, user);
  const titles = new Map();
  for (const t of data.titles) {
    if (t.kind !== 'psalm' && !settings.titles) continue;
    const list = titles.get(t.verse) || [];
    list.push(t);
    titles.set(t.verse, list);
  }
  const noteNumbers = new Map();
  if (settings.sourceNotes) {
    data.notes.forEach((n, i) => {
      const list = noteNumbers.get(n.verse) || [];
      list.push(i + 1);
      noteNumbers.set(n.verse, list);
    });
  }

  let paragraph = null;
  for (const v of data.verses) {
    const before = titles.get(v.verse) || [];
    if (before.length) paragraph = null;
    for (const t of before) article.append(titleNode(t));
    const node = verseNode(v, settings, marks, noteNumbers);
    if (settings.layout === 'verses') {
      article.append(h('p', { class: 'line' }, node));
      continue;
    }
    if (!paragraph || v.paragraph) {
      paragraph = h('p', { class: 'para' });
      article.append(paragraph);
    } else {
      paragraph.append(' ');
    }
    paragraph.append(node);
  }

  if (settings.sourceNotes && data.notes.length) {
    const list = h('ol', { class: 'notes' });
    data.notes.forEach((n, i) => {
      list.append(
        h('li', { id: `fn${i + 1}`, role: 'doc-footnote' },
          h('a', { href: `#fnref${i + 1}`, class: 'fn-back', 'aria-label': `Back to verse ${n.verse}` }, String(n.verse)),
          ' ',
          n.marker ? h('b', {}, `${n.marker} `) : null,
          withBreaks(n.text)),
      );
    });
    article.append(h('section', { class: 'source-notes', role: 'doc-endnotes', 'aria-label': 'Notes of the edition' }, h('h2', {}, 'Notes'), list));
  }
  return article;
}

/** Translations side by side, one row per verse of the first. */
export function renderParallel({ data, modules, bookNames, settings }) {
  const table = h('table', { class: 'parallel' });
  const head = h('tr', {});
  modules.forEach((m, i) => {
    head.append(h('th', { scope: 'col', lang: m.language, dir: m.direction }, `${m.abbreviation} · ${bookNames[i]}`));
  });
  table.append(h('thead', {}, head));
  const body = h('tbody', {});
  for (const row of data.rows) {
    const tr = h('tr', { dataset: { canon: row.canon } });
    row.cells.forEach((cell, i) => {
      const m = modules[i];
      const td = h('td', { lang: m.language, dir: m.direction });
      for (const piece of cell) {
        const label = piece.chapter !== data.primary.chapter && i > 0 ? `${piece.chapter},${piece.verse}` : String(piece.verse);
        td.append(
          h('span', { class: 'verse', dataset: i === 0 ? { verse: piece.verse, canon: row.canon } : { } },
            settings.verseNumbers ? h('sup', { class: 'vn' }, label) : null,
            h('span', { class: 'vt' }, withBreaks(piece.text)), ' '),
        );
      }
      if (!cell.length && i > 0) td.append(h('span', { class: 'empty', 'aria-label': 'No corresponding verse' }, '·'));
      tr.append(td);
    });
    body.append(tr);
  }
  table.append(body);
  return h('div', { class: 'parallel-wrap' }, h('h1', { class: 'sr-only' }, `${bookNames[0]} ${data.primary.chapter}`), table);
}
