/** The library: what the installed material says about a passage, dictionaries, books, and the modules themselves. */

import { api, ask, onAndroid, pickFile, pickFolder } from './api.js';
import { clear, h, withBreaks } from './dom.js';
import { openGuide } from './guide.js';
import { renderMarkup, splitRange } from './markup.js';
import { closeOverlay, openOverlay, toast } from './overlay.js';
import { formatReference, splitOsis } from './reference.js';

export const LIBRARY_TABS = [
  { id: 'passage', label: 'This passage' },
  { id: 'dictionary', label: 'Dictionary', kind: 'dictionary' },
  { id: 'books', label: 'Books', kind: 'book' },
  { id: 'installed', label: 'Installed' },
];

export const KIND_LABELS = {
  bible: ['Bible', 'Bibles'],
  commentary: ['Commentary', 'Commentaries'],
  crossrefs: ['Cross-references', 'Cross-references'],
  dictionary: ['Dictionary or lexicon', 'Dictionaries and lexicons'],
  book: ['Book', 'Books'],
};

const SCHEME_LABELS = {
  hebrew: 'Hebrew numbering',
  vulgate: 'Vulgate numbering',
  english: 'English numbering',
  lxx: 'Septuagint numbering',
  martini: 'Martini numbering',
  douay: 'Douay-Rheims numbering',
  webc: 'World English Bible numbering',
};

/** Verses shown in a preview before it offers the whole passage instead. */
const PREVIEW_VERSES = 12;

export function schemeLabel(name) {
  return SCHEME_LABELS[name] || name;
}

function empty(text) {
  return h('p', { class: 'empty-state' }, text);
}

/** Tabs worth showing: a dictionary tab only with a dictionary installed, and so on. */
export function libraryTabs(modules) {
  return LIBRARY_TABS.filter((t) => !t.kind || modules.some((m) => m.kind === t.kind));
}

/** Labels for internal ranges in the reader's translation, such as "Gv 3,16-18". */
async function rangeLabels(ctx, ranges) {
  const module = ctx.module();
  const refs = ranges.flatMap((r) => [r.start, r.end]);
  if (!refs.length) return [];
  let places = [];
  try {
    places = await api.resolve(module.abbreviation, refs);
  } catch {
    places = [];
  }
  return ranges.map((r, i) => {
    const a = places[i * 2];
    const b = places[i * 2 + 1];
    if (!a) {
      const s = splitOsis(r.start);
      return s ? formatReference(module.language, s.book, s.chapter, s.verse) : r.start;
    }
    const own = module.books.find((x) => x.osis === a.book);
    if (!b || b.book !== a.book) return formatReference(module.language, a.book, a.chapter, a.verse, a.chapter, a.verse, own?.abbrev);
    return formatReference(module.language, a.book, a.chapter, a.verse, b.chapter, b.verse, own?.abbrev);
  });
}

/** The verses of a range in the reader's translation, with a way to read them in place. */
function preview(ctx, range, anchor) {
  const next = anchor.nextElementSibling;
  if (next?.classList.contains('preview')) {
    next.remove();
    anchor.setAttribute('aria-expanded', 'false');
    return;
  }
  const module = ctx.module();
  const box = h('div', { class: 'preview', lang: module.language, dir: module.direction || 'ltr' }, h('p', { class: 'fine' }, 'Reading…'));
  anchor.after(box);
  anchor.setAttribute('aria-expanded', 'true');
  api.passage(module.abbreviation, range.start, range.end).then((verses) => {
    if (!verses.length) {
      clear(box, h('p', { class: 'fine' }, `This passage is not in ${module.abbreviation}.`));
      return;
    }
    const shown = verses.slice(0, PREVIEW_VERSES);
    clear(box,
      h('p', {}, shown.map((v) => [h('sup', { class: 'vn' }, String(v.verse)), withBreaks(v.text), ' '])),
      verses.length > shown.length ? h('p', { class: 'fine' }, `${verses.length - shown.length} more verses`) : null,
      h('div', { class: 'row' },
        h('button', { type: 'button', onclick: () => ctx.openRange(range.start, range.end) }, `Open in ${module.abbreviation}`)));
  }).catch((error) => clear(box, h('p', { class: 'fine' }, String(error))));
}

/** Markup handlers shared by every tab: passages preview in place, headwords open in the dictionary. */
function handlersFor(ctx, module = null) {
  return {
    onRef: (range, button) => preview(ctx, range, button),
    onKey: (key) => ctx.openEntry?.(module, key),
  };
}

