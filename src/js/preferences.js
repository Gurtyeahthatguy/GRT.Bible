/** The settings dialog: reading, copying, videos and the reader's file. */

import { api, ask, onAndroid, pickFile, pickSave } from './api.js';
import { h } from './dom.js';
import { openGuide } from './guide.js';
import { KIND_LABELS } from './library.js';
import { closeOverlay, openOverlay, toast } from './overlay.js';
import { THEMES } from './settings.js';

function field(label, control, note = null) {
  return h('label', { class: 'field' }, h('span', { class: 'field-label' }, label), control, note ? h('span', { class: 'fine' }, note) : null);
}

function check(label, value, onchange) {
  const box = h('input', { type: 'checkbox' });
  box.checked = value;
  box.addEventListener('change', () => onchange(box.checked));
  return h('label', { class: 'check' }, box, h('span', {}, label));
}

function choice(name, options, value, onchange) {
  return h('div', { class: 'choice', role: 'radiogroup' },
    options.map(([v, label]) => {
      const radio = h('input', { type: 'radio', name, value: v });
      radio.checked = v === value;
      radio.addEventListener('change', () => radio.checked && onchange(v));
      return h('label', { class: 'check' }, radio, h('span', {}, label));
    }));
}

export function openPreferences(ctx) {
  const s = ctx.settings();
  const set = (key, value) => ctx.setSetting(key, value);

  const theme = h('select', {}, THEMES.map((t) => h('option', { value: t.id }, t.label)));
  theme.value = s.theme;
  theme.addEventListener('change', () => set('theme', theme.value));

  const size = h('input', { type: 'range', min: 12, max: 36, value: s.fontSize });
  const sizeValue = h('output', {}, `${s.fontSize} px`);
  size.addEventListener('input', () => {
    sizeValue.textContent = `${size.value} px`;
    set('fontSize', Number(size.value));
  });

  const width = h('input', { type: 'range', min: 60, max: 70, value: s.lineWidth });
  const widthValue = h('output', {}, `${s.lineWidth} characters`);
  width.addEventListener('input', () => {
    widthValue.textContent = `${width.value} characters`;
    set('lineWidth', Number(width.value));
  });

  const counts = Object.entries(KIND_LABELS)
    .map(([kind, [, plural]]) => [plural, ctx.allModules().filter((m) => m.kind === kind).length])
    .filter(([, n]) => n > 0)
    .map(([plural, n]) => `${n} ${plural.toLowerCase()}`);

  const body = h('div', { class: 'preferences' },
    h('h2', {}, 'Settings'),
    h('section', {},
      h('h3', {}, 'Reading'),
      field('Theme', theme),
      field('Text size', h('span', { class: 'row' }, size, sizeValue)),
      field('Line length', h('span', { class: 'row' }, width, widthValue)),
      h('div', { class: 'field' }, h('span', { class: 'field-label' }, 'Layout'),
        choice('layout', [['paragraphs', 'Continuous paragraphs'], ['verses', 'One verse per line']], s.layout, (v) => set('layout', v))),
      check('Verse numbers', s.verseNumbers, (v) => set('verseNumbers', v)),
      check('Section headings and chapter summaries', s.titles, (v) => set('titles', v)),
      check('Notes of the edition', s.sourceNotes, (v) => set('sourceNotes', v))),
    h('section', {},
      h('h3', {}, 'Copying'),
      h('div', { class: 'field' }, h('span', { class: 'field-label' }, 'Reference'),
        choice('refpos', [['after', 'After the text'], ['before', 'Before the text']], s.copy.refPosition, (v) => set('copy', { ...ctx.settings().copy, refPosition: v }))),
      check('Include the translation abbreviation', s.copy.includeModule, (v) => set('copy', { ...ctx.settings().copy, includeModule: v })),
      check('Include verse numbers', s.copy.includeNumbers, (v) => set('copy', { ...ctx.settings().copy, includeNumbers: v }))),
    h('section', {},
      h('h3', {}, 'Videos'),
      check('Show the video section', s.videos === true, (v) => set('videos', v)),
      h('p', { class: 'fine' }, `Videos are chosen by the author and open in your ${onAndroid() ? 'browser or video app' : 'web browser'}, after checking the site is on the allowed list. Nothing about your reading is sent anywhere, and GRT Bible is not affiliated with the channels it lists.`)),
    h('section', {},
      h('h3', {}, 'Your data'),
      h('p', { class: 'fine' }, 'Reading position, history, notes, highlights, bookmarks, saved videos and your video lists live in one .grt file.'),
      h('div', { class: 'row' },
        h('button', {
          onclick: async () => {
            const path = await pickSave('Export your data', 'GRT Bible.grt', [{ name: 'GRT file', extensions: ['grt'] }]);
            if (!path) return;
            try {
              await ctx.flushUser();
              await api.exportUser(path);
              toast('Your data was exported.');
            } catch (error) {
              toast(String(error), 5000);
            }
          },
        }, 'Export…'),
        h('button', {
          onclick: async () => {
            const path = await pickFile('Import your data', [{ name: 'GRT file', extensions: ['grt'] }]);
            if (!path) return;
            if (!(await ask('Replace your notes, highlights, bookmarks and history with the ones in this file?'))) return;
            try {
              ctx.replaceUser(await api.importUser(path));
              toast('Your data was imported.');
            } catch (error) {
              toast(String(error), 5000);
            }
          },
        }, 'Import…'))),
    h('section', {},
      h('h3', {}, 'Modules'),
      h('p', { class: 'fine' }, `Installed: ${counts.join(', ')}. Translations, commentaries, cross-references, dictionaries and books are listed, installed and removed in the library.`),
      h('div', { class: 'row' },
        h('button', { onclick: () => ctx.openLibrary('installed') }, 'Open the library'),
        h('button', { class: 'quiet', onclick: () => openGuide() }, 'How to install modules'))),
    h('section', {},
      h('h3', {}, 'About'),
      h('p', { class: 'fine' }, onAndroid()
        ? `GRT Bible ${ctx.version()}. Public domain texts, read on this phone. GRT Bible has no permission to use the internet; only the video section hands links to your browser or video app.`
        : `GRT Bible ${ctx.version()}. Public domain texts, read on this computer. GRT Bible does not connect to the internet; only the video section hands links to your browser.`)),
    h('div', { class: 'row end' }, h('button', { class: 'primary', onclick: closeOverlay }, 'Done')));
  openOverlay(body, { label: 'Settings', wide: true });
}

export function askAboutVideos(ctx) {
  const body = h('div', { class: 'first-run' },
    h('h2', {}, 'Show the video section?'),
    h('p', {}, `GRT Bible can list Catholic videos about the passage you are reading, chosen by the author and not connected with the channels that published them. They open in your ${onAndroid() ? 'browser or video app' : 'web browser'}.`),
    h('p', { class: 'fine' }, 'The Bible itself works entirely offline either way. You can change this later in Settings.'),
    h('div', { class: 'row end' },
      h('button', { onclick: () => { ctx.setSetting('videos', false); closeOverlay(); } }, 'No, thanks'),
      h('button', { class: 'primary', onclick: () => { ctx.setSetting('videos', true); closeOverlay(); } }, 'Show videos')));
  openOverlay(body, { label: 'Video section', onClose: () => { if (ctx.settings().videos === null) ctx.setSetting('videos', false); } });
}
