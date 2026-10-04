/** Starts the program and ties the pieces together. */

import { api, setPlatform } from './api.js';
import { BY_OSIS, nameIn } from './books.js';
import { copyText } from './copy.js';
import { $, $$, clear, debounce, h } from './dom.js';
import { openNavigator } from './navigator.js';
import { closeOverlay, isOpen, toast } from './overlay.js';
import { openPalette } from './palette.js';
import { askAboutVideos, openPreferences } from './preferences.js';
import { renderChapter, renderParallel } from './reader.js';
import { compareOsis, formatReference } from './reference.js';
import { renderSearch } from './search.js';
import { applyAppearance, HIGHLIGHT_COLORS, normalise } from './settings.js';
import { renderStudy, STUDY_TABS } from './study.js';
import { renderVideos } from './videos.js';

const HISTORY_LIMIT = 100;

const state = {
  version: '',
  platform: '',
  modules: [],
  settings: normalise({}),
  user: null,
  module: null,
  book: 'Gen',
  chapter: 1,
  data: null,
  parallel: null,
  selection: [],
  anchor: null,
  panel: null,
  studyTab: 'today',
};

export function normaliseUser(raw) {
  const user = raw && typeof raw === 'object' ? raw : {};
  return {
    reading: {
      current: user.reading?.current ?? null,
      history: Array.isArray(user.reading?.history) ? user.reading.history : [],
    },
    notes: Array.isArray(user.notes) ? user.notes : [],
    highlights: Array.isArray(user.highlights) ? user.highlights : [],
    bookmarks: Array.isArray(user.bookmarks) ? user.bookmarks : [],
    saved_videos: Array.isArray(user.saved_videos) ? user.saved_videos : [],
  };
}

const moduleInfo = (abbr = state.module) => state.modules.find((m) => m.abbreviation === abbr) || null;

function bookName(module, osis) {
  const own = module?.books.find((b) => b.osis === osis);
  return own ? own.name : nameIn(osis, module?.language || 'en')[0];
}

