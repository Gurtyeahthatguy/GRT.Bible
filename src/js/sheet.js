/** The study sheet: a passage with the reader's notes, the commentaries shown, the variants and the words, on one page. */

import { api } from './api.js';
import { h, withBreaks } from './dom.js';
import { shownCommentaries } from './library.js';
import { renderMarkup } from './markup.js';
import { compareOsis, splitOsis } from './reference.js';

/** Verses beyond which the words of the original are left out, as they would fill pages. */
const WORD_LIMIT = 10;

function paragraphsOf(markup) {
  const holder = document.createElement('div');
  holder.append(renderMarkup(markup));
  const blocks = [...holder.querySelectorAll('p, h4')].map((p) => p.textContent.replace(/\s+/g, ' ').trim()).filter(Boolean);
  return blocks.length ? blocks : [holder.textContent.replace(/\s+/g, ' ').trim()].filter(Boolean);
}

function within(osis, start, end) {
  return compareOsis(osis, start) >= 0 && compareOsis(osis, end) <= 0;
}

/**
 * What a sheet holds, as plain text in sections, so the printed page and the file for Paper say the same.
 * `range` is a run of internal verses, `verses` the verses of the reader's translation in it.
 */
export async function buildSheet(ctx, { start, end, title }) {
  const module = ctx.module();
  const sections = [];
  const verses = await api.passage(module.abbreviation, start, end);
  sections.push({ heading: `${title} · ${module.abbreviation}`, lang: module.language, lines: verses.map((v) => ({ label: String(v.verse), text: v.text })) });

  const notes = ctx.user().notes.filter((n) => within(n.start, start, end)).sort((a, b) => compareOsis(a.start, b.start));
  if (notes.length) {
    sections.push({
      heading: 'My notes',
      lines: notes.flatMap((n) => [
        { label: ctx.osisLabel(n.start, n.end), text: [n.quote ? `“${n.quote}”` : '', n.text.trim()].filter(Boolean).join(' ') },
        ...(n.tags?.length ? [{ text: n.tags.map((t) => `#${t}`).join(' '), fine: true }] : []),
      ]),
    });
  }

  const commentaries = ctx.modules().filter((m) => m.kind === 'commentary');
  const ids = new Set(verses.map((v) => v.canon));
  for (const abbreviation of shownCommentaries(commentaries, ctx.settings().library)) {
    const seen = new Set();
    const lines = [];
    for (const canon of ids) {
      for (const entry of await api.commentary(abbreviation, canon).catch(() => [])) {
        // Introductions to the book or chapter stay in the library.
        if (seen.has(entry.id) || /\.0(-|$)/.test(entry.spans[0] || '')) continue;
        seen.add(entry.id);
        const [first, last] = (entry.spans[0] || canon).split('-');
        paragraphsOf(entry.body).forEach((text, i) => lines.push({ label: i ? '' : ctx.osisLabel(first, last || first), text }));
      }
    }
    if (lines.length) sections.push({ heading: `Commentary · ${abbreviation}`, lines });
  }

  const variants = [];
  for (const canon of ids) {
    for (const set of await api.apparatus(canon).catch(() => [])) {
      for (const item of set.items) paragraphsOf(item.body).forEach((text, i) => variants.push({ label: i ? '' : ctx.osisLabel(canon), text, lang: 'grc' }));
    }
  }
  if (variants.length) sections.push({ heading: 'Variants', lines: variants });

  const at = splitOsis(start);
  const original = ctx.modules().find((m) => m.kind === 'bible' && m.words && m.books.some((b) => b.osis === at?.book));
  if (original && at && verses.length && verses.length <= WORD_LIMIT) {
    const chapters = [...new Set(verses.map((v) => splitOsis(v.canon)?.chapter).filter(Boolean))];
    const rows = [];
    for (const chapter of chapters) {
      const words = await api.chapterWords(original.abbreviation, at.book, chapter).catch(() => []);
      for (const w of words) {
        if (!ids.has(`${at.book}.${chapter}.${w.verse}`) || !(w.lemma || w.strong)) continue;
        rows.push([`${chapter},${w.verse}`, w.text, w.lemma, w.xlit, w.gloss, w.parsing || w.features, w.strong]);
      }
    }
    if (rows.length) {
      sections.push({ heading: `Words · ${original.abbreviation}`, lang: original.language, table: { columns: ['Verse', 'Word', 'Lemma', 'Sound', 'Meaning', 'Form', 'Strong'], rows } });
    }
  } else if (original && verses.length > WORD_LIMIT) {
    sections.push({ heading: 'Words', lines: [{ text: `Select up to ${WORD_LIMIT} verses to add the words of ${original.abbreviation}.`, fine: true }] });
  }
  return { title, module: module.abbreviation, sections };
}

/** The sheet as blocks for the printed pages. */
export function sheetBlocks(sheet) {
  const blocks = [h('h1', {}, sheet.title)];
  for (const section of sheet.sections) {
    blocks.push(h('h2', { class: 'keep' }, section.heading));
    for (const line of section.lines || []) {
      blocks.push(h('p', { class: line.fine ? 'fine' : 'sheet-line', lang: line.lang || section.lang || null },
        line.label ? h('b', {}, `${line.label} `) : null, withBreaks(line.text)));
    }
    if (section.table) {
      blocks.push(h('table', { class: 'sheet-words whole', lang: section.lang || null },
        h('thead', {}, h('tr', {}, section.table.columns.map((c) => h('th', {}, c)))),
        h('tbody', {}, section.table.rows.map((row) => h('tr', {}, row.map((cell) => h('td', {}, cell)))))));
    }
  }
  return blocks;
}

function xml(text) {
  return String(text ?? '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

/** Text with line breaks as ODF paragraph content. */
function odfText(text) {
  return String(text ?? '').split('\n').map(xml).join('<text:line-break/>');
}

/** The sheet as a flat OpenDocument text, which GRT Paper and other word processors open. */
export function sheetFodt(sheet) {
  const body = [`<text:h text:style-name="Heading_20_1" text:outline-level="1">${xml(sheet.title)}</text:h>`];
  for (const section of sheet.sections) {
    body.push(`<text:h text:style-name="Heading_20_2" text:outline-level="2">${xml(section.heading)}</text:h>`);
    for (const line of section.lines || []) {
      const label = line.label ? `<text:span text:style-name="Strong">${xml(line.label)} </text:span>` : '';
      body.push(`<text:p text:style-name="${line.fine ? 'Fine' : 'Text_20_body'}">${label}${odfText(line.text)}</text:p>`);
    }
    if (section.table) {
      const cell = (text) => `<table:table-cell office:value-type="string"><text:p>${xml(text)}</text:p></table:table-cell>`;
      body.push(`<table:table table:name="${xml(section.heading)}"><table:table-column table:number-columns-repeated="${section.table.columns.length}"/>`
        + `<table:table-header-rows><table:table-row>${section.table.columns.map(cell).join('')}</table:table-row></table:table-header-rows>`
        + section.table.rows.map((row) => `<table:table-row>${row.map(cell).join('')}</table:table-row>`).join('')
        + '</table:table>');
    }
  }
  return `<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
<office:styles>
<style:style style:name="Strong" style:family="text"><style:text-properties fo:font-weight="bold"/></style:style>
<style:style style:name="Fine" style:family="paragraph"><style:text-properties fo:font-size="9pt" fo:color="#66646d"/></style:style>
</office:styles>
<office:body><office:text>
${body.join('\n')}
</office:text></office:body></office:document>
`;
}
