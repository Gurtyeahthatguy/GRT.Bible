/** Word by word differences between two renderings of the same verses. */

import { h } from './dom.js';

const WORD = /[\p{L}\p{N}][\p{L}\p{N}\p{M}'’]*/gu;

/** A word as compared: no case, accents or apostrophes. */
function folded(word) {
  return word.normalize('NFD').replace(/\p{M}/gu, '').replace(/['’]/g, '').toLowerCase().replace(/ς/g, 'σ');
}

export function wordsOf(text) {
  return [...(text || '').matchAll(WORD)].map((m) => folded(m[0]));
}

/** Which words of each text are not in the longest run the two share, by position. */
export function changedWords(a, b) {
  const x = wordsOf(a);
  const y = wordsOf(b);
  const rows = x.length + 1;
  const cols = y.length + 1;
  const table = new Uint16Array(rows * cols);
  for (let i = x.length - 1; i >= 0; i--) {
    for (let j = y.length - 1; j >= 0; j--) {
      table[i * cols + j] = x[i] === y[j] ? table[(i + 1) * cols + j + 1] + 1 : Math.max(table[(i + 1) * cols + j], table[i * cols + j + 1]);
    }
  }
  const left = new Set();
  const right = new Set();
  let i = 0;
  let j = 0;
  while (i < x.length && j < y.length) {
    if (x[i] === y[j]) {
      i++;
      j++;
    } else if (table[(i + 1) * cols + j] >= table[i * cols + j + 1]) {
      left.add(i++);
    } else {
      right.add(j++);
    }
  }
  while (i < x.length) left.add(i++);
  while (j < y.length) right.add(j++);
  return { left, right };
}

/** The text as nodes, its changed words marked; `start` is the index of its first word in the row. */
export function markChanged(text, changed, start = 0) {
  const nodes = [];
  let index = start;
  let last = 0;
  const plain = (piece) => {
    const lines = piece.split('\n');
    lines.forEach((line, k) => {
      if (k) nodes.push(h('br'));
      if (line) nodes.push(line);
    });
  };
  for (const m of (text || '').matchAll(WORD)) {
    plain(text.slice(last, m.index));
    nodes.push(changed.has(index) ? h('mark', { class: 'diff' }, m[0]) : m[0]);
    index++;
    last = m.index + m[0].length;
  }
  plain(text.slice(last));
  return { nodes, next: index };
}
