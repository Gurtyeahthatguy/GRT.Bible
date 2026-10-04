/** The reference field: type "Gv 3,16", "sir 24" or "Ps 23" and go there. */

import { nameIn } from './books.js';
import { clear, h } from './dom.js';
import { closeOverlay, openOverlay } from './overlay.js';
import { formatReference, parseReference } from './reference.js';

/** Choices for what was typed, in the numbering and names of the current translation. */
export function suggestions(input, module) {
  const parsed = parseReference(input);
  const items = [];
  if (parsed) {
    for (const osis of parsed.books) {
      const own = module.books.find((b) => b.osis === osis);
      const abbrev = own ? own.abbrev : null;
      const label = formatReference(module.language, osis, parsed.chapter, parsed.verse, parsed.chapterEnd ?? parsed.chapter, parsed.verseEnd ?? parsed.verse, abbrev);
      const name = own ? own.name : nameIn(osis, module.language)[0];
      const tooFar = own && parsed.chapter > own.chapters;
      items.push({
        kind: 'go',
        osis,
        label,
        detail: !own ? 'Not in this translation' : tooFar ? `${name} has ${own.chapters} chapters` : name,
        disabled: !own || tooFar,
        reference: parsed,
      });
    }
  }
  const text = String(input || '').trim();
  if (text) items.push({ kind: 'search', label: `Search for “${text}”`, detail: 'Full text', disabled: false, text });
  return { items, ambiguous: Boolean(parsed && parsed.exact && parsed.books.length > 1) };
}

export function openPalette({ module, onGo, onSearch }) {
  let selected = 0;
  let items = [];
  let chosen = false;
  const input = h('input', { type: 'text', class: 'palette-input', placeholder: 'Gv 3,16  ·  sir 24  ·  John 3:16', 'aria-label': 'Reference', autocomplete: 'off', autocapitalize: 'none', spellcheck: 'false', autofocus: true });
  const hint = h('p', { class: 'palette-hint', 'aria-live': 'polite' });
  const list = h('ul', { class: 'palette-list', role: 'listbox', 'aria-label': 'Matches' });

  const activate = (item) => {
    if (!item || item.disabled) return;
    closeOverlay();
    if (item.kind === 'go') onGo(item.osis, item.reference);
    else onSearch(item.text);
  };

  const render = () => {
    const result = suggestions(input.value, module);
    items = result.items;
    if (selected >= items.length) selected = 0;
    const firstEnabled = items.findIndex((i) => !i.disabled);
    if (items[selected] && items[selected].disabled && firstEnabled >= 0) selected = firstEnabled;
    hint.textContent = result.ambiguous ? 'More than one book matches. Choose one.' : '';
    clear(list, items.map((item, i) => h('li', {
      role: 'option',
      class: `${i === selected ? 'selected' : ''} ${item.disabled ? 'disabled' : ''}`.trim(),
      'aria-selected': i === selected ? 'true' : 'false',
      'aria-disabled': item.disabled ? 'true' : null,
      onmousedown: (e) => {
        e.preventDefault();
        activate(item);
      },
    }, h('span', { class: 'label' }, item.label), h('span', { class: 'detail' }, item.detail))));
  };

  input.addEventListener('input', () => {
    selected = 0;
    chosen = false;
    render();
  });
  input.addEventListener('keydown', (e) => {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (!items.length) return;
      const step = e.key === 'ArrowDown' ? 1 : -1;
      selected = (selected + step + items.length) % items.length;
      chosen = true;
      render();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (suggestions(input.value, module).ambiguous && !chosen) {
        const touch = window.matchMedia?.('(pointer: coarse)').matches;
        hint.textContent = touch ? 'More than one book matches. Tap the one you mean.' : 'More than one book matches. Choose one with the arrow keys, then press Enter.';
        return;
      }
      activate(items[selected]);
    }
  });
  openOverlay(h('div', { class: 'palette' }, input, hint, list), { label: 'Go to a passage' });
  render();
  return input;
}