// This passage

async function crossReferences(ctx, at) {
  const section = h('section', { class: 'lib-section' }, h('h3', {}, 'Cross-references'));
  if (!ctx.modules().some((m) => m.kind === 'crossrefs')) {
    section.append(h('p', { class: 'fine' }, 'No cross-references are installed. The Treasury of Scripture Knowledge is published as a module beside each release.'));
    return section;
  }
  const sets = await api.crossrefs(at);
  const all = sets.flatMap((s) => s.groups.flatMap((g) => g.targets));
  const ranges = all.map((t) => splitRange(t)).filter(Boolean);
  const labels = await rangeLabels(ctx, ranges);
  const labelOf = new Map(all.map((t, i) => [t, labels[i]]));
  for (const set of sets) {
    const body = h('div', { class: 'groups' });
    if (!set.groups.length) body.append(h('p', { class: 'fine' }, 'Nothing for this verse.'));
    for (const group of set.groups) {
      const line = h('p', { class: 'group' }, group.label ? h('b', {}, `${group.label} `) : null);
      for (const target of group.targets) {
        const range = splitRange(target);
        if (!range) continue;
        const button = h('button', { class: 'link ref', type: 'button', 'aria-expanded': 'false' }, labelOf.get(target) || target);
        button.addEventListener('click', () => preview(ctx, range, button));
        line.append(button, ' ');
      }
      body.append(line);
    }
    section.append(h('details', { open: true }, h('summary', {}, `${set.module} · ${set.title}`), body));
  }
  return section;
}

function spanLabel(ctx, span, label) {
  const range = splitRange(span);
  if (!range) return label;
  const s = splitOsis(range.start);
  if (s && s.verse === 0 && s.chapter === 0) return `Introduction to ${ctx.bookName(s.book)}`;
  if (s && s.verse === 0) return `Introduction to ${ctx.bookName(s.book)} ${s.chapter}`;
  return label;
}

async function commentarySection(ctx, at) {
  const commentaries = ctx.modules().filter((m) => m.kind === 'commentary');
  const section = h('section', { class: 'lib-section' }, h('h3', {}, 'Commentary'));
  if (!commentaries.length) {
    section.append(h('p', { class: 'fine' }, 'No commentary is installed. Install one from the Installed tab.'));
    return section;
  }
  const chosen = commentaries.find((m) => m.abbreviation === ctx.settings().library.commentary) || commentaries[0];
  const picker = h('select', { 'aria-label': 'Commentary' }, commentaries.map((m) => h('option', { value: m.abbreviation }, `${m.abbreviation} · ${m.title}`)));
  picker.value = chosen.abbreviation;
  picker.addEventListener('change', () => {
    ctx.setSetting('library', { ...ctx.settings().library, commentary: picker.value });
    ctx.refreshLibrary();
  });
  const step = (forward) => async () => {
    try {
      const target = await api.commentaryStep(chosen.abbreviation, at, forward);
      if (target) ctx.openRange(target, target);
      else toast(forward ? 'The commentary ends here.' : 'The commentary starts here.');
    } catch (error) {
      toast(String(error), 5000);
    }
  };
  section.append(h('div', { class: 'row' }, picker,
    h('button', { type: 'button', class: 'quiet', 'aria-label': 'Previous comment', title: 'Previous comment', onclick: step(false) }, '‹'),
    h('button', { type: 'button', class: 'quiet', 'aria-label': 'Next comment', title: 'Next comment', onclick: step(true) }, '›')));
  const entries = await api.commentary(chosen.abbreviation, at);
  if (!entries.length) {
    section.append(h('p', { class: 'fine' }, `${chosen.abbreviation} says nothing about this verse.`));
    return section;
  }
  const spans = entries.map((e) => splitRange(e.spans[0]) || { start: at, end: at });
  const labels = await rangeLabels(ctx, spans);
  const handlers = handlersFor(ctx);
  for (const [i, entry] of entries.entries()) {
    const label = spanLabel(ctx, entry.spans[0], labels[i]);
    const intro = /\.0(-|$)/.test(entry.spans[0] || '');
    const article = h('article', { class: 'entry', lang: chosen.language, dir: chosen.direction || 'ltr' }, renderMarkup(entry.body, handlers));
    section.append(intro
      ? h('details', { class: 'entry-wrap' }, h('summary', {}, label), article)
      : h('div', { class: 'entry-wrap' }, h('h4', { class: 'entry-label' }, label), article));
  }
  return section;
}

