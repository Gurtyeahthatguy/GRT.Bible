/** Small helpers for building the page. */

/** Creates an element: `h('button', { class: 'x', onclick }, 'Label')`. */
export function h(tag, attrs = {}, ...children) {
  const el = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs || {})) {
    if (value === null || value === undefined || value === false) continue;
    if (key.startsWith('on') && typeof value === 'function') {
      el.addEventListener(key.slice(2), value);
    } else if (key === 'class') {
      el.className = value;
    } else if (key === 'dataset') {
      Object.assign(el.dataset, value);
    } else if (value === true) {
      el.setAttribute(key, '');
    } else {
      el.setAttribute(key, String(value));
    }
  }
  append(el, children);
  return el;
}

function append(el, children) {
  for (const child of children.flat(Infinity)) {
    if (child === null || child === undefined || child === false) continue;
    el.append(child instanceof Node ? child : document.createTextNode(String(child)));
  }
}

export function clear(el, ...children) {
  el.replaceChildren();
  append(el, children);
  return el;
}

export const $ = (selector, root = document) => root.querySelector(selector);
export const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];

/** Text with its line breaks kept as `<br>`. */
export function withBreaks(text) {
  const parts = String(text).split('\n');
  return parts.flatMap((part, i) => (i ? [document.createElement('br'), part] : [part]));
}

/** Folds text the way search does, remembering where each folded character came from. */
export function foldWithMap(text, language) {
  let folded = '';
  const map = [];
  const latin = language === 'la';
  for (let i = 0; i < text.length; i++) {
    const decomposed = text[i].normalize('NFD').replace(/\p{M}/gu, '');
    for (const ch of decomposed) {
      let c = ch.toLowerCase();
      if (c === 'ς') c = 'σ';
      if (latin && c === 'j') c = 'i';
      if (latin && c === 'v') c = 'u';
      if (c === 'æ') c = 'ae';
      if (c === 'œ') c = 'oe';
      for (const piece of c) {
        folded += piece;
        map.push(i);
      }
    }
  }
  return { folded, map };
}

/** The text split into plain and matching pieces, for highlighting search terms. */
export function markTerms(text, terms, language) {
  if (!terms.length) return [text];
  const { folded, map } = foldWithMap(text, language);
  const ranges = [];
  for (const term of terms) {
    if (!term) continue;
    let at = folded.indexOf(term);
    while (at !== -1) {
      const start = map[at];
      const end = (map[at + term.length - 1] ?? text.length - 1) + 1;
      ranges.push([start, end]);
      at = folded.indexOf(term, at + term.length);
    }
  }
  ranges.sort((a, b) => a[0] - b[0]);
  const out = [];
  let cursor = 0;
  for (const [start, end] of ranges) {
    if (start < cursor) continue;
    if (start > cursor) out.push(text.slice(cursor, start));
    out.push(h('mark', {}, text.slice(start, end)));
    cursor = end;
  }
  if (cursor < text.length) out.push(text.slice(cursor));
  return out;
}

export function debounce(fn, wait) {
  let timer = null;
  const wrapped = (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      fn(...args);
    }, wait);
  };
  wrapped.flush = (...args) => {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
      fn(...args);
    }
  };
  return wrapped;
}
