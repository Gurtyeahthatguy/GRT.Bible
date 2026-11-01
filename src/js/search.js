/** Search across one translation or all of them: words of the text, Strong's numbers, lemmas and forms. */

import { api } from './api.js';
import { clear, foldWithMap, h, markTerms } from './dom.js';
import { shaper } from './original.js';

const PAGE = 50;

export function renderSearch(container, ctx, initial = '') {
  const settings = ctx.settings();
  const input = h('input', { type: 'search', class: 'search-input', placeholder: 'Words, "a phrase", G25, lemma:…, morph:…', 'aria-label': 'Search text', autocapitalize: 'none', autofocus: true });
  const within = h('input', { type: 'checkbox' });
  const withinLabel = h('label', { class: 'check', hidden: true }, within, h('span', {}, 'Within these results'));
  const saved = h('div', { class: 'saved-searches' });
  const save = h('button', { type: 'button', class: 'quiet', title: 'Keep this search to run it again', hidden: true }, 'Save');
  input.value = initial;
  const scope = h('select', { 'aria-label': 'Where to search' },
    h('option', { value: 'module' }, 'This translation'),
    h('option', { value: 'all' }, 'All translations'));
  scope.value = settings.searchScope;
  const part = h('select', { 'aria-label': 'Which books' },
    h('option', { value: 'all' }, 'Whole Bible'),
    h('option', { value: 'OT' }, 'Old Testament'),
    h('option', { value: 'NT' }, 'New Testament'),
    h('option', { value: 'book' }, 'This book'));
  const status = h('p', { class: 'search-status', 'aria-live': 'polite' });
  const results = h('ol', { class: 'results' });
  const more = h('button', { class: 'more', hidden: true }, 'More results');
  let offset = 0;
  let last = null;
  let effective = null;

  const drawSaved = () => {
    const list = ctx.user().saved_searches;
    clear(saved, list.map((item) => h('span', { class: 'tag' },
      h('button', { type: 'button', class: 'link', onclick: () => { input.value = item.query; within.checked = false; run(); } }, item.label || item.query),
      h('button', {
        type: 'button',
        class: 'link',
        'aria-label': `Forget ${item.query}`,
        title: 'Forget this search',
        onclick: () => {
          ctx.user().saved_searches = list.filter((x) => x.id !== item.id);
          ctx.saveUser();
          drawSaved();
        },
      }, ' ×'))));
  };

  const books = () => {
    const module = ctx.module();
    if (part.value === 'book') return [ctx.place().book];
    if (part.value === 'all') return [];
    return module.books.filter((b) => ctx.testament(b.osis) === part.value).map((b) => b.osis);
  };

  const run = async (append = false) => {
    const query = input.value.trim();
    if (!query) {
      clear(results);
      status.textContent = '';
      more.hidden = true;
      return;
    }
    if (!append) offset = 0;
    const modules = scope.value === 'all' ? [] : [ctx.module().abbreviation];
    const full = !append && within.checked && effective ? `(${effective}) AND (${query})` : append ? effective : query;
    status.textContent = 'Searching…';
    let reply;
    try {
      reply = await api.search(full, modules, books(), offset, PAGE);
    } catch (error) {
      status.textContent = `The search could not run: ${error}`;
      return;
    }
    last = query;
    effective = full;
    withinLabel.hidden = false;
    save.hidden = false;
    if (!append) clear(results);
    const byModule = new Map(ctx.modules().map((m) => [m.abbreviation, m]));
    for (const hit of reply.hits) {
      const module = byModule.get(hit.module);
      const marks = (hit.marks || []).map((m) => foldWithMap(m, module?.language).folded);
      const shape = shaper(module?.language, ctx.settings().original);
      const label = ctx.placeLabel({ book: hit.book, chapter: hit.chapter, verse: hit.verse }, null, module);
      results.append(h('li', {},
        h('button', { class: 'result', onclick: () => ctx.openPlace(hit.module, hit.book, hit.chapter, hit.verse) },
          h('span', { class: 'result-ref' }, label, scope.value === 'all' ? h('span', { class: 'muted' }, ` · ${hit.module}`) : null),
          h('span', { class: 'result-text', lang: module?.language, dir: module?.direction }, markTerms(shape(hit.text.replace(/\n/g, ' ')), [...reply.terms, ...marks], module?.language)))));
    }
    offset += reply.hits.length;
    const count = reply.total === 0 ? 'No verses found.' : `${reply.total} ${reply.total === 1 ? 'verse' : 'verses'}`;
    status.textContent = full !== query ? `${count}, within the earlier results` : count;
    more.hidden = offset >= reply.total;
  };

  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') run();
  });
  scope.addEventListener('change', () => {
    ctx.setSetting('searchScope', scope.value);
    if (last) run();
  });
  part.addEventListener('change', () => last && run());
  more.addEventListener('click', () => run(true));
  save.addEventListener('click', () => {
    const query = effective || input.value.trim();
    if (!query) return;
    const list = ctx.user().saved_searches;
    if (!list.some((x) => x.query === query)) {
      list.push({ id: `s${Date.now().toString(36)}`, query, label: query });
      ctx.saveUser();
    }
    drawSaved();
  });
  const help = h('details', { class: 'search-help' }, h('summary', {}, 'How to search'),
    h('ul', {},
      h('li', {}, h('code', {}, 'luce'), ' words, and the beginnings of words'),
      h('li', {}, h('code', {}, '"in principio"'), ' a phrase'),
      h('li', {}, h('code', {}, 'G25'), ' or ', h('code', {}, 'H7225'), ' a Strong\'s number, in modules that carry them'),
      h('li', {}, h('code', {}, 'lemma:ἀγαπάω'), ' every form of a word'),
      h('li', {}, h('code', {}, 'morph:"aorist imperative"'), ', ', h('code', {}, 'morph:qal,perfect'), ' a form'),
      h('li', {}, h('code', {}, 'lemma:ἀγαπάω+morph:imperative'), ' both on one word'),
      h('li', {}, h('code', {}, 'AND'), ', ', h('code', {}, 'OR'), ', ', h('code', {}, 'NOT'), ', parentheses, and ', h('code', {}, 'G25 NEAR/3 G2316'), ' within three words')));

  container.append(
    h('div', { class: 'search-form' }, input, h('div', { class: 'row' }, scope, part), h('div', { class: 'row' }, withinLabel, save)),
    help, saved, status, results, more);
  drawSaved();
  if (initial) run();
  return input;
}