function newId(prefix) {
  return `${prefix}${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
}

// Saving

const saveUserSoon = debounce(async () => {
  try {
    await api.saveUser(state.user);
  } catch (error) {
    toast(`Your data could not be saved: ${error}`, 6000);
  }
}, 1000);

const saveSettingsSoon = debounce(async () => {
  try {
    await api.saveSettings(state.settings);
  } catch (error) {
    toast(`Settings could not be saved: ${error}`, 6000);
  }
}, 400);

function setSetting(key, value) {
  const before = state.settings;
  state.settings = normalise({ ...before, [key]: value });
  applyAppearance(state.settings);
  saveSettingsSoon();
  if (['layout', 'verseNumbers', 'titles', 'sourceNotes'].includes(key)) render();
  if (key === 'videos') {
    $('#btn-videos').hidden = state.settings.videos !== true;
    if (!state.settings.videos && state.panel === 'videos') hidePanel();
    render();
  }
}

// Reading position

function visit(ref) {
  const [book, chapter] = ref.split('.');
  const history = state.user.reading.history.filter((p) => {
    const [b, c] = p.ref.split('.');
    return !(b === book && c === chapter);
  });
  history.unshift({ module: state.module, ref, offset: 0 });
  state.user.reading.history = history.slice(0, HISTORY_LIMIT);
}

function topVerse() {
  const reader = $('#reader');
  const top = reader.getBoundingClientRect().top;
  for (const el of $$('.verse[data-canon]', reader)) {
    const box = el.getBoundingClientRect();
    if (box.bottom > top + 4) {
      const offset = box.height > 0 ? Math.min(1, Math.max(0, (top - box.top) / box.height)) : 0;
      return { canon: el.dataset.canon, verse: Number(el.dataset.verse), offset };
    }
  }
  return null;
}

const trackPosition = debounce(() => {
  if (!state.data) return;
  const found = topVerse();
  if (!found) return;
  state.user.reading.current = { module: state.module, ref: found.canon, offset: Number(found.offset.toFixed(3)) };
  saveUserSoon();
}, 600);

function scrollToVerse(verse, offset = 0) {
  const reader = $('#reader');
  if (!verse) {
    reader.scrollTop = 0;
    return;
  }
  const el = $(`.verse[data-verse="${verse}"]`, reader);
  if (!el) return;
  const header = $('.parallel thead', reader);
  const covered = header ? header.getBoundingClientRect().height : 0;
  const delta = el.getBoundingClientRect().top - reader.getBoundingClientRect().top;
  reader.scrollTop += delta - 12 - covered + el.getBoundingClientRect().height * offset;
}

// Opening chapters

async function openPlace(module, book, chapter, verse = null, verseEnd = null, { offset = 0, select = false } = {}) {
  let info = moduleInfo(module) || moduleInfo(state.settings.module) || state.modules[0];
  if (!info) return;
  if (!info.books.some((b) => b.osis === book)) {
    toast(`${info.abbreviation} does not contain ${bookName(info, book)}.`);
    book = info.books[0].osis;
    chapter = 1;
    verse = null;
  }
  let data;
  try {
    data = await api.chapter(info.abbreviation, book, chapter);
  } catch (error) {
    toast(String(error), 5000);
    return;
  }
  state.module = info.abbreviation;
  state.book = data.book;
  state.chapter = data.chapter;
  state.data = data;
  state.parallel = null;
  if (state.settings.module !== info.abbreviation) setSetting('module', info.abbreviation);
  if (state.settings.parallel.enabled) {
    const others = state.settings.parallel.modules.filter((m) => m !== info.abbreviation && moduleInfo(m));
    if (others.length) {
      try {
        state.parallel = await api.parallel([info.abbreviation, ...others], data.book, data.chapter);
      } catch (error) {
        toast(String(error), 5000);
      }
    }
  }
  state.selection = [];
  state.anchor = null;
  if (data.verses.length) visit(data.verses[0].canon);
  render();
  scrollToVerse(verse, offset);
  if (select && verse) {
    const last = verseEnd && verseEnd >= verse ? verseEnd : verse;
    state.selection = data.verses.map((v) => v.verse).filter((v) => v >= verse && v <= last);
    state.anchor = verse;
    showSelection();
  } else if (verse) {
    const el = $(`.verse[data-verse="${verse}"]`);
    if (el) {
      el.classList.add('flash');
      setTimeout(() => el.classList.remove('flash'), 1600);
    }
  }
  updateToolbar();
  trackPosition();
  if (state.panel === 'videos') refreshPanel();
}

async function openCanon(ref, module = state.module) {
  const [place] = await api.resolve(module, [ref]);
  if (place) await openPlace(module, place.book, place.chapter, place.verse);
}

function render() {
  const reader = $('#reader');
  const info = moduleInfo();
  if (!state.data || !info) return;
  const scroll = reader.scrollTop;
  let content;
  if (state.parallel) {
    const modules = state.parallel.modules.map((m) => moduleInfo(m));
    const names = modules.map((m) => bookName(m, state.book));
    content = renderParallel({ data: state.parallel, modules, bookNames: names, settings: state.settings });
  } else {
    content = renderChapter({
      data: state.data,
      module: info,
      bookName: bookName(info, state.book),
      settings: state.settings,
      user: state.user,
      videos: state.settings.videos ? state.data.videos : [],
    });
  }
  clear(reader, content);
  reader.scrollTop = scroll;
  showSelection();
}

function updateToolbar() {
  const info = moduleInfo();
  $('#btn-location').textContent = `${bookName(info, state.book)} ${state.chapter}`;
  $('#btn-prev').disabled = !state.data?.prev;
  $('#btn-next').disabled = !state.data?.next;
  $('#module-picker').value = state.module;
  $('#module-label').textContent = state.module;
  $('#btn-parallel').setAttribute('aria-pressed', state.settings.parallel.enabled ? 'true' : 'false');
  document.title = `${bookName(info, state.book)} ${state.chapter} · ${state.module} · GRT Bible`;
  drawParallelBar();
}

function fillModulePicker() {
  clear($('#module-picker'), state.modules.map((m) => h('option', { value: m.abbreviation }, `${m.abbreviation} · ${m.title}`)));
}

function drawParallelBar() {
  const bar = $('#parallel-bar');
  bar.hidden = !state.settings.parallel.enabled;
  if (bar.hidden) return;
  const chosen = new Set(state.settings.parallel.modules);
  clear(bar,
    h('span', { class: 'muted' }, 'Compare with'),
    state.modules.filter((m) => m.abbreviation !== state.module).map((m) => {
      const box = h('input', { type: 'checkbox', value: m.abbreviation });
      box.checked = chosen.has(m.abbreviation);
      box.addEventListener('change', () => {
        let list = state.settings.parallel.modules.filter((x) => x !== m.abbreviation);
        if (box.checked) list = [...list, m.abbreviation].slice(-3);
        setSetting('parallel', { enabled: true, modules: list });
        reopen();
      });
      return h('label', { class: 'check', title: m.title }, box, h('span', {}, m.abbreviation));
    }));
}

function reopen() {
  const found = topVerse();
  openPlace(state.module, state.book, state.chapter, found?.verse ?? null, null, { offset: found?.offset ?? 0 });
}

// Selection

/** The selection after clicking a verse, always one unbroken run. */
export function nextSelection(current, verse, anchor, order) {
  const at = (v) => order.indexOf(v);
  if (anchor !== null && anchor !== undefined && at(anchor) >= 0) {
    const [a, b] = [Math.min(at(anchor), at(verse)), Math.max(at(anchor), at(verse))];
    return order.slice(a, b + 1);
  }
  if (!current.length) return [verse];
  const first = at(current[0]);
  const last = at(current[current.length - 1]);
  const here = at(verse);
  if (current.includes(verse)) {
    if (current.length === 1) return [];
    if (here === first) return order.slice(first + 1, last + 1);
    if (here === last) return order.slice(first, last);
    return [verse];
  }
  if (here === first - 1) return order.slice(here, last + 1);
  if (here === last + 1) return order.slice(first, here + 1);
  return [verse];
}

function selectedVerses() {
  const set = new Set(state.selection);
  return (state.data?.verses || []).filter((v) => set.has(v.verse));
}

function selectionLabel(verses = selectedVerses()) {
  const info = moduleInfo();
  if (!verses.length) return '';
  const own = info.books.find((b) => b.osis === state.book);
  return formatReference(info.language, state.book, state.chapter, verses[0].verse, state.chapter, verses[verses.length - 1].verse, own?.abbrev);
}

function showSelection() {
  const set = new Set(state.selection);
  for (const el of $$('#reader .verse[data-verse]')) {
    el.classList.toggle('selected', set.has(Number(el.dataset.verse)));
  }
  const bar = $('#selection-bar');
  if (!state.selection.length) {
    bar.hidden = true;
    return;
  }
  const verses = selectedVerses();
  const first = verses[0];
  const last = verses[verses.length - 1];
  const bookmarked = state.user.bookmarks.some((b) => b.ref === first.canon);
  clear(bar,
    h('span', { class: 'selection-label' }, selectionLabel(verses)),
    h('button', { onclick: () => copySelection() }, 'Copy'),
    h('span', { class: 'swatches', role: 'group', 'aria-label': 'Highlight' },
      HIGHLIGHT_COLORS.map((color) => h('button', { class: `swatch hl-${color}`, 'aria-label': `Highlight ${color}`, title: `Highlight ${color}`, onclick: () => highlight(color) })),
      h('button', { class: 'swatch none', 'aria-label': 'Remove highlight', title: 'Remove highlight', onclick: () => highlight(null) })),
    h('button', {
      onclick: () => {
        const note = { id: newId('n'), start: first.canon, end: last.canonEnd, text: '' };
        state.user.notes.push(note);
        saveUserSoon();
        render();
        state.studyTab = 'notes';
        showPanel('study').then(() => {
          const area = $(`textarea[data-note="${note.id}"]`);
          if (area) {
            area.scrollIntoView?.({ block: 'nearest' });
            area.focus();
          }
        });
      },
    }, 'Note'),
    h('button', {
      'aria-pressed': bookmarked ? 'true' : 'false',
      onclick: () => {
        if (bookmarked) state.user.bookmarks = state.user.bookmarks.filter((b) => b.ref !== first.canon);
        else state.user.bookmarks.push({ id: newId('b'), ref: first.canon, label: '' });
        saveUserSoon();
        render();
        refreshPanel();
      },
    }, bookmarked ? 'Bookmarked' : 'Bookmark'),
    h('button', { class: 'quiet', 'aria-label': 'Clear selection', onclick: clearSelection }, 'Close'));
  bar.hidden = false;
}

function clearSelection() {
  state.selection = [];
  state.anchor = null;
  showSelection();
}

function highlight(color) {
  const verses = selectedVerses();
  if (!verses.length) return;
  const start = verses[0].canon;
  const end = verses[verses.length - 1].canonEnd;
  state.user.highlights = state.user.highlights.filter((x) => compareOsis(x.end, start) < 0 || compareOsis(x.start, end) > 0);
  if (color) state.user.highlights.push({ start, end, color });
  saveUserSoon();
  render();
  refreshPanel();
}

async function writeClipboard(text) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const area = h('textarea', { class: 'sr-only' });
    area.value = text;
    document.body.append(area);
    area.select();
    document.execCommand('copy');
    area.remove();
  }
}

function copyPayload(verses, excerpt = null) {
  const info = moduleInfo();
  const own = info.books.find((b) => b.osis === state.book);
  return copyText({
    verses: verses.map((v) => ({ chapter: state.chapter, verse: v.verse, text: v.text })),
    excerpt,
    book: state.book,
    language: info.language,
    abbrev: own?.abbrev,
    module: info.abbreviation,
    options: state.settings.copy,
  });
}

async function copySelection() {
  const verses = selectedVerses();
  if (!verses.length) return false;
  await writeClipboard(copyPayload(verses));
  toast(`Copied ${selectionLabel(verses)}`);
  return true;
}

/** What the reader selected with the mouse, as verses and clean text. */
function textSelection() {
  const selection = window.getSelection();
  if (!selection || selection.isCollapsed || !selection.rangeCount) return null;
  const reader = $('#reader');
  const range = selection.getRangeAt(0);
  if (!reader.contains(range.commonAncestorContainer)) return null;
  const numbers = $$('.verse[data-verse]', reader).filter((el) => selection.containsNode(el, true)).map((el) => Number(el.dataset.verse));
  if (!numbers.length) return null;
  const fragment = range.cloneContents();
  for (const el of fragment.querySelectorAll('.vn, .fn-ref, .marker, .notes, .chapter-head')) el.remove();
  const set = new Set(numbers);
  const verses = state.data.verses.filter((v) => set.has(v.verse));
  return { verses, excerpt: fragment.textContent.replace(/\s+/g, ' ').trim() };
}

/** A quick horizontal stroke turns the page: to the left for the next chapter, to the right for the previous. */
export function swipeDirection(start, end) {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  if (end.at - start.at > 600 || Math.abs(dx) < 70 || Math.abs(dy) > Math.abs(dx) / 2) return null;
  return dx < 0 ? 'next' : 'prev';
}

// Layers the Android back button closes, topmost first

const back = { listener: null, queue: Promise.resolve() };

function layerOpen() {
  return isOpen() || !$('#selection-bar').hidden || !$('#panel').hidden;
}

function closeTopLayer() {
  if (isOpen()) closeOverlay();
  else if (state.selection.length) clearSelection();
  else if (state.panel) hidePanel();
}

/** Listens to the back button only while something is open, so with nothing open it leaves the program. */
function syncBackButton() {
  const app = window.__TAURI__?.app;
  if (state.platform !== 'android' || !app?.onBackButtonPress) return;
  back.queue = back.queue.then(async () => {
    if (layerOpen() && !back.listener) {
      back.listener = await app.onBackButtonPress(closeTopLayer);
    } else if (!layerOpen() && back.listener) {
      const listener = back.listener;
      back.listener = null;
      await listener.unregister();
    }
  }).catch(() => {});
}

// Panel

async function showPanel(kind, options = {}) {
  state.panel = kind;
  if (state.settings.panel !== kind) setSetting('panel', kind);
  const panel = $('#panel');
  panel.hidden = false;
  const body = h('div', { class: 'panel-body' });
  const close = h('button', { class: 'quiet', 'aria-label': 'Close panel', onclick: hidePanel }, 'Close');
  const titles = { search: 'Search', study: 'Study', videos: 'Videos' };
  const header = h('header', { class: 'panel-head' }, h('h2', {}, titles[kind]), close);
  const parts = [header];
  if (kind === 'study') {
    parts.push(h('nav', { class: 'tabs', role: 'tablist' }, STUDY_TABS.map((t) => h('button', {
      role: 'tab',
      'aria-selected': t.id === state.studyTab ? 'true' : 'false',
      onclick: () => {
        state.studyTab = t.id;
        showPanel('study');
      },
    }, t.label))));
  }
  parts.push(body);
  clear(panel, parts);
  if (kind === 'search') {
    const input = renderSearch(body, ctx, options.initial || '');
    input.focus();
  } else if (kind === 'study') {
    await renderStudy(body, state.studyTab, ctx);
  } else if (kind === 'videos') {
    await renderVideos(body, ctx, options.chapterOnly);
  }
  for (const [id, k] of [['#btn-search', 'search'], ['#btn-study', 'study'], ['#btn-videos', 'videos']]) {
    $(id).setAttribute('aria-pressed', state.panel === k ? 'true' : 'false');
  }
}

function hidePanel() {
  state.panel = null;
  if (state.settings.panel !== null) setSetting('panel', null);
  $('#panel').hidden = true;
  for (const id of ['#btn-search', '#btn-study', '#btn-videos']) $(id).setAttribute('aria-pressed', 'false');
}

function togglePanel(kind, options) {
  if (state.panel === kind) hidePanel();
  else showPanel(kind, options);
}

function refreshPanel() {
  if (state.panel && state.panel !== 'search') showPanel(state.panel);
}

// The context panels and dialogs work through

const ctx = {
  module: () => moduleInfo(),
  modules: () => state.modules,
  settings: () => state.settings,
  setSetting,
  user: () => state.user,
  saveUser: () => saveUserSoon(),
  flushUser: () => api.saveUser(state.user),
  replaceUser: (data) => {
    state.user = normaliseUser(data);
    render();
    refreshPanel();
  },
  refresh: () => {
    render();
    refreshPanel();
  },
  openPlace: (module, book, chapter, verse, verseEnd) => openPlace(module, book, chapter, verse, verseEnd, { select: Boolean(verseEnd) }),
  place: () => ({ book: state.book, chapter: state.chapter }),
  testament: (osis) => BY_OSIS.get(osis)?.testament,
  placeLabel: (place, verseEnd = null, module = moduleInfo()) => {
    const own = module?.books.find((b) => b.osis === place.book);
    return formatReference(module?.language || 'en', place.book, place.chapter, place.verse, place.chapter, verseEnd ?? place.verse, own?.abbrev);
  },
  chapterLabel: (module, book, chapter) => `${bookName(moduleInfo(module), book)} ${chapter}`,
  chapterVideos: () => state.data?.videos || [],
  version: () => state.version,
  platform: () => state.platform,
  modulesChanged: async (abbreviation) => {
    const info = await api.startup();
    state.modules = info.modules;
    fillModulePicker();
    const target = abbreviation && moduleInfo(abbreviation) ? abbreviation : moduleInfo(state.module) ? state.module : state.modules[0].abbreviation;
    const found = topVerse();
    if (found && target !== state.module) {
      await openCanon(found.canon, target);
    } else {
      await openPlace(target, state.book, state.chapter);
    }
  },
};

// Commands

function goRelative(which) {
  const target = state.data?.[which];
  if (target) openPlace(state.module, target.book, target.chapter);
}

function goBook(step) {
  const books = moduleInfo().books;
  const at = books.findIndex((b) => b.osis === state.book);
  const next = books[at + step];
  if (next) openPlace(state.module, next.osis, 1);
}

function palette() {
  openPalette({
    module: moduleInfo(),
    onGo: (osis, reference) => {
      const own = moduleInfo().books.find((b) => b.osis === osis);
      const chapter = Math.min(reference.chapter, own ? own.chapters : reference.chapter);
      const sameChapterEnd = reference.chapterEnd === null || reference.chapterEnd === reference.chapter;
      openPlace(state.module, osis, chapter, reference.verse, sameChapterEnd ? reference.verseEnd : null, { select: reference.verse !== null });
    },
    onSearch: (text) => showPanel('search', { initial: text }),
  });
}

function bind() {
  $('#btn-prev').addEventListener('click', () => goRelative('prev'));
  $('#btn-next').addEventListener('click', () => goRelative('next'));
  $('#btn-location').addEventListener('click', () =>
    openNavigator({ module: moduleInfo(), current: { book: state.book, chapter: state.chapter }, onPick: (book, chapter) => openPlace(state.module, book, chapter) }));
  $('#module-picker').addEventListener('change', async (e) => {
    const target = e.target.value;
    const found = topVerse();
    if (found) await openCanon(found.canon, target);
    else await openPlace(target, state.book, state.chapter);
  });
  $('#btn-parallel').addEventListener('click', () => {
    const enabled = !state.settings.parallel.enabled;
    let modules = state.settings.parallel.modules.filter((m) => m !== state.module && moduleInfo(m));
    if (enabled && !modules.length) {
      const other = state.modules.find((m) => m.abbreviation !== state.module && m.books.some((b) => b.osis === state.book));
      if (other) modules = [other.abbreviation];
    }
    setSetting('parallel', { enabled, modules });
    reopen();
  });
  $('#btn-goto').addEventListener('click', palette);
  $('#btn-search').addEventListener('click', () => togglePanel('search'));
  $('#btn-study').addEventListener('click', () => togglePanel('study'));
  $('#btn-videos').addEventListener('click', () => togglePanel('videos'));
  $('#btn-settings').addEventListener('click', () => openPreferences(ctx));

  const reader = $('#reader');
  reader.addEventListener('scroll', trackPosition);
  let touch = null;
  reader.addEventListener('touchstart', (event) => {
    const t = event.touches[0];
    touch = event.touches.length === 1 ? { x: t.clientX, y: t.clientY, at: event.timeStamp } : null;
  }, { passive: true });
  reader.addEventListener('touchend', (event) => {
    const start = touch;
    touch = null;
    const t = event.changedTouches[0];
    if (!start || !t || isOpen() || !window.getSelection()?.isCollapsed) return;
    const direction = swipeDirection(start, { x: t.clientX, y: t.clientY, at: event.timeStamp });
    if (direction) goRelative(direction);
  }, { passive: true });
  reader.addEventListener('click', (event) => {
    const noteButton = event.target.closest('.marker.note');
    if (noteButton) {
      state.studyTab = 'notes';
      showPanel('study').then(() => {
        const area = $(`textarea[data-note="${noteButton.dataset.note}"]`);
        if (area) {
          area.scrollIntoView?.({ block: 'nearest' });
          area.focus();
        }
      });
      return;
    }
    if (event.target.closest('#chapter-videos')) {
      showPanel('videos', { chapterOnly: true });
      return;
    }
    if (event.target.closest('a')) return;
    const verse = event.target.closest('.verse[data-verse]');
    if (!verse) return;
    const selection = window.getSelection();
    if (selection && !selection.isCollapsed) return;
    const number = Number(verse.dataset.verse);
    state.selection = nextSelection(state.selection, number, event.shiftKey ? state.anchor : null, state.data.verses.map((v) => v.verse));
    state.anchor = state.selection.length ? (event.shiftKey ? state.anchor : number) : null;
    showSelection();
  });

  document.addEventListener('copy', (event) => {
    const picked = textSelection();
    if (!picked) return;
    event.preventDefault();
    event.clipboardData.setData('text/plain', copyPayload(picked.verses, picked.excerpt));
  });

  document.addEventListener('keydown', (event) => {
    const typing = event.target.closest?.('input, textarea, select, [contenteditable]');
    const ctrl = event.ctrlKey || event.metaKey;
    const key = event.key.toLowerCase();
    if (event.key === 'Escape') {
      closeTopLayer();
      return;
    }
    if (ctrl && key === 'k') {
      event.preventDefault();
      if (!isOpen()) palette();
      return;
    }
    if (isOpen()) return;
    if (ctrl && key === 'f') {
      event.preventDefault();
      showPanel('search');
      return;
    }
    if (ctrl && key === 'c' && !typing) {
      if (!textSelection() && state.selection.length) {
        event.preventDefault();
        copySelection();
      }
      return;
    }
    if (ctrl && (event.key === '+' || event.key === '=' || event.key === '-')) {
      event.preventDefault();
      setSetting('fontSize', state.settings.fontSize + (event.key === '-' ? -1 : 1));
      return;
    }
    if (typing || ctrl || event.altKey) return;
    if (event.key === 'ArrowLeft') goRelative('prev');
    else if (event.key === 'ArrowRight') goRelative('next');
    else if (event.key === '[') goBook(-1);
    else if (event.key === ']') goBook(1);
  });

  window.addEventListener('pagehide', () => {
    saveUserSoon.flush();
    saveSettingsSoon.flush();
  });

  const layers = new MutationObserver(syncBackButton);
  for (const id of ['#overlay', '#selection-bar', '#panel']) layers.observe($(id), { attributes: true, attributeFilter: ['hidden'] });
  window.matchMedia?.('(prefers-color-scheme: dark)').addEventListener?.('change', () => applyAppearance(state.settings));
}

export async function boot() {
  let info;
  try {
    info = await api.startup();
  } catch (error) {
    clear($('#reader'), h('p', { class: 'empty-state' }, `GRT Bible could not start: ${error}`));
    return;
  }
  state.version = info.version;
  state.platform = info.platform || '';
  setPlatform(state.platform);
  document.documentElement.dataset.platform = state.platform;
  state.modules = info.modules;
  state.settings = normalise(info.settings);
  state.user = normaliseUser(info.user);
  applyAppearance(state.settings);
  $('#btn-videos').hidden = state.settings.videos !== true;
  bind();
  if (!state.modules.length) {
    clear($('#reader'), h('p', { class: 'empty-state' }, 'No translations are installed. Open Settings to import one.'));
    return;
  }
  fillModulePicker();
  for (const notice of info.notices) toast(notice, 8000);

  const current = state.user.reading.current;
  const module = moduleInfo(current?.module) ? current.module : moduleInfo(state.settings.module) ? state.settings.module : state.modules[0].abbreviation;
  let opened = false;
  if (current?.ref) {
    try {
      const [place] = await api.resolve(module, [current.ref]);
      if (place) {
        await openPlace(module, place.book, place.chapter, place.verse, null, { offset: current.offset || 0 });
        opened = true;
      }
    } catch {
      opened = false;
    }
  }
  if (!opened) {
    const first = moduleInfo(module).books[0].osis;
    await openPlace(module, first, 1);
  }
  if (state.settings.videos === null) askAboutVideos(ctx);
  const panel = state.settings.panel;
  if (panel === 'study' || panel === 'search' || (panel === 'videos' && state.settings.videos)) showPanel(panel);
}

if (typeof window !== 'undefined' && window.__TAURI__ && !window.__GRT_TEST__) {
  boot();
}

export { state, ctx };
