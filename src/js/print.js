/** Printing: pages of a fixed size with our own header, shown first, then handed to the system's print dialog. */

import { api, onAndroid } from './api.js';
import { clear, h, withBreaks } from './dom.js';
import { renderChapter } from './reader.js';
import { PAPERS } from './settings.js';

const MM = 96 / 25.4;
const MARGIN = { top: 16, bottom: 16, side: 18 };
const HEADER = 8;
const FOOTER = 8;
const PAGE_STYLE = 'grt-print-page';

/** The text area of a page, in millimetres. */
export function textArea(paper) {
  const size = PAPERS[paper] || PAPERS.a4;
  return {
    width: size.width - 2 * MARGIN.side,
    height: size.height - MARGIN.top - MARGIN.bottom - HEADER - FOOTER,
    paper: size,
  };
}

function setPaper(width, height) {
  let style = document.getElementById(PAGE_STYLE);
  if (!style) {
    style = h('style', { id: PAGE_STYLE });
    document.head.append(style);
  }
  // A page exactly as high as the paper rounds over it and leaves a blank page after it.
  style.textContent = `@page { size: ${width}mm ${height}mm; margin: 0; }
@media print { .print-page { height: calc(${height}mm - 1px) !important; } }`;
}

/** The lines of text in an element, top and bottom, relative to a point. */
function textLines(element, origin) {
  const range = document.createRange();
  range.selectNodeContents(element);
  const rects = [...(range.getClientRects?.() ?? [])]
    .filter((r) => r.height > 0 && r.width > 0)
    .map((r) => ({ top: r.top - origin, bottom: r.bottom - origin }))
    .sort((a, b) => a.top - b.top);
  const lines = [];
  for (const rect of rects) {
    const last = lines[lines.length - 1];
    if (last && rect.top < last.bottom - 1) last.bottom = Math.max(last.bottom, rect.bottom);
    else lines.push({ ...rect });
  }
  return lines;
}

/** Cuts a laid-out column into windows a page high, between blocks or between lines. */
export function flowBreaks(column, height, { keepWithNext = () => false, whole = () => false } = {}) {
  height = Math.max(height, 24);
  const origin = column.getBoundingClientRect().top;
  const blocks = [...column.children].map((element) => {
    const rect = element.getBoundingClientRect();
    return { element, top: rect.top - origin, bottom: rect.bottom - origin };
  }).filter((b) => b.bottom > b.top);
  if (!blocks.length) return [{ start: 0, end: 0 }];
  const cuts = [];
  blocks.forEach((block, i) => {
    if (!whole(block.element)) {
      const lines = textLines(block.element, origin);
      // Two lines at least stay on each side of a split.
      for (let k = 2; k < lines.length - 1; k += 1) {
        const at = (lines[k - 1].bottom + lines[k].top) / 2;
        if (at > block.top && at < block.bottom) cuts.push({ end: at, next: at, rank: 1 });
      }
    }
    const following = blocks[i + 1];
    if (following) cuts.push({ end: block.bottom, next: following.top, rank: keepWithNext(block.element) ? 2 : 0 });
  });
  cuts.sort((a, b) => a.end - b.end);
  const last = blocks[blocks.length - 1].bottom;
  const pages = [];
  let start = Math.min(0, blocks[0].top);
  while (start < last - 0.5) {
    const limit = start + height;
    if (last <= limit) {
      pages.push({ start, end: last });
      break;
    }
    const open = cuts.filter((c) => c.next > start && c.end <= limit && c.end > start);
    const cut = open.filter((c) => c.rank < 2).pop() ?? open.pop();
    if (cut) {
      pages.push({ start, end: cut.end });
      start = cut.next;
    } else {
      pages.push({ start, end: limit });
      start = limit;
    }
  }
  return pages;
}

/** One page's window of a laid-out column, as copies of the blocks that show in it. */
function flowPage(column, { start, end }) {
  const box = column.getBoundingClientRect();
  const body = h('div', { class: 'print-body' });
  body.style.height = `${Math.max(0, end - start)}px`;
  for (const element of column.children) {
    const rect = element.getBoundingClientRect();
    const top = rect.top - box.top;
    const bottom = rect.bottom - box.top;
    if ((bottom <= start && bottom > top) || (top >= end && top > start)) continue;
    const copy = element.cloneNode(true);
    Object.assign(copy.style, { position: 'absolute', top: `${top - start}px`, left: `${rect.left - box.left}px`, width: `${rect.width}px`, margin: '0' });
    body.append(copy);
  }
  return body;
}

