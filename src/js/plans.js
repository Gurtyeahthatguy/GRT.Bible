/** Reading plans with their progress, and the same plan as tasks in GRT Dates. */

import { api, ask, pickFile, pickSave } from './api.js';
import { h } from './dom.js';
import { rangeLabels } from './library.js';
import { joinLabels, shiftDate, isoDate } from './liturgy.js';
import { splitRange } from './markup.js';
import { toast } from './overlay.js';

/** The day of a plan that falls on a date, from 1; 0 before it starts. */
export function planDay(start, today) {
  const [a, b] = [start, today].map((iso) => {
    const [y, m, d] = iso.split('-').map(Number);
    return Date.UTC(y, m - 1, d);
  });
  return Math.floor((b - a) / 86_400_000) + 1;
}

function escapeText(text) {
  return text.replace(/\\/g, '\\\\').replace(/;/g, '\\;').replace(/,/g, '\\,').replace(/\n/g, '\\n');
}

function fold(line) {
  const encoder = new TextEncoder();
  const pieces = [];
  let current = '';
  let width = 0;
  for (const ch of line) {
    const size = encoder.encode(ch).length;
    if (width + size > 75) {
      pieces.push(current);
      current = ' ';
      width = 1;
    }
    current += ch;
    width += size;
  }
  pieces.push(current);
  return pieces.join('\r\n');
}

export function taskUid(progress, day) {
  return `grt-bible-${progress.plan}-${progress.start}-day${day}@grt`;
}

/** A plan as an iCalendar file of tasks, one a day, that GRT Dates opens as a calendar. */
export function planIcs(title, progress, labels) {
  const done = new Set(progress.done);
  const lines = ['BEGIN:VCALENDAR', 'VERSION:2.0', 'PRODID:-//GRT//Bible//EN', `X-WR-CALNAME:${escapeText(title)}`];
  labels.forEach((label, i) => {
    const day = i + 1;
    const due = shiftDate(progress.start, i).replace(/-/g, '');
    lines.push('BEGIN:VTODO', `UID:${taskUid(progress, day)}`, 'DTSTAMP:19800101T000000Z',
      `SUMMARY:${escapeText(`Read ${label}`)}`,
      `DESCRIPTION:${escapeText(`${title}, day ${day} of ${labels.length}`)}`,
      `DUE;VALUE=DATE:${due}`,
      `STATUS:${done.has(day) ? 'COMPLETED' : 'NEEDS-ACTION'}`);
    if (done.has(day)) lines.push('PERCENT-COMPLETE:100');
    lines.push('END:VTODO');
  });
  lines.push('END:VCALENDAR');
  return `${lines.map(fold).join('\r\n')}\r\n`;
}

/** The days of a plan an iCalendar file marks as done, found by the ids planIcs gave them. */
export function doneInIcs(text, progress) {
  const unfolded = text.replace(/\r\n?/g, '\n').replace(/\n[ \t]/g, '');
  const prefix = `grt-bible-${progress.plan}-${progress.start}-day`;
  const done = [];
  for (const block of unfolded.split('BEGIN:VTODO').slice(1)) {
    const body = block.split('END:VTODO')[0];
    const uid = /^UID:(.*)$/m.exec(body)?.[1]?.trim() || '';
    if (!uid.startsWith(prefix)) continue;
    const day = Number.parseInt(uid.slice(prefix.length), 10);
    const completed = /^STATUS:COMPLETED\s*$/mi.test(body) || /^COMPLETED[:;]/m.test(body) || /^PERCENT-COMPLETE:100\s*$/m.test(body);
    if (Number.isInteger(day) && completed) done.push(day);
  }
  return done.sort((a, b) => a - b);
}

const schedules = new Map();

async function schedule(id) {
  if (!schedules.has(id)) schedules.set(id, await api.planSchedule(id));
  return schedules.get(id);
}

async function dayLabels(ctx, days) {
  const ranges = days.map((d) => d.map((r) => splitRange(r)).filter(Boolean));
  const flat = await rangeLabels(ctx, ranges.flat());
  let i = 0;
  return ranges.map((r) => joinLabels(r.map(() => flat[i++])));
}

function readButton(ctx, ranges, label) {
  const button = h('button', { type: 'button', class: 'link ref', 'aria-expanded': 'false' }, label);
  const parts = ranges.map((r) => splitRange(r)).filter(Boolean);
  const last = parts[parts.length - 1];
  const sameBook = last.start.split('.')[0] === parts[0].start.split('.')[0];
  button.addEventListener('click', () => ctx.openRange(parts[0].start, sameBook ? last.end : parts[0].end));
  return button;
}

