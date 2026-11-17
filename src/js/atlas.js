/** The atlas: the places of a passage on a map, and the events of the Bible in order of time. */

import { api } from './api.js';
import { clear, h } from './dom.js';
import { renderMarkup, splitRange } from './markup.js';
import { splitOsis } from './reference.js';

const SVG = 'http://www.w3.org/2000/svg';
/** The projection of scripts/build-atlas.py: hundredths of a degree from 12 W and 50 N, scaled for latitude 32. */
const WEST = -12;
const NORTH = 50;
const SCALE = Math.cos((32 * Math.PI) / 180);
const WHOLE = { x: 0, y: 0, w: 6530, h: 4500 };
const HOLY_LAND = { x: 3860, y: 1640, w: 300, h: 450 };

export function project(lat, lon) {
  return { x: (lon - WEST) * SCALE * 100, y: (NORTH - lat) * 100 };
}

/** `1446 BC` or `AD 30`, from a year where 0 is 1 BC. */
export function yearLabel(year) {
  return year <= 0 ? `${1 - year} BC` : `AD ${year}`;
}

export function yearsLabel(first, last) {
  if (first === last) return yearLabel(first);
  if (first <= 0 && last <= 0) return `${1 - first}-${1 - last} BC`;
  return `${yearLabel(first)} to ${yearLabel(last)}`;
}

