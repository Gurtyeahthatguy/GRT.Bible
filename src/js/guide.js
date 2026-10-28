/** The module guide, shipped inside the program and drawn from its Markdown. */

import { api } from './api.js';
import { h } from './dom.js';
import { openOverlay, toast } from './overlay.js';

/** Bold, code and links inside a line; links keep their words, since nothing here goes online. */
function inline(text) {
  const out = [];
  const pattern = /(\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\([^)]+\))/g;
  let last = 0;
  for (const match of text.matchAll(pattern)) {
    if (match.index > last) out.push(text.slice(last, match.index));
    const token = match[0];
    if (token.startsWith('**')) out.push(h('strong', {}, token.slice(2, -2)));
    else if (token.startsWith('`')) out.push(h('code', {}, token.slice(1, -1)));
    else {
      const [, words, target] = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(token);
      out.push(words === target ? h('code', {}, words) : h('span', {}, words, ' ', h('span', { class: 'fine' }, `(${target})`)));
    }
    last = match.index + token.length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

/** Headings, paragraphs, lists, tables and fenced code: the Markdown the guide uses. */
export function renderGuide(markdown) {
  const root = h('div', { class: 'guide' });
  const lines = String(markdown).replace(/\r\n?/g, '\n').split('\n');
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (line.startsWith('```')) {
      const code = [];
      i += 1;
      while (i < lines.length && !lines[i].startsWith('```')) code.push(lines[i++]);
      i += 1;
      root.append(h('pre', {}, h('code', {}, code.join('\n'))));
      continue;
    }
    const heading = /^(#{1,4})\s+(.*)$/.exec(line);
    if (heading) {
      root.append(h(`h${Math.min(heading[1].length + 1, 5)}`, {}, inline(heading[2])));
      i += 1;
      continue;
    }
    if (/^\s*([-*]|\d+\.)\s+/.test(line)) {
      const ordered = /^\s*\d+\./.test(line);
      const list = h(ordered ? 'ol' : 'ul');
      while (i < lines.length && /^\s*([-*]|\d+\.)\s+/.test(lines[i])) {
        let item = lines[i].replace(/^\s*([-*]|\d+\.)\s+/, '');
        i += 1;
        while (i < lines.length && /^\s{2,}\S/.test(lines[i]) && !/^\s*([-*]|\d+\.)\s+/.test(lines[i])) item += ` ${lines[i++].trim()}`;
        list.append(h('li', {}, inline(item)));
      }
      root.append(list);
      continue;
    }
    if (line.startsWith('|')) {
      const table = h('table');
      while (i < lines.length && lines[i].startsWith('|')) {
        const cells = lines[i].split('|').slice(1, -1).map((c) => c.trim());
        if (!cells.every((c) => /^-+$/.test(c))) table.append(h('tr', {}, cells.map((c) => h('td', {}, inline(c)))));
        i += 1;
      }
      root.append(table);
      continue;
    }
    if (/^-{3,}$/.test(line.trim())) {
      root.append(h('hr'));
      i += 1;
      continue;
    }
    if (!line.trim()) {
      i += 1;
      continue;
    }
    const para = [];
    while (i < lines.length && lines[i].trim() && !/^(#{1,4}\s|```|\||\s*([-*]|\d+\.)\s+)/.test(lines[i])) para.push(lines[i++].trim());
    root.append(h('p', {}, inline(para.join(' '))));
  }
  return root;
}

export async function openGuide() {
  try {
    const text = await api.guide();
    openOverlay(renderGuide(text), { label: 'Installing modules', wide: true });
  } catch (error) {
    toast(String(error), 5000);
  }
}