/** Lays blocks out at the printed width, cuts them into pages and draws header and footer on each. */
async function paginate({ blocks, paper, size, header, lang, dir, className = '' }) {
  const area = textArea(paper);
  const column = h('div', { class: `print-text ${className}`, lang, dir });
  Object.assign(column.style, { position: 'fixed', left: '-100000px', top: '0', visibility: 'hidden', width: `${area.width}mm`, fontSize: `${size}pt` });
  column.append(...blocks);
  document.body.append(column);
  if (document.fonts?.ready) await document.fonts.ready;
  const keepWithNext = (el) => el.matches('h1, h2, h3, h4, .chapter-head, .keep');
  const whole = (el) => el.classList.contains('whole');
  const breaks = flowBreaks(column, area.height * MM, { keepWithNext, whole });
  const pages = breaks.map((b, i) => {
    const page = h('div', { class: 'print-page' });
    Object.assign(page.style, { width: `${area.paper.width}mm`, height: `${area.paper.height}mm`, padding: `${MARGIN.top}mm ${MARGIN.side}mm ${MARGIN.bottom}mm` });
    const body = flowPage(column, b);
    body.className = `print-body print-text ${className}`;
    body.setAttribute('lang', lang);
    body.setAttribute('dir', dir);
    body.style.fontSize = `${size}pt`;
    body.style.marginTop = `${HEADER}mm`;
    page.append(
      h('div', { class: 'print-head' }, h('span', {}, header.left), h('span', {}, header.right)),
      body,
      h('div', { class: 'print-foot' }, `${i + 1} / ${breaks.length}`));
    return page;
  });
  column.remove();
  return { width: area.paper.width, height: area.paper.height, pages };
}

// What can be printed

/** The chapter as it is shown, without the parts that only work on screen. */
function chapterBlocks(view, print, onlyVerses = null) {
  const data = onlyVerses
    ? { ...view.data, verses: view.data.verses.filter((v) => onlyVerses.has(v.verse)), titles: view.data.titles.filter((t) => onlyVerses.has(t.verse)), notes: view.data.notes.filter((n) => onlyVerses.has(n.verse)) }
    : view.data;
  const settings = { ...view.settings, verseNumbers: print.numbers, titles: print.headings, sourceNotes: print.editionNotes };
  const article = renderChapter({ data, module: view.module, bookName: view.bookName, settings, user: view.user, videos: [] });
  for (const el of article.querySelectorAll('.marker, .video-link, button')) el.remove();
  for (const el of article.querySelectorAll('.selected, .flash')) el.classList.remove('selected', 'flash');
  for (const el of article.querySelectorAll('a')) el.removeAttribute('href');
  if (onlyVerses) {
    const first = data.verses[0];
    const last = data.verses[data.verses.length - 1];
    const title = article.querySelector('h1');
    if (title && first) title.textContent = view.label(first.verse, last.verse);
  }
  const blocks = [...article.children].flatMap((el) => (el.matches('section') ? [...el.children] : [el]));
  if (print.myNotes) {
    const canon = new Set(data.verses.map((v) => v.canon));
    const notes = view.user.notes.filter((n) => canon.has(n.start) && n.text.trim());
    if (notes.length) {
      blocks.push(h('h2', { class: 'keep' }, 'My notes'));
      for (const note of notes) {
        const verse = data.verses.find((v) => v.canon === note.start);
        blocks.push(h('p', { class: 'my-note' }, h('b', {}, `${view.label(verse.verse, verse.verse)} `), withBreaks(note.text)));
      }
    }
  }
  return blocks;
}

function comparisonBlocks(view, print) {
  const columns = view.parallel.modules.length;
  const grid = `repeat(${columns}, minmax(0, 1fr))`;
  const head = h('div', { class: 'compare-row compare-head keep whole' }, view.modules.map((m, i) => h('div', { lang: m.language, dir: m.direction }, `${m.abbreviation} · ${view.bookNames[i]} ${view.parallel.primary.chapter}`)));
  head.style.gridTemplateColumns = grid;
  const rows = view.parallel.rows.map((row) => {
    const el = h('div', { class: 'compare-row whole' }, row.cells.map((cell, i) => {
      const m = view.modules[i];
      return h('div', { lang: m.language, dir: m.direction }, cell.map((piece) => [
        print.numbers ? h('sup', { class: 'vn' }, piece.chapter !== view.parallel.primary.chapter && i > 0 ? `${piece.chapter},${piece.verse}` : String(piece.verse)) : null,
        withBreaks(piece.text), ' ']));
    }));
    el.style.gridTemplateColumns = grid;
    return el;
  });
  return [head, ...rows];
}

function notesBlocks(described) {
  const blocks = [h('h1', {}, 'My notes')];
  for (const d of described) {
    blocks.push(h('h2', { class: 'keep' }, d.label));
    if (d.text) blocks.push(h('p', { class: 'quote keep' }, d.text.replace(/\n/g, ' ')));
    blocks.push(h('p', {}, withBreaks(d.note.text || '')));
  }
  return blocks;
}

// The preview

function control(label, input) {
  return h('label', {}, label, input);
}

function select(options, value, onchange) {
  const el = h('select', {}, options.map(([v, label]) => h('option', { value: v }, label)));
  el.value = value;
  el.addEventListener('change', () => onchange(el.value));
  return el;
}

function toggle(label, value, onchange) {
  const box = h('input', { type: 'checkbox' });
  box.checked = value;
  box.addEventListener('change', () => onchange(box.checked));
  return h('label', { class: 'check' }, box, label);
}

