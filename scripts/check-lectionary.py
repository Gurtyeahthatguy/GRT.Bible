#!/usr/bin/env python3
"""Compares the computed lectionary with the daily pages of the Italian bishops' conference.

    python3 -I scripts/check-lectionary.py <folder of pages> [--verbose]

The folder holds pages saved from chiesacattolica.it/liturgia-del-giorno, named YYYYMMDD.html.
A page holds one or more Masses, each starting at its first reading, and often the next day's as well.
A day passes when every reading of one of its Masses is among the computed ones, by book, chapter and
first verse give or take two, or by the start of a later piece, as in `Ez 33,1.7-9`. A page that
matches only a neighbouring day is counted apart. The pages are never kept in the repository.
"""

import html
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CONVERT = ROOT / 'target' / 'release' / 'grtb-convert'

# CEI 2008 abbreviations to OSIS.
CEI = {
    'Gen': 'Gen', 'Es': 'Exod', 'Lv': 'Lev', 'Nm': 'Num', 'Dt': 'Deut', 'Gs': 'Josh', 'Gdc': 'Judg', 'Rt': 'Ruth',
    '1Sam': '1Sam', '2Sam': '2Sam', '1Re': '1Kgs', '2Re': '2Kgs', '1Cr': '1Chr', '2Cr': '2Chr', 'Esd': 'Ezra',
    'Ne': 'Neh', 'Tb': 'Tob', 'Gdt': 'Jdt', 'Est': 'Esth', '1Mac': '1Macc', '2Mac': '2Macc', 'Gb': 'Job', 'Sal': 'Ps',
    'Pr': 'Prov', 'Qo': 'Eccl', 'Ct': 'Song', 'Sap': 'Wis', 'Sir': 'Sir', 'Is': 'Isa', 'Ger': 'Jer', 'Lam': 'Lam',
    'Bar': 'Bar', 'Ez': 'Ezek', 'Dn': 'Dan', 'Os': 'Hos', 'Gl': 'Joel', 'Am': 'Amos', 'Abd': 'Obad', 'Gio': 'Jonah',
    'Mi': 'Mic', 'Na': 'Nah', 'Ab': 'Hab', 'Sof': 'Zeph', 'Ag': 'Hag', 'Zc': 'Zech', 'Ml': 'Mal', 'Mt': 'Matt',
    'Mc': 'Mark', 'Lc': 'Luke', 'Gv': 'John', 'At': 'Acts', 'Rm': 'Rom', '1Cor': '1Cor', '2Cor': '2Cor', 'Gal': 'Gal',
    'Ef': 'Eph', 'Fil': 'Phil', 'Col': 'Col', '1Ts': '1Thess', '2Ts': '2Thess', '1Tm': '1Tim', '2Tm': '2Tim',
    'Tt': 'Titus', 'Fm': 'Phlm', 'Eb': 'Heb', 'Gc': 'Jas', '1Pt': '1Pet', '2Pt': '2Pet', '1Gv': '1John',
    '2Gv': '2John', '3Gv': '3John', 'Gd': 'Jude', 'Ap': 'Rev',
}
SLOTS = {'Prima Lettura': 'first', 'Seconda Lettura': 'second', 'Salmo Responsoriale': 'psalm', 'Vangelo': 'gospel'}
ONE_CHAPTER = {'Obad', 'Phlm', '2John', '3John', 'Jude'}


def first_point(book, text):
    """Chapter and first verse of a reference such as `6,12-16`, `62 (63)` or `41-42 (42-43)`."""
    text = text.strip()
    m = re.match(r'^(\d+)(?:-\d+)?\s*(?:[A-B]\s*)?(?:\((\d+)(?:-\d+)?\))?\s*(?:,\s*(\d+))?', text)
    if not m:
        return None
    chapter = int(m.group(1))
    if book == 'Ps':
        # The Hebrew number is in brackets; one too far off is a misprint.
        hebrew = int(m.group(2)) if m.group(2) else chapter
        chapter = hebrew if abs(hebrew - chapter) <= 2 else chapter + 1
    if book in ONE_CHAPTER and not m.group(3):
        return (1, chapter)
    return (chapter, int(m.group(3)) if m.group(3) else 0)


def starts(book, text):
    """The first point of a reference and of each piece after a dot: `33,1.7-9` starts at 33:1 and 33:7."""
    first = first_point(book, text)
    if not first:
        return []
    out = [first]
    for piece in text.split('.')[1:]:
        m = re.match(r'\s*(\d+)', piece)
        if m:
            out.append((first[0], int(m.group(1))))
    return out


