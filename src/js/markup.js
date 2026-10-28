/** Commentary, dictionary and book text, rebuilt element by element from the subset a module stores. */

import { h } from './dom.js';

const BLOCKS = new Set(['p', 'h4']);
const INLINE = new Set(['b', 'i', 'sup']);
const SKIPPED = new Set(['script', 'style', 'template', 'svg', 'math', 'iframe', 'object']);
const RANGE = /^[1-3]?[A-Za-z]+\.\d+\.\d+(-[1-3]?[A-Za-z]+\.\d+\.\d+)?$/;

/** The parts of a stored range: `Gen.1.1-Gen.1.3` into start and end. */
export function splitRange(text) {
  if (!RANGE.test(text || '')) return null;
  const [start, end] = text.split('-');
  return { start, end: end || start };
}

function walk(node, handlers, out) {
  for (const child of node.childNodes) {
    if (child.nodeType === 3) {
      out.append(child.nodeValue);
      continue;
    }
    if (child.nodeType !== 1) continue;
    const tag = child.nodeName.toLowerCase();
    if (SKIPPED.has(tag)) continue;
    if (tag === 'br') {
      out.append(h('br'));
      continue;
    }
    let target = null;
    if (BLOCKS.has(tag) || INLINE.has(tag)) {
      target = h(tag);
    } else if (tag === 'a' && child.hasAttribute('data-ref')) {
      const range = splitRange(child.getAttribute('data-ref'));
      if (range) {
        target = h('button', { class: 'link ref', type: 'button', dataset: { start: range.start, end: range.end } });
        target.addEventListener('click', () => handlers.onRef?.(range, target));
      }
    } else if (tag === 'a' && child.hasAttribute('data-key')) {
      const key = child.getAttribute('data-key');
      target = h('button', { class: 'link key', type: 'button', dataset: { key } });
      target.addEventListener('click', () => handlers.onKey?.(key, target));
    }
    // Anything else keeps its words and loses its element.
    walk(child, handlers, target || out);
    if (target) out.append(target);
  }
}

/** A fragment of safe elements for a module's markup; links call `onRef` and `onKey`. */
export function renderMarkup(markup, handlers = {}) {
  const parsed = new DOMParser().parseFromString(`<body>${markup || ''}</body>`, 'text/html');
  const fragment = document.createDocumentFragment();
  walk(parsed.body, handlers, fragment);
  return fragment;
}
