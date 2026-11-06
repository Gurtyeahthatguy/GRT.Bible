#!/usr/bin/env python3
"""Builds crates/grtb/data/liturgy.json from the Liturgical Calendar API and Lectio sources.

    python3 -I scripts/build-liturgy.py sources

sources/litcal holds files of Liturgical-Calendar/LiturgicalCalendarAPI (Apache-2.0) at
51c3f0786de1fabf7bea595e701d4965b44998f7, sources/lectio files of nyabongo/lectio (MIT) at
4544e9ca69ce0fbbfd686b75ad6b1e55e3d810c6. Only references are kept, never the text of a reading.
"""

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCES = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ROOT / 'sources')
LITCAL = SOURCES / 'litcal'
LECTIO = SOURCES / 'lectio' / 'calendar' / 'lectionary'
OUT = ROOT / 'crates' / 'grtb' / 'data' / 'liturgy.json'

# The abbreviations of the American lectionary, as Lectio writes them, and their OSIS books.
BOOKS = {
    'Gn': 'Gen', 'Ex': 'Exod', 'Lv': 'Lev', 'Nm': 'Num', 'Dt': 'Deut', 'Jos': 'Josh', 'Jgs': 'Judg', 'Ru': 'Ruth',
    '1 Sm': '1Sam', '2 Sm': '2Sam', '1 Kgs': '1Kgs', '2 Kgs': '2Kgs', '1 Chr': '1Chr', '2 Chr': '2Chr', 'Ezr': 'Ezra',
    'Neh': 'Neh', 'Tb': 'Tob', 'Jdt': 'Jdt', 'Est': 'Esth', '1 Mc': '1Macc', '2 Mc': '2Macc', 'Jb': 'Job', 'Ps': 'Ps',
    'Prv': 'Prov', 'Eccl': 'Eccl', 'Sg': 'Song', 'Wis': 'Wis', 'Sir': 'Sir', 'Is': 'Isa', 'Jer': 'Jer', 'Lam': 'Lam',
    'Bar': 'Bar', 'Ez': 'Ezek', 'Dn': 'Dan', 'Hos': 'Hos', 'Jl': 'Joel', 'Am': 'Amos', 'Ob': 'Obad', 'Jon': 'Jonah',
    'Mi': 'Mic', 'Na': 'Nah', 'Hb': 'Hab', 'Zep': 'Zeph', 'Hg': 'Hag', 'Zec': 'Zech', 'Mal': 'Mal', 'Mt': 'Matt',
    'Mk': 'Mark', 'Lk': 'Luke', 'Jn': 'John', 'Acts': 'Acts', 'Rom': 'Rom', '1 Cor': '1Cor', '2 Cor': '2Cor',
    'Gal': 'Gal', 'Eph': 'Eph', 'Phil': 'Phil', 'Col': 'Col', '1 Thes': '1Thess', '2 Thes': '2Thess', '1 Tm': '1Tim',
    '2 Tm': '2Tim', 'Ti': 'Titus', 'Phlm': 'Phlm', 'Heb': 'Heb', 'Jas': 'Jas', '1 Pt': '1Pet', '2 Pt': '2Pet',
    '1 Jn': '1John', '2 Jn': '2John', '3 Jn': '3John', 'Jude': 'Jude', 'Rv': 'Rev',
}
ONE_CHAPTER = {'Obad', 'Phlm', '2John', '3John', 'Jude'}

