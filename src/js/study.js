/** The reader's own material: verse of the day, notes, highlights, bookmarks and history. */

import { api } from './api.js';
import { h } from './dom.js';
import { compareOsis } from './reference.js';
import { HIGHLIGHT_COLORS } from './settings.js';

export const STUDY_TABS = [
  { id: 'today', label: 'Today' },
  { id: 'notes', label: 'Notes' },
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

async function notes(container, ctx) {
  const user = ctx.user();
  if (!user.notes.length) {
    container.append(empty('No notes yet. Select a verse and choose Note.'));
    return;
  }
  const sorted = [...user.notes].sort((a, b) => compareOsis(a.start, b.start));
  const described = await describe(ctx, sorted.map((n) => n.start));
  for (const [i, note] of sorted.entries()) {
    const d = described[i];
    const area = h('textarea', { class: 'note-text', rows: 4, 'aria-label': `Note on ${d.label}`, dataset: { note: note.id } });
    area.value = note.text;
    area.addEventListener('input', () => {
      note.text = area.value;
      ctx.saveUser();
    });
    container.append(
      h('article', { class: 'card', id: `note-${note.id}` },
        h('header', {},
          h('button', { class: 'link', onclick: () => d.place && ctx.openPlace(ctx.module().abbreviation, d.place.book, d.place.chapter, d.place.verse) }, d.label),
          h('button', {
            class: 'quiet danger',
            'aria-label': `Delete note on ${d.label}`,
            onclick: () => {
              user.notes = user.notes.filter((n) => n.id !== note.id);
              ctx.saveUser();
              ctx.refresh();
            },
          }, 'Delete')),
        d.text ? h('p', { class: 'quote' }, d.text.replace(/\n/g, ' ')) : null,
        area),
    );
  }
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

const RENDERERS = { today, notes, highlights, bookmarks, history };

export async function renderStudy(container, tab, ctx) {
  const renderer = RENDERERS[tab] || today;
  try {
    await renderer(container, ctx);
  } catch (error) {
    container.append(empty(String(error)));
  }
}
