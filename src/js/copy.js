/** Text for the clipboard: the verses and their reference, in the chosen arrangement. */

import { formatReference } from './reference.js';

/** The clipboard text; `excerpt`, when given, replaces the verse text. */
export function copyText({ verses, excerpt = null, book, language, abbrev, module, options }) {
  if (!verses.length) return '';
  const first = verses[0];
  const last = verses[verses.length - 1];
  const reference = formatReference(language, book, first.chapter, first.verse, last.chapter, last.verse, abbrev);
  const label = options.includeModule ? `${reference} (${module})` : reference;
  let body;
  if (excerpt !== null) {
    body = excerpt.replace(/\s+\n/g, '\n').trim();
  } else {
    body = verses
      .map((v) => {
        const text = v.text.replace(/\n/g, ' ');
        return options.includeNumbers ? `${v.verse} ${text}` : text;
      })
      .join(' ');
  }
  return options.refPosition === 'before' ? `${label}\n${body}` : `${body}\n${label}`;
}

function escapeHtml(text) {
  return String(text).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
}

/** The same quotation as HTML: a block quote and its reference, which Paper keeps as a quote. */
export function copyHtml({ verses, excerpt = null, book, language, abbrev, module, options }) {
  if (!verses.length) return '';
  const first = verses[0];
  const last = verses[verses.length - 1];
  const reference = formatReference(language, book, first.chapter, first.verse, last.chapter, last.verse, abbrev);
  const label = escapeHtml(options.includeModule ? `${reference} (${module})` : reference);
  let body;
  if (excerpt !== null) {
    body = escapeHtml(excerpt.replace(/\s+/g, ' ').trim());
  } else {
    body = verses
      .map((v) => {
        const text = escapeHtml(v.text.replace(/\n/g, ' '));
        return options.includeNumbers ? `<sup>${v.verse}</sup>${text}` : text;
      })
      .join(' ');
  }
  const quote = `<blockquote>${body}</blockquote>`;
  const cite = `<p><cite>${label}</cite></p>`;
  return options.refPosition === 'before' ? `${cite}${quote}` : `${quote}${cite}`;
}
