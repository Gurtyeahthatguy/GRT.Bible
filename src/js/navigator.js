/** Books and chapters to pick from; books the translation lacks are shown but cannot be chosen. */

import { BOOKS, nameIn } from './books.js';
import { clear, h } from './dom.js';
import { closeOverlay, openOverlay } from './overlay.js';

export function openNavigator({ module, current, onPick }) {
  const present = new Map(module.books.map((b) => [b.osis, b]));
  const chapters = h('div', { class: 'chapter-grid', role: 'group', 'aria-label': 'Chapters' });
  const heading = h('h2', { class: 'nav-heading' });

  // A phone shows one step at a time: the books, then the chapters of the one chosen.
  const body = h('div', { class: 'navigator', dataset: { step: 'books' } });

  const showChapters = (osis, step = 'chapters') => {
    const book = present.get(osis);
    if (!book) return;
    body.dataset.step = step;
    heading.textContent = book.name;
    clear(chapters);
    for (let c = 1; c <= book.chapters; c++) {
      chapters.append(
        h('button', {
          class: osis === current.book && c === current.chapter ? 'current' : '',
          onclick: () => {
            closeOverlay();
            onPick(osis, c);
          },
        }, String(c)),
      );
    }
    const first = chapters.querySelector('button');
    if (first) first.focus();
  };

  const column = (testament, label) =>
    h('section', { class: 'book-column' },
      h('h2', {}, label),
      h('ul', { class: 'book-list' },
        BOOKS.filter((b) => b.testament === testament).map((b) => {
          const own = present.get(b.osis);
          const [name] = own ? [own.name] : nameIn(b.osis, module.language);
          return h('li', {},
            h('button', {
              class: `${own ? '' : 'missing'} ${b.osis === current.book ? 'current' : ''}`.trim(),
              'aria-disabled': own ? null : 'true',
              title: own ? null : 'Not in this translation',
              onclick: () => {
                if (!own) return;
                if (own.chapters === 1) {
                  closeOverlay();
                  onPick(b.osis, 1);
                } else {
                  showChapters(b.osis);
                }
              },
            }, name));
        })));

  const backToBooks = h('button', { class: 'nav-back', onclick: () => { body.dataset.step = 'books'; } }, '‹ Books');
  const chapterColumn = h('section', { class: 'chapter-column' }, backToBooks, heading, chapters);
  body.append(column('OT', 'Old Testament'), column('NT', 'New Testament'), chapterColumn);
  openOverlay(body, { label: 'Books and chapters', wide: true });
  showChapters(current.book, 'books');
  if (chapterColumn.offsetParent === null) body.querySelector('.book-list button.current')?.scrollIntoView?.({ block: 'center' });
}
