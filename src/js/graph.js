/** Graphs of a passage's references and of the links between notes, as JSON Canvas files that GRT Graphs opens. */

const WIDTH = 280;
const HEIGHT = 140;

function card(id, text, x, y, color = null) {
  return { id, type: 'text', text, x: Math.round(x), y: Math.round(y), width: WIDTH, height: HEIGHT, ...(color ? { color } : {}) };
}

function short(text, length = 220) {
  const flat = (text || '').replace(/\s+/g, ' ').trim();
  return flat.length > length ? `${flat.slice(0, length - 1)}…` : flat;
}

/** A verse in the middle and the passages it sends to around it, one ring per group of references. */
export function referencesCanvas(center, groups) {
  const nodes = [card('verse', `**${center.label}**\n\n${short(center.text)}`, -WIDTH / 2, -HEIGHT / 2, '1')];
  const edges = [];
  const all = groups.flatMap((g) => g.targets.map((t) => ({ ...t, group: g.label })));
  const radius = Math.max(420, (all.length * (WIDTH + 40)) / (2 * Math.PI));
  all.forEach((t, i) => {
    const angle = (2 * Math.PI * i) / Math.max(all.length, 1) - Math.PI / 2;
    const id = `ref${i + 1}`;
    nodes.push(card(id, `**${t.label}**${t.text ? `\n\n${short(t.text)}` : ''}`, Math.cos(angle) * radius - WIDTH / 2, Math.sin(angle) * radius - HEIGHT / 2));
    edges.push({ id: `e${i + 1}`, fromNode: 'verse', toNode: id, toEnd: 'arrow', ...(t.group ? { label: t.group } : {}) });
  });
  return `${JSON.stringify({ nodes, edges }, null, 2)}\n`;
}

/** Notes as cards in a grid, in the order of the Bible, with an arrow for each link. */
export function notesCanvas(described) {
  const columns = Math.max(1, Math.ceil(Math.sqrt(described.length)));
  const ids = new Set(described.map((d) => d.note.id));
  const nodes = described.map((d, i) => {
    const tags = d.note.tags?.length ? `\n\n${d.note.tags.map((t) => `#${t}`).join(' ')}` : '';
    const quote = d.note.quote ? `\n\n“${short(d.note.quote, 80)}”` : '';
    return card(d.note.id, `**${d.label}**${quote}\n\n${short(d.note.text)}${tags}`, (i % columns) * (WIDTH + 80), Math.floor(i / columns) * (HEIGHT + 80));
  });
  const edges = described.flatMap((d) => (d.note.links || []).filter((id) => ids.has(id))
    .map((id) => ({ id: `${d.note.id}-${id}`, fromNode: d.note.id, toNode: id, toEnd: 'arrow' })));
  return `${JSON.stringify({ nodes, edges }, null, 2)}\n`;
}
