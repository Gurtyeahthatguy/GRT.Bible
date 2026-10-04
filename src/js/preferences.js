/** The settings dialog: reading, copying, videos, the reader's file and the installed translations. */

import { api, ask, onAndroid, pickFile, pickFolder, pickSave } from './api.js';
import { clear, h } from './dom.js';
import { closeOverlay, openOverlay, toast } from './overlay.js';
import { THEMES } from './settings.js';

const SCHEME_LABELS = {
  hebrew: 'Hebrew numbering',
  vulgate: 'Vulgate numbering',
  english: 'English numbering',
  lxx: 'Septuagint numbering',
  martini: 'Martini numbering',
  douay: 'Douay-Rheims numbering',
  webc: 'World English Bible numbering',
};

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

function importDialog(ctx) {
  const body = h('div', { class: 'import' });
  const status = h('p', { class: 'fine', 'aria-live': 'polite' });

  const chooseStep = () => {
    const android = onAndroid();
    clear(body,
      h('h2', {}, 'Import a translation'),
      h('p', {}, android
        ? 'Choose a SWORD module as a ZIP, or USFM files as a ZIP or a single file. Only import texts you have the right to use.'
        : 'Choose a SWORD module (a ZIP or its folder) or USFM files (a ZIP, a folder or a single file). Only import texts you have the right to use.'),
      h('div', { class: 'row' },
        h('button', { onclick: async () => inspect(await pickFile('Choose a module file', [{ name: 'SWORD or USFM', extensions: ['zip', 'usfm', 'sfm'] }])) }, 'Choose a file…'),
        android ? null : h('button', { onclick: async () => inspect(await pickFolder('Choose a module folder')) }, 'Choose a folder…')),
      status);
  };

  const inspect = async (path) => {
    if (!path) return;
    status.textContent = 'Reading the file…';
    try {
      const found = await api.inspectImport(path);
      formStep(path, found);
    } catch (error) {
      status.textContent = String(error);
    }
  };

  const formStep = (path, found) => {
    const abbreviation = h('input', { type: 'text', value: found.abbreviation, maxlength: 10, required: true });
    const title = h('input', { type: 'text', value: found.title, required: true });
    const language = h('input', { type: 'text', value: found.language, placeholder: 'it, en, la…', maxlength: 8, required: true });
    const best = found.ranking[0]?.[0];
    const scheme = h('select', {}, found.ranking.map(([name, score]) =>
      h('option', { value: name }, `${SCHEME_LABELS[name] || name}${name === best ? ' (best fit)' : ''}${score ? `, ${score} mismatches` : ''}`)));
    scheme.value = found.versification;
    const go = h('button', { class: 'primary' }, 'Import');
    const result = h('p', { class: 'fine', 'aria-live': 'polite' });
    go.addEventListener('click', async () => {
      go.disabled = true;
      result.textContent = 'Converting… this can take a minute.';
      try {
        const info = await api.importModule(path, {
          abbreviation: abbreviation.value.trim(),
          title: title.value.trim(),
          language: language.value.trim(),
          versification: scheme.value,
        });
        closeOverlay();
        toast(`${info.abbreviation} installed. ${info.summary}`, 6000);
        ctx.modulesChanged(info.abbreviation);
      } catch (error) {
        result.textContent = String(error);
        go.disabled = false;
      }
    });
    clear(body,
      h('h2', {}, 'Import a translation'),
      h('p', {}, `${found.kind === 'sword' ? 'SWORD module' : 'USFM text'} with ${found.books} books of the canon.`),
      found.warnings.length ? h('details', {}, h('summary', {}, `${found.warnings.length} notes from the converter`), h('ul', {}, found.warnings.map((w) => h('li', {}, w)))) : null,
      field('Abbreviation', abbreviation, 'Shown in the translation list and in copied references.'),
      field('Title', title),
      field('Language code', language),
      field('Verse numbering', scheme, 'Notes and highlights stay on the right verse across translations only if this is right.'),
      h('div', { class: 'row' }, h('button', { onclick: chooseStep }, 'Back'), go),
      result);
  };

  chooseStep();
  openOverlay(body, { label: 'Import a translation' });
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

  const modules = h('div', { class: 'module-list' });
  const drawModules = () => {
    clear(modules, ctx.modules().map((m) => h('article', { class: 'card' },
      h('header', {}, h('strong', {}, `${m.abbreviation} · ${m.title}`),
        m.bundled ? null : h('button', {
          class: 'quiet danger',
          onclick: async () => {
            if (!(await ask(`Remove ${m.abbreviation} from this computer? Your notes are not affected.`))) return;
            try {
              await api.removeModule(m.abbreviation);
              ctx.modulesChanged(null);
              drawModules();
            } catch (error) {
              toast(String(error), 5000);
            }
          },
        }, 'Remove')),
      h('p', { class: 'fine' }, [m.language, m.year, SCHEME_LABELS[m.versification] || m.versification, m.license].filter(Boolean).join(' · ')),
      h('p', { class: 'fine' }, m.summary),
      m.source ? h('p', { class: 'fine' }, `Source: ${m.source}`) : null)));
  };
  drawModules();

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
      h('p', { class: 'fine' }, 'Reading position, history, notes, highlights, bookmarks and saved videos live in one .grt file.'),
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
      h('h3', {}, 'Translations'),
      modules,
      h('button', { onclick: () => importDialog(ctx) }, 'Import a translation…')),
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