async function planCard(ctx, plan, progress) {
  const user = ctx.user();
  const days = await schedule(plan.id);
  const today = planDay(progress.start, isoDate(new Date()));
  const done = new Set(progress.done);
  const toggle = (day, on) => {
    progress.done = on ? [...new Set([...progress.done, day])].sort((a, b) => a - b) : progress.done.filter((d) => d !== day);
    ctx.saveUser();
  };
  const check = (day) => {
    const box = h('input', { type: 'checkbox', 'aria-label': `Day ${day} read` });
    box.checked = done.has(day);
    box.addEventListener('change', () => {
      toggle(day, box.checked);
      ctx.refresh();
    });
    return box;
  };
  const current = Math.min(Math.max(today, 1), days.length);
  const firstOpen = days.findIndex((_, i) => !done.has(i + 1)) + 1;
  const focus = firstOpen > 0 && firstOpen < current ? firstOpen : current;
  const [focusLabel] = await dayLabels(ctx, [days[focus - 1]]);
  const all = h('details', {}, h('summary', {}, 'Every day'));
  all.addEventListener('toggle', async () => {
    if (!all.open || all.dataset.drawn) return;
    all.dataset.drawn = '1';
    const labels = await dayLabels(ctx, days);
    all.append(h('ol', { class: 'plan-days' }, days.map((ranges, i) => h('li', {}, h('label', {}, check(i + 1), ' '),
      readButton(ctx, ranges, labels[i]), h('span', { class: 'fine' }, ` · ${shiftDate(progress.start, i)}`)))));
  });
  const exportIcs = h('button', {
    class: 'quiet',
    title: 'One task a day, for GRT Dates or any calendar',
    onclick: async () => {
      const path = await pickSave('Send the plan to GRT Dates', `${plan.title}.ics`, [{ name: 'iCalendar', extensions: ['ics'] }]);
      if (!path) return;
      try {
        await api.saveText(path, planIcs(plan.title, progress, await dayLabels(ctx, days)));
        toast('Open the file in GRT Dates. Tick the tasks there and bring the file back here with Update from Dates.', 6000);
      } catch (error) {
        toast(String(error), 5000);
      }
    },
  }, 'Send to Dates…');
  const importIcs = h('button', {
    class: 'quiet',
    title: 'Mark as read the days ticked in GRT Dates',
    onclick: async () => {
      const path = await pickFile('Update from GRT Dates', [{ name: 'iCalendar', extensions: ['ics'] }]);
      if (!path) return;
      try {
        const found = doneInIcs(await api.readText(path), progress).filter((day) => day >= 1 && day <= days.length);
        const before = progress.done.length;
        progress.done = [...new Set([...progress.done, ...found])].sort((a, b) => a - b);
        ctx.saveUser();
        ctx.refresh();
        toast(found.length ? `${progress.done.length - before} more days read.` : 'This calendar has no ticked day of this plan.');
      } catch (error) {
        toast(String(error), 5000);
      }
    },
  }, 'Update from Dates…');
  const stop = h('button', {
    class: 'quiet danger',
    onclick: async () => {
      if (!await ask(`Stop “${plan.title}”? The days you have read are forgotten.`)) return;
      user.plans = user.plans.filter((p) => p !== progress);
      ctx.saveUser();
      ctx.refresh();
    },
  }, 'Stop');
  const status = today < 1 ? `Starts on ${progress.start}` : today > days.length ? 'The plan is over' : `Day ${today} of ${days.length}`;
  return h('article', { class: 'card plan' },
    h('header', {}, h('strong', {}, plan.title), stop),
    h('p', { class: 'fine' }, `${status} · ${progress.done.length} of ${days.length} days read`),
    h('progress', { max: days.length, value: progress.done.length }),
    h('p', {}, h('label', {}, check(focus), ` Day ${focus}: `), readButton(ctx, days[focus - 1], focusLabel)),
    all,
    h('div', { class: 'row' }, exportIcs, importIcs));
}

export async function renderPlans(container, ctx) {
  const user = ctx.user();
  const plans = await api.plans();
  for (const progress of user.plans) {
    const plan = plans.find((p) => p.id === progress.plan);
    if (plan) container.append(await planCard(ctx, plan, progress));
  }
  const free = plans.filter((p) => !user.plans.some((x) => x.plan === p.id));
  if (!free.length) return;
  container.append(h('h3', {}, user.plans.length ? 'Start another plan' : 'Start a plan'),
    h('ul', { class: 'plan-list' }, free.map((p) => h('li', {},
      h('span', {}, p.title),
      h('button', {
        type: 'button',
        onclick: () => {
          user.plans.push({ plan: p.id, start: isoDate(new Date()), done: [] });
          ctx.saveUser();
          ctx.refresh();
        },
      }, 'Start today')))));
}