/** The smallest view, kept to the shape of the map, that holds every point with a margin. */
export function viewAround(points, minimum = 300) {
  if (!points.length) return { ...HOLY_LAND };
  const xs = points.map((p) => p.x);
  const ys = points.map((p) => p.y);
  let [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  const margin = Math.max(30, (x1 - x0 + y1 - y0) * 0.12);
  x0 -= margin; x1 += margin; y0 -= margin; y1 += margin;
  let w = Math.max(x1 - x0, minimum);
  let hgt = Math.max(y1 - y0, minimum);
  // The map is three wide for two high.
  if (hgt / w > 2 / 3) w = hgt * 1.5;
  else hgt = w / 1.5;
  return { x: (x0 + x1) / 2 - w / 2, y: (y0 + y1) / 2 - hgt / 2, w, h: hgt };
}

function svg(tag, attrs = {}) {
  const el = document.createElementNS(SVG, tag);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
  return el;
}

let shapes = null;

async function outlines() {
  if (!shapes) shapes = await api.atlasShapes();
  return shapes;
}

/** The chapter of the verse in focus, as the first and last verse the atlas is asked about. */
function chapterOf(at) {
  const s = splitOsis(at);
  return s ? { first: `${s.book}.${s.chapter}.0`, last: `${s.book}.${s.chapter}.999`, book: s.book, chapter: s.chapter } : null;
}

async function verseList(ctx, spans, limit = 40) {
  const ranges = spans.map((s) => splitRange(s)).filter(Boolean).slice(0, limit);
  const { rangeLabels, preview } = await import('./library.js');
  const labels = await rangeLabels(ctx, ranges);
  return h('p', { class: 'topic-list' }, ranges.map((r, i) => {
    const button = h('button', { type: 'button', class: 'link ref', 'aria-expanded': 'false' }, labels[i]);
    button.addEventListener('click', () => preview(ctx, r, button));
    return [button, ' '];
  }), spans.length > limit ? h('span', { class: 'fine' }, `and ${spans.length - limit} more`) : null);
}

/** A short list of the places of the chapter, for the passage tab. */
export async function placesSection(ctx, at) {
  if (!ctx.modules().some((m) => m.kind === 'atlas')) return null;
  const chapter = chapterOf(at);
  if (!chapter) return null;
  const places = await api.atlasPlaces(chapter.first, chapter.last);
  if (!places.length) return null;
  return h('section', { class: 'lib-section' }, h('h3', {}, 'Places in this chapter'),
    h('p', { class: 'topic-list' }, places.slice(0, 30).map((p) => [h('button', {
      type: 'button',
      class: 'link',
      onclick: () => ctx.openLibrary('map', { place: p.id }),
    }, p.title), ' '])),
    h('button', { type: 'button', onclick: () => ctx.openLibrary('map') }, 'Show them on the map'));
}

export async function mapTab(container, ctx, options = {}) {
  const atlas = ctx.modules().find((m) => m.kind === 'atlas');
  const at = ctx.focus();
  const chapter = at ? chapterOf(at) : null;
  const [all, places] = await Promise.all([outlines(), chapter ? api.atlasPlaces(chapter.first, chapter.last) : []]);
  const map = svg('svg', { class: 'atlas-map', role: 'img', 'aria-label': 'Map of the places of the chapter', preserveAspectRatio: 'xMidYMid meet' });
  map.append(svg('rect', { class: 'sea', x: -2000, y: -2000, width: 11000, height: 9000 }));
  for (const [kind, d] of all) map.append(svg('path', { class: kind, d }));
  const marks = svg('g', { class: 'marks' });
  map.append(marks);
  const details = h('div', { class: 'place-details', 'aria-live': 'polite' });
  let view = { ...WHOLE };
  const setView = (v) => {
    view = v;
    map.setAttribute('viewBox', `${v.x} ${v.y} ${v.w} ${v.h}`);
    // Dots and labels keep their size on screen at any zoom: one unit is one pixel.
    const unit = v.w / (map.clientWidth || 600);
    map.style.setProperty('--unit', String(unit));
    const kept = [];
    for (const g of marks.children) {
      g.setAttribute('transform', `translate(${g.dataset.x} ${g.dataset.y}) scale(${unit})`);
      // A label that would cover one of a place named more often is left out.
      const box = { x: Number(g.dataset.x), y: Number(g.dataset.y) - 14 * unit, w: (g.textContent.length * 6.2 + 10) * unit, h: 14 * unit };
      const covers = kept.some((k) => box.x < k.x + k.w && k.x < box.x + box.w && box.y < k.y + k.h && k.y < box.y + box.h);
      g.classList.toggle('unlabelled', covers && !g.classList.contains('chosen'));
      if (!covers) kept.push(box);
    }
  };
  const show = async (place, focus = false) => {
    const entry = await api.entry(atlas.abbreviation, place.id);
    if (!entry) return;
    for (const mark of marks.querySelectorAll('.chosen')) mark.classList.remove('chosen');
    marks.querySelector(`[data-id="${place.id}"]`)?.classList.add('chosen');
    setView(view);
    if (focus) {
      const p = project(place.lat, place.lon);
      const size = Math.min(view.w, 400);
      setView({ x: p.x - size / 2, y: p.y - (size * 2) / 3 / 2, w: size, h: (size * 2) / 3 });
    }
    clear(details,
      h('h4', {}, place.title),
      renderMarkup(entry.body, {}),
      h('p', { class: 'fine' }, `Named in ${entry.spans.length} ${entry.spans.length === 1 ? 'passage' : 'passages'}.`),
      await verseList(ctx, entry.spans));
  };
  const draw = (list, chosen = null) => {
    clear(marks);
    for (const place of list) {
      const p = project(place.lat, place.lon);
      const g = svg('g', { class: 'place', 'data-id': place.id, 'data-x': p.x, 'data-y': p.y, tabindex: 0, role: 'button', 'aria-label': place.title });
      g.append(svg('circle', { cx: 0, cy: 0, r: 4 }), svg('text', { x: 7, y: -5 }));
      g.lastChild.textContent = place.title;
      g.addEventListener('click', () => show(place));
      g.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          show(place);
        }
      });
      marks.append(g);
    }
    setView(view);
    if (chosen) show(chosen, true);
  };
  const search = h('input', { type: 'search', placeholder: 'Find a place', 'aria-label': 'Find a place' });
  const found = h('ul', { class: 'headwords' });
  let asked = 0;
  search.addEventListener('input', async () => {
    const ticket = ++asked;
    const text = search.value.trim();
    const hits = text ? await api.atlasSearch(text) : [];
    if (ticket !== asked) return;
    clear(found, hits.map((p) => h('li', {}, h('button', {
      type: 'button',
      class: 'link',
      onclick: () => {
        draw([p], p);
      },
    }, p.title))));
  });
  const zoom = (factor) => {
    const w = Math.min(Math.max(view.w * factor, 40), WHOLE.w * 1.5);
    const hgt = (w * 2) / 3;
    setView({ x: view.x + (view.w - w) / 2, y: view.y + (view.h - hgt) / 2, w, h: hgt });
  };
  // Dragging moves the map; the wheel zooms.
  let drag = null;
  map.addEventListener('pointerdown', (event) => {
    if (event.target.closest?.('.place')) return;
    drag = { x: event.clientX, y: event.clientY, view: { ...view } };
    map.setPointerCapture?.(event.pointerId);
  });
  map.addEventListener('pointermove', (event) => {
    if (!drag) return;
    const k = view.w / (map.clientWidth || 600);
    setView({ ...drag.view, x: drag.view.x - (event.clientX - drag.x) * k, y: drag.view.y - (event.clientY - drag.y) * k });
  });
  map.addEventListener('pointerup', () => {
    drag = null;
  });
  map.addEventListener('wheel', (event) => {
    event.preventDefault();
    zoom(event.deltaY > 0 ? 1.25 : 0.8);
  }, { passive: false });
  const fit = () => setView(viewAround(places.map((p) => project(p.lat, p.lon))));
  container.append(
    h('p', { class: 'fine' }, chapter
      ? (places.length === 1 ? 'One place named in this chapter.' : places.length ? `${places.length} places named in this chapter, the most named first.` : 'No place of the atlas is named in this chapter.')
      : 'Open a chapter to see its places.'),
    h('div', { class: 'row map-tools' },
      h('button', { type: 'button', onclick: fit }, 'This chapter'),
      h('button', { type: 'button', onclick: () => setView({ ...HOLY_LAND }) }, 'Holy Land'),
      h('button', { type: 'button', onclick: () => setView({ ...WHOLE }) }, 'Whole map'),
      h('button', { type: 'button', 'aria-label': 'Zoom in', onclick: () => zoom(0.7) }, '+'),
      h('button', { type: 'button', 'aria-label': 'Zoom out', onclick: () => zoom(1.4) }, '−')),
    map,
    h('div', { class: 'search-form' }, search),
    found,
    details,
    h('p', { class: 'fine' }, `${atlas.abbreviation} · ${atlas.title}. Many places of the Bible are located only roughly, and some are disputed.`));
  // The map takes the wider panel, and its size is known only once it is on screen.
  container.closest?.('.panel')?.classList.add('wide');
  draw(places);
  fit();
  requestAnimationFrame?.(() => setView(view));
  const wanted = options.place && places.find((p) => p.id === options.place);
  if (wanted) show(wanted, true);
}