# Lectio's celebrations with readings of their own, by the key the Liturgical Calendar API gives them.
SAINTS = {
    'AllSaints': 'all-saints', 'StAndrewAp': 'andrew-apostle', 'Annunciation': 'annunciation-of-the-lord',
    'Assumption': 'assumption-of-the-blessed-virgin-mary', 'StBarnabasAp': 'barnabas-apostle',
    'StBartholomewAp': 'bartholomew-apostle', 'ChairStPeter': 'chair-of-saint-peter-the-apostle',
    'AllSouls': 'commemoration-of-all-the-faithful-departed', 'ConversionStPaul': 'conversion-of-saint-paul-the-apostle',
    'DedicationStsPeterPaul': 'dedication-of-the-basilicas-of-saints-peter-and-paul-apostles',
    'DedicationLateran': 'dedication-of-the-lateran-basilica', 'ExaltationCross': 'exaltation-of-the-holy-cross',
    'StFaustinaKowalska': 'faustina-kowalska-virgin', 'StGregoryNarek': 'gregory-of-narek-abbot',
    'StHildegardBingen': 'hildegard-of-bingen-abbess', 'GuardianAngels': 'holy-guardian-angels',
    'HolyInnocents': 'holy-innocents-martyrs', 'ImmaculateConception': 'immaculate-conception-of-the-blessed-virgin-mary',
    'ImmaculateHeart': 'immaculate-heart-of-mary', 'StJamesAp': 'james-apostle', 'StJohnEvangelist': 'john-apostle',
    'StJohnAvila': 'john-of-avila-priest', 'StJohnPaulII': 'john-paul-ii-pope', 'StJohnXXIII': 'john-xxiii-pope',
    'StJoseph': 'joseph-spouse-of-mary', 'StJosephWorker': 'joseph-the-worker', 'StLawrenceDeacon': 'lawrence-of-rome-deacon',
    'StLukeEvangelist': 'luke-evangelist', 'StMarkEvangelist': 'mark-evangelist',
    'StMartha': 'martha-of-bethany-mary-of-bethany-and-lazarus-of-bethany', 'StMaryMagdalene': 'mary-magdalene',
    'MaryMotherChurch': 'mary-mother-of-the-church', 'StMatthiasAp': 'matthias-apostle',
    'StsArchangels': 'michael-gabriel-and-raphael-archangels', 'NameJesus': 'most-holy-name-of-jesus',
    'NativityJohnBaptist': 'nativity-of-john-the-baptist', 'NativityVirginMary': 'nativity-of-the-blessed-virgin-mary',
    'OurLadyOfLoreto': 'our-lady-of-loreto', 'OurLadyOfSorrows': 'our-lady-of-sorrows',
    'BeheadingJohnBaptist': 'passion-of-saint-john-the-baptist', 'StPaulVI': 'paul-vi-pope',
    'StsPeterPaulAp': 'peter-and-paul-apostles', 'StsPhilipJames': 'philip-and-james-apostles',
    'Presentation': 'presentation-of-the-lord', 'StSimonStJudeAp': 'simon-and-jude-apostles',
    'StStephenProtomartyr': 'stephen-the-first-martyr', 'StMotherTeresa': 'teresa-of-calcutta-virgin',
    'StThomasAp': 'thomas-apostle', 'StsTimothyTitus': 'timothy-of-ephesus-and-titus-of-crete-bishops',
    'Transfiguration': 'transfiguration-of-the-lord', 'Visitation': 'visitation-of-mary',
    'StMatthewEvangelist': 'matthew-apostle',
}

# Celebrations that came after the 1970 missal and before the files above carry their names.
NAMES = {
    'StJohnNewman': ('Saint John Henry Newman, Priest and Doctor of the Church', 'San John Henry Newman, presbitero e dottore della Chiesa'),
}


def load(path):
    return json.loads((LITCAL / path).read_text(encoding='utf-8'))