def parse_page(path):
    page = path.read_text(encoding='utf-8', errors='replace')
    title = re.search(r'<title>(.*?)</title>', page, re.S)
    name = html.unescape(title.group(1)).split(' - ')[1:-2] if title else []
    out = []
    pieces = re.split(r'<h2 class="cci-liturgia-giorno-section-title">', page)[1:]
    for piece in pieces:
        heading = html.unescape(re.sub(r'<[^>]+>', '', piece.split('</h2>', 1)[0])).strip()
        slot = SLOTS.get(heading)
        if not slot:
            continue
        if slot == 'psalm':
            m = re.search(r'section-versetto">\s*(?:Dal |Da )?(?:Sal|Salmo)\s+([^<]+)<', piece)
            if m:
                out.append((slot, 'Ps', starts('Ps', m.group(1)), 'Sal ' + m.group(1).strip()))
            else:
                m = re.search(r'section-versetto">\s*(?:Dal |Da )?(?:Cantico\s+)?(\S+)\s+([^<]+)<', piece)
                if m and m.group(1) in CEI:
                    book = CEI[m.group(1)]
                    out.append((slot, book, starts(book, m.group(2)), f'{m.group(1)} {m.group(2).strip()}'))
            continue
        m = re.search(r'title="Leggi (\S+) ([^"]+?) su BibbiaEdu"', piece)
        if m and m.group(1) in CEI:
            book = CEI[m.group(1)]
            out.append((slot, book, starts(book, m.group(2)), f'{m.group(1)} {m.group(2)}'))
    return ' - '.join(name).strip(), out


def computed(start, end):
    lines = subprocess.run([str(CONVERT), 'liturgy', start, '--to', end], check=True, capture_output=True, text=True).stdout
    return {d['date']: d for d in map(json.loads, lines.splitlines())}


def points_of(day):
    found = set()
    for mass in day['masses'] + day['optionalMasses']:
        for r in mass['readings']:
            for ranges in [r['osis']] + [a['osis'] for a in r['alternatives']]:
                for item in ranges:
                    book, chapter, verse = item.split('-')[0].split('.')
                    found.add((book, int(chapter), int(verse)))
    return found


def matches(point, found):
    book, (chapter, verse) = point
    # The Nova Vulgata keeps the prayer of Esther at 4:17a-z, the internal numbering at 13:8-14:19.
    if book == 'Esth' and (chapter, verse) == (4, 17):
        return any(b == book and c in (4, 13, 14) for b, c, _ in found)
    # The Italian Lectionary numbers this passage of Sirach after the Vulgate.
    if book == 'Sir' and (chapter, verse) == (27, 33):
        verse = 30
    return any(b == book and c == chapter and (verse == 0 or abs(v - verse) <= 2) for b, c, v in found)


def groups(readings):
    """The readings of a page cut into Masses, each starting at a first reading."""
    out = []
    for item in readings:
        if item[0] == 'first' or not out:
            out.append([])
        out[-1].append(item)
    return out


def agrees(masses, day):
    found = points_of(day)
    return any(all(not points or any(matches((book, p), found) for p in points) for _, book, points, _ in mass) for mass in masses)


def main():
    folder = pathlib.Path(sys.argv[1])
    verbose = '--verbose' in sys.argv
    pages = sorted(p for p in folder.glob('*.html') if re.fullmatch(r'\d{8}', p.stem))
    if not pages:
        raise SystemExit('no pages')
    iso = lambda p: f'{p.stem[:4]}-{p.stem[4:6]}-{p.stem[6:]}'
    import datetime
    first = datetime.date.fromisoformat(iso(pages[0])) - datetime.timedelta(days=1)
    last = datetime.date.fromisoformat(iso(pages[-1])) + datetime.timedelta(days=1)
    days = computed(first.isoformat(), last.isoformat())
    good = bad = empty = shifted = 0
    for page in pages:
        name, readings = parse_page(page)
        date = datetime.date.fromisoformat(iso(page))
        day = days[date.isoformat()]
        if not readings:
            empty += 1
            if verbose:
                print(f'{date}  no readings on the page')
            continue
        masses = groups(readings)
        if agrees(masses, day):
            good += 1
            if verbose:
                print(f'{date}  ok  {name} = {day["celebration"]["it"]}')
            continue
        near = [d for d in (-1, 1) if agrees(masses, days[(date + datetime.timedelta(days=d)).isoformat()])]
        # A page titled for another date, or carrying only the Gospel of a neighbouring page, is a slip of the site.
        titled = re.search(r'del (\d+) (?:dicembre|gennaio)', name, re.I)
        gospels = {text for mass in masses for slot, *_, text in mass if slot == 'gospel'}
        neighbours = set()
        for d in (-2, -1, 1, 2):
            other = folder / f'{(date + datetime.timedelta(days=d)).strftime("%Y%m%d")}.html'
            if other.exists():
                other_masses = groups(parse_page(other)[1])
                neighbours |= {text for slot, *_, text in (other_masses[0] if other_masses else []) if slot == 'gospel'}
        if (titled and int(titled.group(1)) != date.day) or (gospels and gospels <= neighbours):
            shifted += 1
            print(f'{date}  the page carries the readings of another day  ({name})')
            continue
        if near:
            shifted += 1
            print(f'{date}  the page shows the readings of the day {"before" if near[0] < 0 else "after"}  ({name})')
            continue
        bad += 1
        ours = '; '.join(r['ref'] for m in day['masses'][:1] for r in m['readings'])
        theirs = ' | '.join(', '.join(text for *_, text in mass) for mass in masses)
        print(f'{date}  {name}\n    computed: {day["celebration"]["it"]}  [{ours}]\n    page:     {theirs}')
    print(f'\n{good} days agree, {bad} differ, {shifted} pages show another day, {empty} pages without readings, of {len(pages)}')
    sys.exit(1 if bad else 0)


if __name__ == '__main__':
    main()
