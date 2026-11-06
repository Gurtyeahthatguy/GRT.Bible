/** The reader's own material: verse of the day, notes, highlights, bookmarks and history. */

import { api, pickFile, pickSave } from './api.js';
import { clear, h } from './dom.js';
import { liturgySection } from './liturgy.js';
import { findPhrase } from './phrase.js';
import { renderPlans } from './plans.js';
import { toast } from './overlay.js';
import { compareOsis } from './reference.js';
import { HIGHLIGHT_COLORS } from './settings.js';

export const STUDY_TABS = [
  { id: 'today', label: 'Today' },
  { id: 'notes', label: 'Notes' },
  { id: 'plans', label: 'Plans' },
  { id: 'highlights', label: 'Highlights' },
  { id: 'bookmarks', label: 'Bookmarks' },
  { id: 'history', label: 'History' },
];

export function dayOfYear(date = new Date()) {
  const start = new Date(date.getFullYear(), 0, 1);
  return Math.floor((date - start) / 86_400_000) + 1;
}

/** Labels and texts of internal references in the current translation. */
async function describe(ctx, refs) {
  const module = ctx.module();
  if (!refs.length) return [];
  const [places, texts] = await Promise.all([api.resolve(module.abbreviation, refs), api.verseTexts(module.abbreviation, refs)]);
  return refs.map((ref, i) => ({ ref, place: places[i], text: texts[i], label: places[i] ? ctx.placeLabel(places[i]) : ref }));
}

function empty(text) {
  return h('p', { class: 'empty-state' }, text);
}

async function today(container, ctx) {
  try {
    container.append(await liturgySection(ctx));
  } catch (error) {
    container.append(empty(String(error)));
  }
  const [start, end] = await api.votd(dayOfYear());
  const module = ctx.module();
  const [first, last] = await api.resolve(module.abbreviation, [start, end]);
  if (!first) {
    container.append(empty(`Today's passage is not in ${module.abbreviation}.`));
    return;
  }
  const chapter = await api.chapter(module.abbreviation, first.book, first.chapter);
  const lastVerse = last && last.chapter === first.chapter ? last.verse : first.verse;
  const verses = chapter.verses.filter((v) => v.verse >= first.verse && v.verse <= lastVerse);
  const label = ctx.placeLabel(first, lastVerse);
  container.append(
    h('section', { class: 'votd', lang: module.language, dir: module.direction },
      h('h3', {}, 'Verse of the day'),
      h('blockquote', {}, verses.map((v) => v.text.replace(/\n/g, ' ')).join(' ')),
      h('p', { class: 'votd-ref' }, `${label} (${module.abbreviation})`),
      h('button', { onclick: () => ctx.openPlace(module.abbreviation, first.book, first.chapter, first.verse, lastVerse) }, 'Read the chapter')),
  );
}

const MARKER = /^<!-- grt-bible (\S+) (\S+) (\S+) -->$/;