async function passageTab(container, ctx) {
  const at = ctx.focus();
  if (!at) {
    container.append(empty('Open a chapter to see what the library says about it.'));
    return;
  }
  const [label] = await rangeLabels(ctx, [{ start: at, end: at }]);
  container.append(h('p', { class: 'lib-focus' }, h('strong', {}, label),
    h('span', { class: 'fine' }, ctx.hasSelection() ? ' · the selected verse' : ' · the first verse in view, or select one')));
  container.append(await crossReferences(ctx, at), await commentarySection(ctx, at));
}

// Dictionary

async function dictionaryTab(container, ctx, options = {}) {
  const dictionaries = ctx.modules().filter((m) => m.kind === 'dictionary');
  if (!dictionaries.length) {
    container.append(empty('No dictionary is installed.'));
    return;
  }
  const settings = ctx.settings().library;
  const chosen = dictionaries.find((m) => m.abbreviation === (options.module || settings.dictionary)) || dictionaries[0];
  const picker = h('select', { 'aria-label': 'Dictionary' }, dictionaries.map((m) => h('option', { value: m.abbreviation }, `${m.abbreviation} · ${m.title}`)));
  picker.value = chosen.abbreviation;
  const input = h('input', { type: 'search', placeholder: 'Look up a word', 'aria-label': 'Look up a word', autocapitalize: 'none', value: options.key || '' });
  const list = h('ul', { class: 'headwords' });
  const article = h('article', { class: 'entry', lang: chosen.language, dir: chosen.direction || 'ltr', 'aria-live': 'polite' });

  const show = (row) => {
    if (!row) {
      clear(article, h('p', { class: 'fine' }, 'No entry with that headword.'));
      return;
    }
    const handlers = {
      onRef: (range, button) => preview(ctx, range, button),
      onKey: async (key) => {
        input.value = key;
        await search();
        show(await api.entryByKey(chosen.abbreviation, key));
      },
    };
    clear(article, h('h3', {}, row.title || row.key), renderMarkup(row.body, handlers));
    article.scrollIntoView?.({ block: 'nearest' });
  };

  const search = async () => {
    const found = await api.lookup(chosen.abbreviation, input.value, 80);
    clear(list, found.map((row) => h('li', {}, h('button', {
      type: 'button',
      class: 'link',
      onclick: async () => show(await api.entry(chosen.abbreviation, row.id)),
    }, row.title || row.key))));
    if (!found.length) list.append(h('li', { class: 'fine' }, 'Nothing starts with that.'));
    return found;
  };

  picker.addEventListener('change', () => {
    ctx.setSetting('library', { ...ctx.settings().library, dictionary: picker.value });
    ctx.refreshLibrary();
  });
  let timer = null;
  input.addEventListener('input', () => {
    clearTimeout(timer);
    timer = setTimeout(search, 150);
  });
  input.addEventListener('keydown', async (event) => {
    if (event.key !== 'Enter') return;
    const exact = await api.entryByKey(chosen.abbreviation, input.value);
    if (exact) show(exact);
  });
  container.append(h('div', { class: 'search-form' }, picker, input), list, article);
  const found = await search();
  if (options.key) show(await api.entryByKey(chosen.abbreviation, options.key) || (found[0] && await api.entry(chosen.abbreviation, found[0].id)));
}

// Books