# Whole names, as the Liturgical Calendar API writes them.
ENGLISH = {
    'Genesis': 'Gen', 'Exodus': 'Exod', 'Proverbs': 'Prov', 'Isaiah': 'Isa', 'Hosea': 'Hos', 'Psalm': 'Ps', 'Psalms': 'Ps',
    'Sirach': 'Sir', 'Wisdom': 'Wis', 'Jeremiah': 'Jer', 'Ezekiel': 'Ezek', 'Daniel': 'Dan', 'Song of Songs': 'Song',
    'Matthew': 'Matt', 'Mark': 'Mark', 'Luke': 'Luke', 'John': 'John', 'Acts': 'Acts', 'Romans': 'Rom',
    '1 Corinthians': '1Cor', '2 Corinthians': '2Cor', 'Galatians': 'Gal', 'Ephesians': 'Eph', 'Philippians': 'Phil',
    'Colossians': 'Col', '1 Thessalonians': '1Thess', '2 Timothy': '2Tim', '1 Timothy': '1Tim', 'Hebrews': 'Heb',
    'James': 'Jas', '1 Peter': '1Pet', '1 John': '1John', 'Revelation': 'Rev', 'Micah': 'Mic', 'Zechariah': 'Zech',
    'Deuteronomy': 'Deut', '1 Kings': '1Kgs', '2 Kings': '2Kgs', 'Lamentations': 'Lam', 'Zephaniah': 'Zeph',
}

VERSES = {}
for line in (ROOT / 'crates' / 'grtb' / 'data' / 'canon.txt').read_text().splitlines():
    if line.strip():
        name, *counts = line.split()
        VERSES[name] = [int(c) for c in counts]


def osis_ranges(ref):
    """`Is 63:16-17, 19; 64:2-7` as internal OSIS ranges, merged where they touch."""
    ref = ref.strip().replace('\u2013', '-').replace(' - ', '-').replace('- ', '-').replace(' -', '-')
    m = re.match(r'^((?:[123] )?[A-Za-z]+(?: of Songs)?)\s+(.*)$', ref)
    if not m or (m.group(1) not in BOOKS and m.group(1) not in ENGLISH):
        raise ValueError(f'unknown book in {ref!r}')
    book, rest = BOOKS.get(m.group(1)) or ENGLISH[m.group(1)], m.group(2)
    if ':' not in rest and book not in ONE_CHAPTER:
        chapter = int(rest)
        return [f'{book}.{chapter}.1-{book}.{chapter}.{VERSES[book][chapter - 1]}']
    points = []
    chapter = 1 if book in ONE_CHAPTER else None
    for segment in rest.split(';'):
        for part in segment.split(','):
            part = re.sub(r'(?<=\d)[a-z]+', '', part.strip())
            if not part:
                continue
            start, _, end = part.partition('-')
            if ':' in start:
                c, v = start.split(':')
                if book == 'Esth' and c.isalpha():
                    chapter, verse = addition_c(int(v))
                else:
                    chapter, verse = int(c), int(v)
            else:
                verse = int(start)
            if chapter is None:
                raise ValueError(f'no chapter in {ref!r}')
            first = (chapter, verse)
            if end:
                if ':' in end:
                    c, v = end.split(':')
                    if book == 'Esth' and c.isalpha():
                        last = addition_c(int(v))
                    else:
                        last = (int(c), int(v))
                    chapter = last[0]
                else:
                    last = (chapter, int(end))
            else:
                last = first
            points.append((first, last))
    merged = []
    for first, last in points:
        if merged and merged[-1][1][0] == first[0] and first[1] <= merged[-1][1][1] + 1 and first >= merged[-1][0]:
            merged[-1] = (merged[-1][0], max(merged[-1][1], last))
        else:
            merged.append((first, last))
    return [f'{book}.{a[0]}.{a[1]}-{book}.{b[0]}.{b[1]}' for a, b in merged]


def addition_c(verse):
    """Addition C of Esther, which the internal numbering keeps at 13:8-14:19."""
    return (13, verse + 7) if verse <= 11 else (14, verse - 11)


