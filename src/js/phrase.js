/** Finding the words a note is about inside a verse, whatever their case and accents. */

/** The text without accents and case, with the place in the original of each of its characters. */
function folded(text) {
  let out = '';
  const at = [];
  for (let i = 0; i < text.length; i++) {
    const plain = text[i].normalize('NFD').replace(/[̀-֑ͯ-ׇ]/g, '').toLowerCase();
    for (const ch of plain) {
      out += /\s/.test(ch) ? ' ' : ch;
      at.push(i);
    }
  }
  return { out, at };
}

/** Where the quote is in the text, as [start, end) of the text, or null. */
export function findPhrase(text, quote) {
  const q = folded((quote || '').replace(/\s+/g, ' ').trim()).out;
  if (!q || !text) return null;
  const t = folded(text);
  const squeezed = t.out.replace(/ +/g, ' ');
  // Runs of spaces in the text count once, as in the quote.
  const map = [];
  let prevSpace = false;
  for (let i = 0; i < t.out.length; i++) {
    const space = t.out[i] === ' ';
    if (space && prevSpace) continue;
    map.push(t.at[i]);
    prevSpace = space;
  }
  const found = squeezed.indexOf(q);
  if (found < 0) return null;
  return [map[found], map[found + q.length - 1] + 1];
}

/** Wraps the quoted words of an element in a mark, across line breaks; returns whether they were found. */
export function markPhrase(element, quote, className = 'note-phrase') {
  const nodes = [];
  const walker = element.ownerDocument.createTreeWalker(element, 1 | 4);
  let text = '';
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    // A line break reads as a space, as it does in a selection.
    if (n.nodeType === 1) {
      if (n.nodeName === 'BR') text += ' ';
      continue;
    }
    nodes.push({ node: n, start: text.length });
    text += n.nodeValue;
  }
  const span = findPhrase(text, quote);
  if (!span) return false;
  const [from, to] = span;
  for (const { node, start } of nodes) {
    const end = start + node.nodeValue.length;
    if (end <= from || start >= to) continue;
    const a = Math.max(from, start) - start;
    const b = Math.min(to, end) - start;
    const range = element.ownerDocument.createRange();
    range.setStart(node, a);
    range.setEnd(node, b);
    const mark = element.ownerDocument.createElement('mark');
    mark.className = className;
    range.surroundContents(mark);
  }
  return true;
}