async function booksTab(container, ctx) {
  const books = ctx.modules().filter((m) => m.kind === 'book');
  if (!books.length) {
    container.append(empty('No book is installed.'));
    return;
  }
  const chosen = books.find((m) => m.abbreviation === ctx.settings().library.book) || books[0];
  const picker = h('select', { 'aria-label': 'Book' }, books.map((m) => h('option', { value: m.abbreviation }, `${m.abbreviation} · ${m.title}`)));
  picker.value = chosen.abbreviation;
  picker.addEventListener('change', () => {
    ctx.setSetting('library', { ...ctx.settings().library, book: picker.value });
    ctx.refreshLibrary();
  });
  const trail = h('nav', { class: 'trail', 'aria-label': 'Sections' });
  const list = h('ul', { class: 'sections' });
  const article = h('article', { class: 'entry', lang: chosen.language, dir: chosen.direction || 'ltr' });
  const path = [];

  const open = async (heading) => {
    if (heading) path.push(heading);
    const parent = path.length ? path[path.length - 1] : null;
    const children = await api.sections(chosen.abbreviation, parent ? parent.id : null);
    clear(trail,
      h('button', { type: 'button', class: 'link', onclick: () => { path.length = 0; open(null); } }, chosen.abbreviation),
      path.map((p, i) => [' › ', h('button', {
        type: 'button',
        class: 'link',
        onclick: () => { path.length = i + 1; const last = path.pop(); open(last); },
      }, p.title || '·')]));
    clear(list, children.map((c) => h('li', {}, h('button', { type: 'button', class: 'link', onclick: () => open(c) },
      c.title || '·', c.children ? h('span', { class: 'muted' }, ` (${c.children})`) : null))));
    clear(article);
    if (parent) {
      const row = await api.entry(chosen.abbreviation, parent.id);
      if (row?.body) article.append(renderMarkup(row.body, handlersFor(ctx)));
    }
  };
  container.append(picker, trail, list, article);
  await open(null);
}

// Installed

function moduleCard(m, ctx, redraw) {
  const facts = [KIND_LABELS[m.kind]?.[0] || m.kind, m.language, m.year,
    m.kind === 'bible' || m.kind === 'commentary' || m.kind === 'crossrefs' ? schemeLabel(m.versification) : null].filter(Boolean);
  const origin = m.bundled
    ? 'Comes with the program.'
    : m.origin ? `Installed from ${m.origin.file} on ${m.origin.installed}.` : 'Installed from a file.';
  return h('article', { class: 'card module', dataset: { module: m.abbreviation } },
    h('header', {}, h('strong', {}, `${m.abbreviation} · ${m.title}`),
      m.bundled ? null : h('button', {
        type: 'button',
        class: 'quiet danger',
        onclick: async () => {
          if (!(await ask(`Remove ${m.abbreviation} from this ${onAndroid() ? 'phone' : 'computer'}? Your notes are not affected.`))) return;
          try {
            await api.removeModule(m.abbreviation);
            await ctx.modulesChanged(null);
            redraw();
          } catch (error) {
            toast(String(error), 5000);
          }
        },
      }, 'Remove')),
    h('p', { class: 'fine' }, facts.join(' · ')),
    h('p', { class: 'fine' }, `Licence: ${m.license || 'not stated'}`),
    m.source ? h('p', { class: 'fine' }, `Source: ${m.source}`) : null,
    h('p', { class: 'fine' }, `${origin} File: ${m.fileName}`),
    m.summary ? h('p', { class: 'fine' }, m.summary) : null);
}

function installedTab(container, ctx) {
  const draw = () => {
    clear(container);
    const install = h('button', {
      type: 'button',
      class: 'primary',
      onclick: async () => {
        const path = await pickFile('Install a module', [{ name: 'GRT Bible module', extensions: ['grtb'] }]);
        if (!path) return;
        install.disabled = true;
        toast('Checking the module…', 8000);
        try {
          const info = await api.installModule(path);
          toast(`${info.abbreviation} installed: ${info.summary}`, 6000);
          await ctx.modulesChanged(info.kind === 'bible' ? info.abbreviation : null);
          draw();
        } catch (error) {
          toast(String(error), 8000);
        } finally {
          install.disabled = false;
        }
      },
    }, 'Install a .grtb file…');
    container.append(
      h('div', { class: 'row' }, install,
        h('button', { type: 'button', onclick: () => importDialog(ctx) }, 'Convert a SWORD or USFM module…')),
      h('p', { class: 'fine' },
        onAndroid() ? 'The file picker shows every file, because .grtb is not a type Android knows. ' : '',
        h('button', { type: 'button', class: 'link', onclick: () => openGuide() }, 'How to find and install modules')));
    for (const [kind, [, plural]] of Object.entries(KIND_LABELS)) {
      const modules = ctx.modules().filter((m) => m.kind === kind);
      if (!modules.length) continue;
      container.append(h('h3', { class: 'kind-head' }, `${plural} (${modules.length})`), ...modules.map((m) => moduleCard(m, ctx, draw)));
    }
  };
  draw();
}

const RENDERERS = { passage: passageTab, dictionary: dictionaryTab, books: booksTab, installed: installedTab };

export async function renderLibrary(container, tab, ctx, options = {}) {
  const renderer = RENDERERS[tab] || passageTab;
  try {
    await renderer(container, ctx, options);
  } catch (error) {
    container.append(empty(String(error)));
  }
}

