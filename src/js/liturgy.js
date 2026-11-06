/** The day in the Roman calendar of Italy, with its readings opened in the reader's translation. */

import { api } from './api.js';
import { clear, h } from './dom.js';
import { preview, rangeLabels } from './library.js';
import { splitRange } from './markup.js';
import { videoCard, videoCatalog, videoTouches } from './videos.js';

const SLOTS = {
  'first-reading': 'First reading',
  'second-reading': 'Second reading',
  psalm: 'Psalm',
  gospel: 'Gospel',
  epistle: 'Epistle',
};

const GRADES = { solemnity: 'Solemnity', feast: 'Feast', memorial: 'Memorial', optional: 'Optional memorial' };
const SEASONS = { advent: 'Advent', christmas: 'Christmas Time', lent: 'Lent', triduum: 'Easter Triduum', easter: 'Easter Time', ordinary: 'Ordinary Time' };

export function slotLabel(slot) {
  if (SLOTS[slot]) return SLOTS[slot];
  const m = /^(reading|psalm)-(\d+)$/.exec(slot);
  return m ? `${m[1] === 'psalm' ? 'Psalm' : 'Reading'} ${m[2]}` : slot;
}

/** `YYYY-MM-DD` of a local date. */
export function isoDate(date) {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

export function shiftDate(iso, days) {
  const [y, m, d] = iso.split('-').map(Number);
  return isoDate(new Date(y, m - 1, d + days));
}

/** Labels of the pieces of one reading, the book written once: `Is 63,16-17; 63,19; 64,2-7`. */
export function joinLabels(labels) {
  let book = null;
  return labels.map((label) => {
    const m = /^(.*?\S)\s+(\d.*)$/.exec(label);
    if (!m) return label;
    const same = m[1] === book;
    book = m[1];
    return same ? m[2] : label;
  }).join('; ');
}

/** Ranges of the internal OSIS strings of a reading. */
function rangesOf(osisList) {
  return osisList.map((o) => splitRange(o)).filter(Boolean);
}

async function readingButton(ctx, osisList, fallback) {
  const ranges = rangesOf(osisList);
  const labels = ranges.length ? await rangeLabels(ctx, ranges) : [];
  const button = h('button', { type: 'button', class: 'link ref', 'aria-expanded': 'false' }, labels.length ? joinLabels(labels) : fallback);
  if (ranges.length) {
    // The preview shows the whole reading, from its first verse to its last.
    const whole = { start: ranges[0].start, end: ranges[ranges.length - 1].end };
    button.addEventListener('click', () => preview(ctx, whole, button));
  }
  return button;
}

async function massList(ctx, mass, several) {
  const list = h('dl', { class: 'readings' });
  for (const reading of mass.readings) {
    const line = h('dd', {}, await readingButton(ctx, reading.osis, reading.ref));
    for (const alternative of reading.alternatives) {
      line.append(h('span', { class: 'fine' }, ' or '), await readingButton(ctx, alternative.osis, alternative.ref));
    }
    list.append(h('dt', {}, slotLabel(reading.slot)), line);
  }
  return several || mass.id !== 'day' ? h('section', { class: 'mass' }, h('h4', {}, mass.label || mass.id), list) : list;
}

async function gospelVideos(ctx, day) {
  if (ctx.settings().videos !== true) return null;
  const gospel = day.masses.flatMap((m) => m.readings).find((r) => r.slot === 'gospel');
  if (!gospel) return null;
  const ranges = rangesOf(gospel.osis);
  const { channels, videos } = await videoCatalog();
  const anchors = ctx.user().video_anchors || [];
  const found = videos.filter((v) => ranges.some((r) => videoTouches(v, anchors, r.start, r.end))).slice(0, 3);
  if (!found.length) return null;
  const byId = new Map(channels.map((c) => [c.id, c]));
  return h('details', { class: 'liturgy-videos' }, h('summary', {}, `Videos on this Gospel (${found.length})`),
    found.map((v) => videoCard(v, byId.get(v.channel), ctx, () => {})));
}

function notice() {
  const box = h('details', { class: 'fine' }, h('summary', {}, 'Where the calendar comes from'));
  box.addEventListener('toggle', async () => {
    if (!box.open || box.dataset.drawn) return;
    box.dataset.drawn = '1';
    box.append(h('pre', { class: 'notice' }, await api.liturgyNotice().catch((error) => String(error))));
  });
  return box;
}

let shown = null;

/** The section for one date; the arrows move it a day at a time. */
export async function liturgySection(ctx, date = shown || isoDate(new Date())) {
  shown = date;
  const section = h('section', { class: 'liturgy' });
  const draw = async (iso) => {
    shown = iso;
    const day = await api.liturgy(iso);
    const italian = ctx.module().language === 'it';
    const name = (c) => (italian ? c.it : c.en);
    const today = isoDate(new Date());
    const [y, m, d] = iso.split('-').map(Number);
    const when = new Date(y, m - 1, d).toLocaleDateString(italian ? 'it-IT' : 'en-GB', { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' });
    const masses = await Promise.all(day.masses.map((mass) => massList(ctx, mass, day.masses.length > 1)));
    const own = day.optionalMasses.length
      ? h('details', {}, h('summary', {}, 'Readings of the optional memorial'), await Promise.all(day.optionalMasses.map((mass) => massList(ctx, mass, true))))
      : null;
    clear(section,
      h('div', { class: 'row liturgy-nav' },
        h('button', { type: 'button', class: 'quiet', 'aria-label': 'Previous day', onclick: () => draw(shiftDate(iso, -1)) }, '‹'),
        h('span', { class: 'grow' }, when),
        iso !== today ? h('button', { type: 'button', class: 'quiet', onclick: () => draw(today) }, 'Today') : null,
        h('button', { type: 'button', class: 'quiet', 'aria-label': 'Next day', onclick: () => draw(shiftDate(iso, 1)) }, '›')),
      h('h3', {}, h('span', { class: `swatch colour-${day.color}`, title: day.color, 'aria-hidden': 'true' }), name(day.celebration)),
      h('p', { class: 'fine' }, [GRADES[day.celebration.grade], SEASONS[day.season], `Sunday cycle ${day.sundayCycle}`, `weekday cycle ${day.weekdayCycle}`].filter(Boolean).join(' · ')),
      day.optional.length ? h('p', { class: 'fine' }, `Also: ${day.optional.map(name).join('; ')}`) : null,
      masses,
      own,
      await gospelVideos(ctx, day).catch(() => null),
      h('p', { class: 'fine' }, 'The calendar of Italy, computed offline. The references follow the Lectionary of 1981; open them in any installed translation.'),
      notice());
  };
  await draw(date);
  return section;
}