/** Notes as one Markdown file for GRT Notes, each with a marker that brings it back. */
export function notesMarkdown(entries, { module, date }) {
  const lines = ['---', 'title: Notes from GRT Bible', 'tags:', '  - bible', `created: ${date}`, '---', '', '# Notes from GRT Bible', ''];
  // Two notes on one verse get distinct headings.
  const used = new Map();
  const headings = entries.map(({ label }) => {
    const base = `${label} (${module})`;
    const n = (used.get(base) || 0) + 1;
    used.set(base, n);
    return n > 1 ? `${base} ${n}` : base;
  });
  const heading = new Map(entries.map(({ note }, i) => [note.id, headings[i]]));
  for (const [i, { text, note }] of entries.entries()) {
    lines.push(`## ${headings[i]}`);
    if (note.id && note.start) lines.push(`<!-- grt-bible ${note.id} ${note.start} ${note.end || note.start} -->`);
    lines.push('');
    if (text) lines.push(...text.split('\n').map((l) => `> ${l}`), '');
    if (note.quote) lines.push(`**“${note.quote}”**`, '');
    const body = (note.text || '').trim();
    if (body) lines.push(body, '');
    if (note.tags?.length) lines.push(`Tags: ${note.tags.map((t) => `#${t}`).join(' ')}`, '');
    const links = (note.links || []).filter((id) => heading.has(id));
    if (links.length) lines.push(`Links: ${links.map((id) => `[[#${heading.get(id)}]]`).join(' ')}`, '');
  }
  return `${lines.join('\n').trimEnd()}\n`;
}

/** The notes of a file written by notesMarkdown, after any editing: text, tags and links by heading. */
export function parseNotesMarkdown(markdown) {
  const blocks = [];
  let current = null;
  const lines = markdown.replace(/\r\n?/g, '\n').split('\n');
  for (const [i, line] of lines.entries()) {
    const head = /^## (.+)$/.exec(line);
    // A heading followed by a marker starts a note; any other heading is text of the note.
    if (head && (MARKER.test((lines[i + 1] || '').trim()) || !current || !current.id)) {
      current = { heading: head[1].trim(), id: null, start: null, end: null, quote: '', lines: [], tags: [], links: [] };
      blocks.push(current);
      continue;
    }
    if (!current) continue;
    const marker = MARKER.exec(line.trim());
    if (marker) {
      [, current.id, current.start, current.end] = marker;
      continue;
    }
    // The verse quoted under the heading.
    if (line.startsWith('> ') && !current.lines.some((l) => l.trim())) continue;
    const quote = /^\*\*“(.*)”\*\*$/.exec(line.trim());
    if (quote) {
      current.quote = quote[1];
      continue;
    }
    if (/^Tags:\s/.test(line)) {
      current.tags = [...line.matchAll(/#([^\s#]+)/g)].map((m) => m[1]);
      continue;
    }
    if (/^Links:\s/.test(line)) {
      current.links = [...line.matchAll(/\[\[#([^\]]+)\]\]/g)].map((m) => m[1].trim());
      continue;
    }
    current.lines.push(line);
  }
  return blocks.map((b) => ({
    heading: b.heading, id: b.id, start: b.start, end: b.end, quote: b.quote, tags: b.tags, links: b.links,
    text: b.lines.join('\n').trim(),
  }));
}

const OSIS_VERSE = /^[1-4]?[A-Za-z]+\.\d+\.\d+$/;

/** Folds the notes of a file into the reader's: edits replace, notes deleted here come back; returns how many changed. */
export function mergeNotes(notes, parsed) {
  const byHeading = new Map(parsed.filter((b) => b.id).map((b) => [b.heading, b.id]));
  let changed = 0;
  for (const block of parsed) {
    // A spoilt marker is left out.
    if (!block.id || !OSIS_VERSE.test(block.start || '') || (block.end && !OSIS_VERSE.test(block.end))) continue;
    const links = block.links.map((h) => byHeading.get(h)).filter(Boolean);
    const existing = notes.find((n) => n.id === block.id);
    if (existing) {
      const same = existing.text === block.text && existing.tags.join(' ') === block.tags.join(' ') && existing.links.join(' ') === links.join(' ');
      if (!same) {
        Object.assign(existing, { text: block.text, tags: block.tags, links });
        changed++;
      }
    } else {
      notes.push({ id: block.id, start: block.start, end: block.end || block.start, text: block.text, tags: block.tags, links, quote: block.quote, module: '' });
      changed++;
    }
  }
  return changed;
}

function isoDate() {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

/** Whether a note matches what the reader typed in the filter: words of the text, the quote, the place or #tags. */
export function noteMatches(note, label, query) {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  const haystack = `${note.text} ${note.quote || ''} ${label}`.toLowerCase();
  return words.every((w) => (w.startsWith('#') ? (note.tags || []).some((t) => t.toLowerCase() === w.slice(1)) : haystack.includes(w)));
}

let noteFilter = '';

function tagList(tags) {
  return tags.map((t) => t.trim().replace(/^#/, '').replace(/\s+/g, '-')).filter(Boolean).filter((t, i, all) => all.indexOf(t) === i);
}

async function notes(container, ctx) {
  const user = ctx.user();
  if (!user.notes.length) {
    container.append(empty('No notes yet. Select a verse, or some words in it, and choose Note.'));
    container.append(h('div', { class: 'row' }, importButton(ctx)));
    return;
  }
  const sorted = [...user.notes].sort((a, b) => compareOsis(a.start, b.start));
  const described = await describe(ctx, sorted.map((n) => n.start));
  const labelOf = new Map(sorted.map((n, i) => [n.id, described[i].label]));
  const module = ctx.module();
  container.append(h('div', { class: 'row' },
    h('button', {
      onclick: async () => {
        const path = await pickSave('Export your notes', 'GRT Bible notes.md', [{ name: 'Markdown', extensions: ['md'] }]);
        if (!path) return;
        try {
          const entries = sorted.map((note, i) => ({ note, label: described[i].label, text: described[i].text }));
          await api.saveText(path, notesMarkdown(entries, { module: module.abbreviation, date: isoDate() }));
          toast('Your notes were exported as Markdown. Edit them in GRT Notes and bring them back with Import.');
        } catch (error) {
          toast(String(error), 5000);
        }
      },
    }, 'Export as Markdown…'),
    importButton(ctx),
    h('button', { class: 'quiet', onclick: () => ctx.print('notes') }, 'Print…')));
  const tags = [...new Set(user.notes.flatMap((n) => n.tags || []))].sort();
  const filter = h('input', { type: 'search', placeholder: 'Search your notes, or #tag', 'aria-label': 'Search your notes', value: noteFilter });
  const list = h('div', { class: 'note-list' });
  container.append(filter, tags.length ? h('p', { class: 'tags' }, tags.map((t) => h('button', {
    type: 'button',
    class: 'tag',
    onclick: () => {
      filter.value = `#${t}`;
      filter.dispatchEvent(new Event('input'));
    },
  }, `#${t}`))) : null, list);
  const draw = () => {
    clear(list);
    const shown = sorted.map((note, i) => [note, described[i]]).filter(([note, d]) => noteMatches(note, d.label, noteFilter));
    if (!shown.length) list.append(empty('No note matches.'));
    for (const [note, d] of shown) list.append(noteCard(ctx, note, d, sorted, labelOf));
  };
  filter.addEventListener('input', () => {
    noteFilter = filter.value;
    draw();
  });
  draw();
}

function importButton(ctx) {
  return h('button', {
    class: 'quiet',
    title: 'Bring back a Markdown file of notes exported earlier and edited, for example in GRT Notes',
    onclick: async () => {
      const path = await pickFile('Import notes', [{ name: 'Markdown', extensions: ['md', 'markdown', 'txt'] }]);
      if (!path) return;
      try {
        const parsed = parseNotesMarkdown(await api.readText(path));
        if (!parsed.some((b) => b.id)) {
          toast('This file has no notes exported by GRT Bible.', 5000);
          return;
        }
        const changed = mergeNotes(ctx.user().notes, parsed);
        ctx.saveUser();
        ctx.refresh();
        toast(changed ? `${changed} notes brought back.` : 'Nothing had changed.');
      } catch (error) {
        toast(String(error), 5000);
      }
    },
  }, 'Import…');
}

function noteCard(ctx, note, d, all, labelOf) {
  const user = ctx.user();
  const module = ctx.module();
  const area = h('textarea', { class: 'note-text', rows: 4, 'aria-label': `Note on ${d.label}`, dataset: { note: note.id } });
  area.value = note.text;
  area.addEventListener('input', () => {
    note.text = area.value;
    ctx.saveUser();
  });
  const tags = h('input', { type: 'text', class: 'note-tags', placeholder: 'Tags, separated by spaces', 'aria-label': `Tags of the note on ${d.label}`, value: (note.tags || []).map((t) => `#${t}`).join(' ') });
  tags.addEventListener('change', () => {
    note.tags = tagList(tags.value.split(/[\s,]+/));
    ctx.saveUser();
  });
  let phrase = null;
  if (note.quote) {
    const here = d.text && findPhrase(d.text, note.quote);
    phrase = h('p', { class: 'note-quote' }, `“${note.quote}”`,
      here ? null : h('span', { class: 'fine' }, ` · not in ${module.abbreviation}: the note stays on the verse`));
  }
  const others = all.filter((n) => n.id !== note.id);
  const backlinks = all.filter((n) => (n.links || []).includes(note.id));
  const jump = (id) => document.getElementById(`note-${id}`)?.scrollIntoView?.({ block: 'center' });
  const links = h('p', { class: 'tags' },
    (note.links || []).filter((id) => labelOf.has(id)).map((id) => h('span', { class: 'tag' },
      h('button', { class: 'link', onclick: () => jump(id) }, `→ ${labelOf.get(id)}`),
      h('button', {
        class: 'link',
        'aria-label': 'Remove this link',
        onclick: () => {
          note.links = note.links.filter((x) => x !== id);
          ctx.saveUser();
          ctx.refresh();
        },
      }, ' ×'))),
    backlinks.map((n) => h('span', { class: 'tag' }, h('button', { class: 'link', onclick: () => jump(n.id) }, `← ${labelOf.get(n.id)}`))));
  const linker = others.length ? h('select', { 'aria-label': 'Link to another note' },
    h('option', { value: '' }, 'Link to another note…'),
    others.filter((n) => !(note.links || []).includes(n.id)).map((n) => h('option', { value: n.id }, `${labelOf.get(n.id)}${n.text ? ` · ${n.text.slice(0, 30)}` : ''}`))) : null;
  linker?.addEventListener('change', () => {
    if (!linker.value) return;
    note.links = [...(note.links || []), linker.value];
    ctx.saveUser();
    ctx.refresh();
  });
  return h('article', { class: 'card', id: `note-${note.id}` },
    h('header', {},
      h('button', { class: 'link', onclick: () => d.place && ctx.openPlace(ctx.module().abbreviation, d.place.book, d.place.chapter, d.place.verse) }, d.label),
      h('button', {
        class: 'quiet danger',
        'aria-label': `Delete note on ${d.label}`,
        onclick: () => {
          user.notes = user.notes.filter((n) => n.id !== note.id);
          for (const n of user.notes) n.links = (n.links || []).filter((id) => id !== note.id);
          ctx.saveUser();
          ctx.refresh();
        },
      }, 'Delete')),
    d.text ? h('p', { class: 'quote' }, d.text.replace(/\n/g, ' ')) : null,
    phrase,
    area,
    tags,
    links.childNodes.length ? links : null,
    linker);
}

async function highlights(container, ctx) {
  const user = ctx.user();
  if (!user.highlights.length) {
    container.append(empty('Nothing highlighted yet.'));
    return;
  }
  const sorted = [...user.highlights].sort((a, b) => compareOsis(a.start, b.start));
  const described = await describe(ctx, sorted.map((x) => x.start));
  for (const [i, hl] of sorted.entries()) {
    const d = described[i];
    container.append(
      h('article', { class: `card hl-card hl-${HIGHLIGHT_COLORS.includes(hl.color) ? hl.color : 'yellow'}` },
        h('header', {},
          h('button', { class: 'link', onclick: () => d.place && ctx.openPlace(ctx.module().abbreviation, d.place.book, d.place.chapter, d.place.verse) }, d.label),
          h('button', {
            class: 'quiet danger',
            onclick: () => {
              user.highlights = user.highlights.filter((x) => x !== hl);
              ctx.saveUser();
              ctx.refresh();
            },
          }, 'Remove')),
        d.text ? h('p', { class: 'quote' }, d.text.replace(/\n/g, ' ')) : null),
    );
  }
}

async function bookmarks(container, ctx) {
  const user = ctx.user();
  if (!user.bookmarks.length) {
    container.append(empty('No bookmarks yet.'));
    return;
  }
  const described = await describe(ctx, user.bookmarks.map((b) => b.ref));
  for (const [i, mark] of user.bookmarks.entries()) {
    const d = described[i];
    container.append(
      h('article', { class: 'card' },
        h('header', {},
          h('button', { class: 'link', onclick: () => d.place && ctx.openPlace(ctx.module().abbreviation, d.place.book, d.place.chapter, d.place.verse) }, mark.label || d.label),
          h('button', {
            class: 'quiet danger',
            onclick: () => {
              user.bookmarks = user.bookmarks.filter((b) => b.id !== mark.id);
              ctx.saveUser();
              ctx.refresh();
            },
          }, 'Remove')),
        d.text ? h('p', { class: 'quote' }, d.text.replace(/\n/g, ' ')) : null),
    );
  }
}

async function history(container, ctx) {
  const entries = ctx.user().reading.history || [];
  if (!entries.length) {
    container.append(empty('Chapters you read will be listed here.'));
    return;
  }
  const list = h('ol', { class: 'history' });
  const known = new Set(ctx.modules().map((m) => m.abbreviation));
  const groups = new Map();
  for (const entry of entries) {
    const module = known.has(entry.module) ? entry.module : ctx.module().abbreviation;
    const refs = groups.get(module) || [];
    refs.push(entry.ref);
    groups.set(module, refs);
  }
  const places = new Map();
  for (const [module, refs] of groups) {
    const resolved = await api.resolve(module, refs);
    refs.forEach((r, i) => places.set(`${module}|${r}`, resolved[i]));
  }
  for (const entry of entries) {
    const module = known.has(entry.module) ? entry.module : ctx.module().abbreviation;
    const place = places.get(`${module}|${entry.ref}`);
    if (!place) continue;
    list.append(h('li', {}, h('button', { class: 'link', onclick: () => ctx.openPlace(module, place.book, place.chapter, place.verse) },
      `${ctx.chapterLabel(module, place.book, place.chapter)}`, h('span', { class: 'muted' }, ` · ${module}`))));
  }
  container.append(list);
}

const RENDERERS = { today, notes, plans: renderPlans, highlights, bookmarks, history };

export async function renderStudy(container, tab, ctx) {
  const renderer = RENDERERS[tab] || today;
  try {
    await renderer(container, ctx);
  } catch (error) {
    container.append(empty(String(error)));
  }
}