/** Sends the shown pages to the printer: GTK's dialog on Linux, the system's elsewhere. */
async function sendToPrinter(paper) {
  if (onAndroid() && window.grtPrint) {
    window.grtPrint.print(document.title || 'GRT Bible', paper.id);
    return;
  }
  try {
    await api.printPage(paper.width, paper.height);
  } catch {
    window.print();
  }
}

let isOpen = false;
let closeCurrent = null;

export function printPreviewOpen() {
  return isOpen;
}

export function closePrint() {
  closeCurrent?.();
}

/**
 * Opens the print preview.
 * `view` describes what the reader sees: chapter data, module, parallel view, selection and notes.
 */
export function openPrint(view, settings, saveSettings) {
  if (isOpen) return;
  isOpen = true;
  let print = { ...settings };
  const kinds = [['chapter', 'This chapter']];
  if (view.selection.size) kinds.unshift(['selection', 'The selected verses']);
  if (view.parallel) kinds.splice(view.selection.size ? 2 : 1, 0, ['comparison', 'The comparison']);
  if (view.user.notes.length) kinds.push(['notes', 'All my notes']);
  let what = view.initial && kinds.some(([k]) => k === view.initial) ? view.initial : kinds[0][0];

  const controls = h('span', { class: 'print-controls' });
  const count = h('span', { class: 'print-count', 'aria-live': 'polite' });
  const pagesHost = h('div', { class: 'print-pages' });
  const go = h('button', { type: 'button', class: 'primary' }, 'Print…');
  const close = h('button', { type: 'button' }, 'Close');
  const shell = h('div', { id: 'print-view', role: 'dialog', 'aria-label': 'Print' },
    h('div', { class: 'print-toolbar', role: 'toolbar', 'aria-label': 'Print' },
      h('strong', {}, 'Print'), controls, h('span', { class: 'print-spacer' }), count, close, go),
    pagesHost);
  document.body.append(shell);

  let building = Promise.resolve();
  let paper = null;
  const rebuild = () => {
    count.textContent = 'Preparing pages…';
    building = building.then(async () => {
      let blocks;
      let className = '';
      let header = { left: view.title, right: view.module.abbreviation };
      if (what === 'notes') {
        blocks = notesBlocks(await view.describeNotes());
        header = { left: 'My notes', right: view.module.abbreviation };
      } else if (what === 'comparison') {
        blocks = comparisonBlocks(view, print);
        header = { left: view.title, right: view.parallel.modules.join(' · ') };
      } else {
        blocks = chapterBlocks(view, print, what === 'selection' ? view.selection : null);
        className = `chapter layout-${view.settings.layout}`;
      }
      const built = await paginate({ blocks, paper: print.paper, size: print.size, header, lang: view.module.language, dir: view.module.direction || 'ltr', className });
      paper = { id: print.paper, width: built.width, height: built.height };
      setPaper(built.width, built.height);
      const available = Math.max(200, pagesHost.clientWidth - 48);
      const natural = built.width * MM;
      const scale = Math.min(1, available / natural);
      clear(pagesHost, built.pages.map((page) => {
        const frame = h('div', { class: 'print-frame' });
        frame.style.width = `${natural * scale}px`;
        frame.style.height = `${built.height * MM * scale}px`;
        page.style.transform = scale < 1 ? `scale(${scale})` : '';
        frame.append(page);
        return frame;
      }));
      count.textContent = `${built.pages.length} page${built.pages.length === 1 ? '' : 's'}`;
    }).catch((error) => {
      count.textContent = String(error);
    });
    return building;
  };

  const drawControls = () => {
    const change = (key) => (value) => {
      print = { ...print, [key]: value };
      saveSettings(print);
      drawControls();
      rebuild();
    };
    clear(controls,
      control('What', select(kinds, what, (v) => { what = v; drawControls(); rebuild(); })),
      control('Paper', select(Object.entries(PAPERS).map(([id, p]) => [id, p.label]), print.paper, change('paper'))),
      control('Text', select([8, 9, 10, 11, 12, 13, 14, 16].map((n) => [String(n), `${n} pt`]), String(print.size), (v) => change('size')(Number(v)))),
      what === 'notes' ? null : toggle('Verse numbers', print.numbers, change('numbers')),
      what === 'chapter' || what === 'selection' ? [
        toggle('Headings', print.headings, change('headings')),
        toggle('Notes of the edition', print.editionNotes, change('editionNotes')),
        toggle('My notes', print.myNotes, change('myNotes')),
      ] : null);
  };

  const finish = () => {
    document.removeEventListener('keydown', onKey, true);
    shell.remove();
    isOpen = false;
    closeCurrent = null;
  };
  closeCurrent = finish;
  function onKey(event) {
    if (event.key === 'Escape') {
      event.stopPropagation();
      finish();
    }
  }
  document.addEventListener('keydown', onKey, true);
  close.addEventListener('click', finish);
  go.addEventListener('click', async () => {
    await building;
    if (paper) await sendToPrinter(paper);
  });
  drawControls();
  rebuild();
}