// Converting a SWORD or USFM module

function field(label, control, note = null) {
  return h('label', { class: 'field' }, h('span', { class: 'field-label' }, label), control, note ? h('span', { class: 'fine' }, note) : null);
}

export function importDialog(ctx) {
  const body = h('div', { class: 'import' });
  const status = h('p', { class: 'fine', 'aria-live': 'polite' });

  const chooseStep = () => {
    const android = onAndroid();
    clear(body,
      h('h2', {}, 'Convert a module'),
      h('p', {}, android
        ? 'Choose a SWORD module as a ZIP, or USFM files as a ZIP or a single file. Bibles, commentaries, dictionaries and books are all read. Only import texts you have the right to use.'
        : 'Choose a SWORD module (a ZIP or its folder) or USFM files (a ZIP, a folder or a single file). Bibles, commentaries, dictionaries and books are all read. Only import texts you have the right to use.'),
      h('div', { class: 'row' },
        h('button', { type: 'button', onclick: async () => inspect(await pickFile('Choose a module file', [{ name: 'SWORD or USFM', extensions: ['zip', 'usfm', 'sfm'] }])) }, 'Choose a file…'),
        android ? null : h('button', { type: 'button', onclick: async () => inspect(await pickFolder('Choose a module folder')) }, 'Choose a folder…')),
      status);
  };

  const inspect = async (path) => {
    if (!path) return;
    status.textContent = 'Reading the file… a large commentary takes a few seconds.';
    try {
      formStep(path, await api.inspectImport(path));
    } catch (error) {
      status.textContent = String(error);
    }
  };

  const formStep = (path, found) => {
    const abbreviation = h('input', { type: 'text', value: found.abbreviation, maxlength: 10, required: true });
    const title = h('input', { type: 'text', value: found.title, required: true });
    const language = h('input', { type: 'text', value: found.language, placeholder: 'it, en, la…', maxlength: 8, required: true });
    const kind = h('select', {}, found.kinds.map((k) => h('option', { value: k }, KIND_LABELS[k]?.[0] || k)));
    kind.value = found.moduleKind;
    const byVerse = found.moduleKind !== 'dictionary' && found.moduleKind !== 'book';
    const best = found.ranking[0]?.[0];
    const scheme = h('select', {}, found.ranking.map(([name, score]) =>
      h('option', { value: name }, `${schemeLabel(name)}${name === best && found.moduleKind === 'bible' ? ' (best fit)' : ''}${score ? `, ${score} mismatches` : ''}`)));
    if (!found.ranking.some(([name]) => name === found.versification)) scheme.append(h('option', { value: found.versification }, schemeLabel(found.versification)));
    scheme.value = found.versification;
    const go = h('button', { type: 'button', class: 'primary' }, 'Import');
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
          kind: kind.value,
        });
        closeOverlay();
        toast(`${info.abbreviation} installed. ${info.summary}`, 6000);
        ctx.modulesChanged(info.kind === 'bible' ? info.abbreviation : null);
      } catch (error) {
        result.textContent = String(error);
        go.disabled = false;
      }
    });
    const what = found.moduleKind === 'bible'
      ? `${found.kind === 'sword' ? 'SWORD module' : 'USFM text'} with ${found.books} books of the canon.`
      : `SWORD ${(KIND_LABELS[found.moduleKind]?.[0] || found.moduleKind).toLowerCase()} with ${found.entries} entries.`;
    clear(body,
      h('h2', {}, 'Convert a module'),
      h('p', {}, what),
      found.warnings.length ? h('details', {}, h('summary', {}, `${found.warnings.length} notes from the converter`), h('ul', {}, found.warnings.map((w) => h('li', {}, w)))) : null,
      found.kinds.length > 1 ? field('Read it as', kind, 'A commentary made mostly of references, such as the Treasury of Scripture Knowledge, works best as cross-references.') : null,
      field('Abbreviation', abbreviation, 'Shown in lists and in copied references.'),
      field('Title', title),
      field('Language code', language),
      byVerse ? field('Verse numbering', scheme, 'Notes, highlights and references land on the right verse only if this is right.') : null,
      h('div', { class: 'row' }, h('button', { type: 'button', onclick: chooseStep }, 'Back'), go),
      result);
  };

  chooseStep();
  openOverlay(body, { label: 'Convert a module' });
}

export { rangeLabels, preview };