def readings():
    out = {}
    for path in sorted(LECTIO.glob('*/*.json')):
        if path.parent.name == 'crosscheck':
            continue
        for entry in json.loads(path.read_text(encoding='utf-8'))['entries']:
            masses = []
            for mass in entry['masses']:
                items = []
                for r in mass['readings']:
                    item = {'slot': r['slot'], 'ref': r['ref'], 'osis': osis_ranges(r['ref'])}
                    if r.get('cycle'):
                        item['cycle'] = r['cycle']
                    if r.get('alternatives'):
                        item['or'] = [{'ref': a['ref'], 'osis': osis_ranges(a['ref'])} for a in r['alternatives']]
                    items.append(item)
                masses.append({'id': mass['id'], 'label': mass.get('label', ''), 'readings': items})
            out[entry['key']] = masses
    return out


# Feasts of Italy and Europe, and memorials with readings of their own in the Italian Lectionary.
PROPER_FEASTS = {
    'StBenedict': ['Prv 2:1-9', 'Ps 34:2-11', 'Mt 19:27-29'],
    'StBridget': ['Gal 2:19-20', 'Ps 34:2-11', 'Jn 15:1-8'],
    'StEdithStein': ['Hos 2:16, 17, 21-22', 'Ps 45:11-12, 14-17', 'Mt 25:1-13'],
    'StFrancisAssisi': ['Gal 6:14-18', 'Ps 16:1-2, 5, 7-8, 11', 'Mt 11:25-30'],
    'QueenshipMary': ['Is 9:1-6', 'Ps 113:1-8', 'Lk 1:26-38'],
    'OurLadyOfTheRosary': ['Acts 1:12-14', 'Lk 1:46-55', 'Lk 1:26-38'],
    'PresentationMary': ['Zec 2:14-17', 'Lk 1:46-55', 'Mt 12:46-50'],
}
SLOT_ORDER = ['first-reading', 'psalm', 'gospel']

# The three Masses of All Souls in the Italian Missal.
ALL_SOULS = [
    ('mass-1', 'First Mass', ['Jb 19:1, 23-27', 'Ps 27:1, 4, 7-9, 13-14', 'Rom 5:5-11', 'Jn 6:37-40']),
    ('mass-2', 'Second Mass', ['Is 25:6-9', 'Ps 25:6-7, 17-18, 20-21', 'Rom 8:14-23', 'Mt 25:31-46']),
    ('mass-3', 'Third Mass', ['Wis 3:1-9', 'Ps 42:2-3, 5; 43:3-5', 'Rv 21:1-7', 'Mt 5:1-12']),
]


def litcal_readings(days):
    """Readings for feasts that Lectio leaves out: the Liturgical Calendar API, then the table above."""
    sanctorum = load('lectionary/sanctorum/en.json')
    out = {}
    for day in days:
        key = day['key']
        if key in SAINTS or (day['grade'] < 4 and key not in PROPER_FEASTS):
            continue
        found = sanctorum.get(key) or {}
        refs = [found.get('first_reading', ''), found.get('responsorial_psalm', ''), found.get('gospel', '')]
        if not all(refs):
            refs = PROPER_FEASTS.get(key)
        if not refs:
            continue
        readings = []
        for slot, text in zip(SLOT_ORDER, refs):
            first, *others = text.split('|')
            item = {'slot': slot, 'ref': first.strip(), 'osis': osis_ranges(first)}
            if others:
                item['or'] = [{'ref': o.strip(), 'osis': osis_ranges(o)} for o in others]
            readings.append(item)
        out[f'proper-{key}'] = [{'id': 'day', 'label': '', 'readings': readings}]
        day['readings'] = f'proper-{key}'
    return out


def colour(colours):
    return colours[0] if colours else 'white'