export async function timelineTab(container, ctx) {
  const atlas = ctx.modules().find((m) => m.kind === 'atlas');
  const events = await api.atlasTimeline();
  const at = ctx.focus();
  const chapter = at ? chapterOf(at) : null;
  const inChapter = (e) => {
    const range = splitRange(e.span);
    if (!range || !chapter) return false;
    const a = splitOsis(range.start);
    const b = splitOsis(range.end);
    return a?.book === chapter.book && b?.book === chapter.book && a.chapter <= chapter.chapter && b.chapter >= chapter.chapter;
  };
  const filter = h('input', { type: 'search', placeholder: 'Find an event', 'aria-label': 'Find an event' });
  const list = h('ol', { class: 'timeline' });
  const here = events.filter(inChapter);
  const draw = () => {
    const words = filter.value.toLowerCase().split(/\s+/).filter(Boolean);
    clear(list);
    let era = null;
    for (const e of events) {
      if (words.length && !words.every((w) => e.title.toLowerCase().includes(w))) continue;
      const label = eraLabel(e.first);
      if (label !== era) {
        era = label;
        list.append(h('li', { class: 'era' }, label));
      }
      const item = h('li', { class: `event${inChapter(e) ? ' here' : ''}`, dataset: { id: e.id } },
        h('span', { class: 'when' }, yearsLabel(e.first, e.last)), ' ',
        h('button', { type: 'button', class: 'link', 'aria-expanded': 'false' }, e.title));
      const button = item.querySelector('button');
      button.addEventListener('click', async () => {
        const open = item.querySelector('.event-details');
        if (open) {
          open.remove();
          button.setAttribute('aria-expanded', 'false');
          return;
        }
        button.setAttribute('aria-expanded', 'true');
        const entry = await api.entry(atlas.abbreviation, e.id);
        item.append(h('div', { class: 'event-details' }, entry?.body ? renderMarkup(entry.body, {}) : null, await verseList(ctx, entry?.spans || [])));
      });
      list.append(item);
    }
  };
  filter.addEventListener('input', draw);
  container.append(
    h('p', { class: 'fine' }, here.length ? `${here.length} events of this chapter are marked.` : 'No dated event is told in this chapter.'),
    h('div', { class: 'row' }, filter,
      here.length ? h('button', { type: 'button', onclick: () => list.querySelector('.event.here')?.scrollIntoView({ block: 'center' }) }, 'Go to this chapter') : null),
    list,
    h('p', { class: 'fine' }, `${atlas.abbreviation} · ${atlas.title}. The dates follow a traditional chronology built from the Bible itself; many are approximate, and scholars date much of the earlier history differently.`));
  draw();
}

/** The century, or before 1000 BC the millennium, a year falls in, as a heading. */
export function eraLabel(year) {
  if (year < -999) {
    const thousands = Math.floor((1 - year) / 1000);
    return `${thousands * 1000}s BC`;
  }
  if (year <= 0) {
    const hundreds = Math.floor((1 - year) / 100);
    return hundreds ? `${hundreds * 100}s BC` : 'The first century BC';
  }
  const hundreds = Math.floor((year - 1) / 100);
  return hundreds ? `AD ${hundreds * 100}s` : 'The first century AD';
}
