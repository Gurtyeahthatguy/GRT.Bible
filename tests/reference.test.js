import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

import { ALIASES, BOOKS } from '../src/js/books.js';
import { compareOsis, formatReference, parseReference } from '../src/js/reference.js';

const here = dirname(fileURLToPath(import.meta.url));

describe('parseReference', () => {
  it('reads a single verse', () => {
    expect(parseReference('Gv 3,16')).toMatchObject({ books: ['John'], chapter: 3, verse: 16, verseEnd: null });
  });

  it('reads a range', () => {
    expect(parseReference('Gv 3,16-18')).toMatchObject({ books: ['John'], chapter: 3, verse: 16, verseEnd: 18 });
  });

  it('reads a whole chapter, in any case', () => {
    expect(parseReference('sir 24')).toMatchObject({ books: ['Sir'], chapter: 24, verse: null });
    expect(parseReference('SIR 24')).toMatchObject({ books: ['Sir'], chapter: 24 });
  });

  it('reads numbered books with or without a space', () => {
    expect(parseReference('1mac 2,15-20')).toMatchObject({ books: ['1Macc'], chapter: 2, verse: 15, verseEnd: 20 });
    expect(parseReference('1 Mac 2,15')).toMatchObject({ books: ['1Macc'], verse: 15 });
    expect(parseReference('III Ioannis 1')).toMatchObject({ books: ['3John'] });
  });

  it('reads Latin abbreviations', () => {
    expect(parseReference('Ps 23')).toMatchObject({ books: ['Ps'], chapter: 23 });
    expect(parseReference('Eccli 24,3')).toMatchObject({ books: ['Sir'], chapter: 24, verse: 3 });
    expect(parseReference('Io 1,1')).toMatchObject({ books: ['John'] });
    expect(parseReference('Ion 2')).toMatchObject({ books: ['Jonah'] });
  });

  it('reads full names with any separator', () => {
    for (const text of ['Giovanni 3 16', 'Giovanni 3:16', 'Giovanni 3.16', 'giovanni 3,16']) {
      expect(parseReference(text)).toMatchObject({ books: ['John'], chapter: 3, verse: 16 });
    }
    expect(parseReference('Giosue 1,9')).toMatchObject({ books: ['Josh'] });
    expect(parseReference('Giosuè 1,9')).toMatchObject({ books: ['Josh'] });
    expect(parseReference('Cantico dei Cantici 8,6')).toMatchObject({ books: ['Song'], chapter: 8, verse: 6 });
  });

  it('reads English references', () => {
    expect(parseReference('John 3:16')).toMatchObject({ books: ['John'], chapter: 3, verse: 16 });
    expect(parseReference('1 Kings 19:12')).toMatchObject({ books: ['1Kgs'] });
  });

  it('reads ranges across chapters and chapter ranges', () => {
    expect(parseReference('Gv 3,16-4,2')).toMatchObject({ chapter: 3, verse: 16, chapterEnd: 4, verseEnd: 2 });
    expect(parseReference('Mt 5-7')).toMatchObject({ chapter: 5, verse: null, chapterEnd: 7 });
  });

  it('asks when an abbreviation means two books', () => {
    const gn = parseReference('Gn 3');
    expect(gn.books.sort()).toEqual(['Gen', 'Jonah']);
    expect(gn.exact).toBe(true);
    expect(parseReference('Gen 3').books).toEqual(['Gen']);
    expect(parseReference('Giona 3').books).toEqual(['Jonah']);
  });

  it('offers books for a partial name', () => {
    const partial = parseReference('Gio 3');
    expect(partial.exact).toBe(false);
    expect(partial.books).toEqual(['Josh', 'Job', 'Joel', 'Jonah', 'John']);
  });

  it('refuses nonsense', () => {
    expect(parseReference('')).toBeNull();
    expect(parseReference('xyz 3')).toBeNull();
    expect(parseReference('Gv 3,18-16')).toBeNull();
    expect(parseReference('Gv 0')).toBeNull();
  });
});

describe('formatReference', () => {
  it('follows the conventions of the language', () => {
    expect(formatReference('it', 'John', 3, 16, 3, 17)).toBe('Gv 3,16-17');
    expect(formatReference('la', 'John', 3, 16, 3, 16)).toBe('Io 3,16');
    expect(formatReference('en', 'John', 3, 16, 3, 17)).toBe('John 3:16-17');
    expect(formatReference('it', 'John', 3, 16, 4, 2)).toBe('Gv 3,16-4,2');
    expect(formatReference('grc', 'Matt', 5, null)).toBe('Mt 5');
  });

  it('prefers the abbreviation a module gives', () => {
    expect(formatReference('en', '1Sam', 3, 10, 3, 10, '1 Sam')).toBe('1 Sam 3:10');
  });
});

describe('the book table', () => {
  it('has the 73 books of the canon in order', () => {
    expect(BOOKS).toHaveLength(73);
    expect(BOOKS.filter((b) => b.testament === 'OT')).toHaveLength(46);
    expect(BOOKS.filter((b) => b.deutero)).toHaveLength(7);
  });

  it('agrees with the names the modules are built with', () => {
    const tsv = readFileSync(join(here, '..', 'crates', 'grtb', 'data', 'books.tsv'), 'utf8').trim().split(/\r?\n/).slice(1);
    expect(tsv).toHaveLength(73);
    tsv.forEach((line, i) => {
      const [osis, , , enName, enAbbr, itName, itAbbr, laName, laAbbr] = line.split('\t');
      const book = BOOKS[i];
      expect(book.osis).toBe(osis);
      expect(book.names.en).toEqual([enName, enAbbr]);
      expect(book.names.it).toEqual([itName, itAbbr]);
      expect(book.names.la).toEqual([laName, laAbbr]);
    });
  });

  it('only leaves deliberate ambiguities', () => {
    const shared = [...ALIASES].filter(([, books]) => books.length > 1).map(([alias]) => alias);
    expect(shared).toEqual(['gn']);
  });

  it('orders internal references by canon', () => {
    expect(compareOsis('Tob.1.1', 'Esth.1.1')).toBeLessThan(0);
    expect(compareOsis('Ps.23.1', 'Ps.9.1')).toBeGreaterThan(0);
  });
});