def sanctorale():
    names_en, names_it = {}, {}
    for missal in ['propriumdesanctis_1970', 'propriumdesanctis_2002', 'propriumdesanctis_2008']:
        names_en.update(load(f'missals/{missal}/i18n/en.json'))
        names_it.update(load(f'missals/{missal}/i18n/it.json'))
    names_it.update(load('missals/propriumdesanctis_IT_1983/i18n/it_IT.json'))
    names_en.update(load('decrees/i18n/en.json'))
    names_it.update(load('decrees/i18n/it.json'))
    days = {}
    for missal in ['propriumdesanctis_1970', 'propriumdesanctis_2002', 'propriumdesanctis_2008', 'propriumdesanctis_IT_1983']:
        for x in load(f'missals/{missal}/{missal}.json'):
            days[x['event_key']] = {'key': x['event_key'], 'month': x['month'], 'day': x['day'], 'grade': x['grade'], 'color': colour(x.get('color'))}
    for decree in load('decrees/decrees.json'):
        event, action = decree['liturgical_event'], decree['metadata']['action']
        key = event['event_key']
        if action == 'createNew' and event.get('type') == 'fixed':
            days[key] = {'key': key, 'month': event['month'], 'day': event['day'], 'grade': event['grade'], 'color': colour(event.get('color'))}
        elif action == 'setProperty' and decree['metadata'].get('property') == 'grade':
            days[key]['grade'] = event['grade']
    # Italy and Europe: the patrons are feasts, Saint Nicholas a memorial.
    for path in ['calendars/nations/IT/IT.json', 'calendars/wider_regions/europe/europe.json']:
        for change in load(path)['litcal']:
            event, meta = change['liturgical_event'], change['metadata']
            if meta.get('until_year') or event['event_key'] not in days:
                continue
            if meta['action'] in ('makePatron', 'setProperty') and 'grade' in event:
                days[event['event_key']]['grade'] = event['grade']
    out = []
    for key, day in sorted(days.items(), key=lambda kv: (kv[1]['month'], kv[1]['day'], -kv[1]['grade'], kv[0])):
        en, it = names_en.get(key), names_it.get(key)
        if key in NAMES:
            en, it = NAMES[key]
        if not en or not it:
            raise SystemExit(f'no name for {key}')
        day.update({'en': en, 'it': it})
        if key in SAINTS:
            day['readings'] = SAINTS[key]
        out.append(day)
    return out


def temporale():
    en = load('missals/propriumdetempore/i18n/en.json')
    it = load('missals/propriumdetempore/i18n/it.json')
    out = {}
    for x in load('missals/propriumdetempore/propriumdetempore.json'):
        key = x['event_key']
        out[key] = {'grade': x['grade'], 'color': colour(x.get('color')), 'en': en.get(key, key), 'it': it.get(key, key)}
    for decree in load('decrees/decrees.json'):
        event = decree['liturgical_event']
        if decree['metadata']['action'] == 'createNew' and event.get('type') == 'mobile':
            key = event['event_key']
            out[key] = {'grade': event['grade'], 'color': colour(event.get('color')),
                        'en': load('decrees/i18n/en.json')[key], 'it': load('decrees/i18n/it.json')[key]}
    return out


def main():
    data = {
        'source': 'Calendar: Liturgical Calendar API (Apache-2.0). Readings: Lectio (MIT), from the Ordo Lectionum Missae of 1981.',
        'temporale': temporale(),
        'sanctorale': sanctorale(),
        'readings': readings(),
    }
    data['readings'].update(litcal_readings(data['sanctorale']))
    slots = ['first-reading', 'psalm', 'second-reading', 'gospel']
    data['readings'][SAINTS['AllSouls']] = [
        {'id': id, 'label': label, 'readings': [{'slot': slot, 'ref': ref, 'osis': osis_ranges(ref)} for slot, ref in zip(slots, refs)]}
        for id, label, refs in ALL_SOULS
    ]
    missing = sorted({d['readings'] for d in data['sanctorale'] if 'readings' in d} - set(data['readings']))
    if missing:
        raise SystemExit(f'readings missing for {missing}')
    OUT.write_text(json.dumps(data, ensure_ascii=False, separators=(',', ':'), sort_keys=True) + '\n', encoding='utf-8')
    print(f'{OUT}: {len(data["sanctorale"])} saints, {len(data["readings"])} sets of readings')


if __name__ == '__main__':
    main()
