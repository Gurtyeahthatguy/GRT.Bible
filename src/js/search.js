/** Full-text search across one translation or all of them. */

import { api } from './api.js';
import { clear, h, markTerms } from './dom.js';

const PAGE = 50;

export function renderSearch(container, ctx, initial = '') {
  const settings = ctx.settings();
  const input = h('input', { type: 'search', class: 'search-input', placeholder: 'Words, or "an exact phrase"', 'aria-label': 'Search text', autocapitalize: 'none', autofocus: true });
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
    status.textContent = 'Searching…';
    let reply;
    try {
      reply = await api.search(query, modules, books(), offset, PAGE);
    } catch (error) {
      status.textContent = `The search could not run: ${error}`;
      return;
    }
    last = query;
    if (!append) clear(results);
    const byModule = new Map(ctx.modules().map((m) => [m.abbreviation, m]));
    for (const hit of reply.hits) {
      const module = byModule.get(hit.module);
      const label = ctx.placeLabel({ book: hit.book, chapter: hit.chapter, verse: hit.verse }, null, module);
      results.append(h('li', {},
        h('button', { class: 'result', onclick: () => ctx.openPlace(hit.module, hit.book, hit.chapter, hit.verse) },
          h('span', { class: 'result-ref' }, label, scope.value === 'all' ? h('span', { class: 'muted' }, ` · ${hit.module}`) : null),
          h('span', { class: 'result-text', lang: module?.language, dir: module?.direction }, markTerms(hit.text.replace(/\n/g, ' '), reply.terms, module?.language)))));
    }
    offset += reply.hits.length;
    status.textContent = reply.total === 0 ? 'No verses found.' : `${reply.total} ${reply.total === 1 ? 'verse' : 'verses'}`;
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

  container.append(
    h('div', { class: 'search-form' }, input, h('div', { class: 'row' }, scope, part)),
    status, results, more);
  if (initial) run();
  return input;
}
