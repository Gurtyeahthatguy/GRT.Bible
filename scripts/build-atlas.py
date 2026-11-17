#!/usr/bin/env python3
"""Writes sources/atlas.json for `grtb-convert atlas` from Theographic and Natural Earth.

    python3 -I scripts/build-atlas.py sources

sources/theographic holds Places.csv and Events.csv of robertrouse/theographic-bible-metadata (CC BY-SA 4.0)
at 014222af2416edffb8ae10be783e68b009692b49; sources/naturalearth holds ne_50m_land, ne_10m_lakes and
ne_10m_rivers_lake_centerlines of nvkelso/natural-earth-vector (public domain) at
ca96624a56bd078437bca8184e78163e5039ad19. The map is an equirectangular projection, scaled for latitude 32,
in hundredths of a degree from 12 W and 50 N; src/js/atlas.js projects points the same way.
"""

import csv
import html
import json
import math
import pathlib
import re
import sys

SOURCES = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else 'sources')
OUT = SOURCES / 'atlas.json'
WEST, EAST, SOUTH, NORTH = -12.0, 65.0, 5.0, 50.0
SCALE = math.cos(math.radians(32))


def project(lon, lat):
    return round((lon - WEST) * SCALE * 100), round((NORTH - lat) * 100)


def simplify(points, tolerance):
    """Douglas-Peucker on projected points."""
    if len(points) < 3:
        return points
    keep = [False] * len(points)
    keep[0] = keep[-1] = True
    stack = [(0, len(points) - 1)]
    while stack:
        a, b = stack.pop()
        (x1, y1), (x2, y2) = points[a], points[b]
        dx, dy = x2 - x1, y2 - y1
        length = math.hypot(dx, dy) or 1.0
        far, at = 0.0, None
        for i in range(a + 1, b):
            x, y = points[i]
            d = abs(dy * x - dx * y + x2 * y1 - y2 * x1) / length
            if d > far:
                far, at = d, i
        if at is not None and far > tolerance:
            keep[at] = True
            stack += [(a, at), (at, b)]
    return [p for p, k in zip(points, keep) if k]


def inside(ring):
    return any(WEST <= lon <= EAST and SOUTH <= lat <= NORTH for lon, lat in ring)


def path(ring, tolerance, closed):
    points = [project(lon, lat) for lon, lat in ring]
    # A closed ring starts where it ends, so each half is simplified on its own.
    half = len(points) // 2
    points = simplify(points[:half + 1], tolerance) + simplify(points[half:], tolerance)[1:] if closed else simplify(points, tolerance)
    if len(points) < (3 if closed else 2):
        return ''
    out = [f'M{points[0][0]} {points[0][1]}'] + [f'L{x} {y}' for x, y in points[1:]]
    return ' '.join(out) + (' Z' if closed else '')


def shapes():
    out = []
    for kind, name, tolerance, closed in [('land', 'ne_50m_land', 4, True), ('lake', 'ne_10m_lakes', 2, True),
                                          ('river', 'ne_10m_rivers_lake_centerlines', 2, False)]:
        data = json.loads((SOURCES / 'naturalearth' / f'{name}.geojson').read_text(encoding='utf-8'))
        for feature in data['features']:
            geometry = feature['geometry']
            if not geometry:
                continue
            if geometry['type'] in ('Polygon', 'LineString'):
                parts = [geometry['coordinates']]
            else:
                parts = geometry['coordinates']
            for part in parts:
                rings = part if geometry['type'].endswith('Polygon') else [part]
                for ring in rings[:1] if closed else rings:
                    if inside(ring):
                        d = path(ring, tolerance, closed)
                        if d:
                            out.append([kind, d])
    return out


def rows(name):
    with open(SOURCES / 'theographic' / f'{name}.csv', encoding='utf-8-sig', newline='') as f:
        return list(csv.DictReader(f))


def year_of(text):
    m = re.match(r'^(-?\d+)', text.strip())
    return int(m.group(1)) if m else None


def years_of(start, duration):
    first = year_of(start)
    if first is None:
        return None
    m = re.match(r'^(\d+)Y', duration or '')
    return [first, first + int(m.group(1)) if m else first]


def places():
    out, names = [], {}
    for p in rows('Places'):
        names[p['placeID']] = p['displayTitle']
        names[p['placeLookup']] = p['displayTitle']
        if not p['latitude'] or not p['longitude'] or not p['verses']:
            continue
        kind = ' · '.join(x for x in [p['featureType'], p['featureSubType']] if x)
        lines = [kind] if kind else []
        if p['precision'] in ('Rough', 'Related-Surrounding', 'Related-Within'):
            lines.append('The place is located only roughly.')
        if p['comment']:
            lines.append(p['comment'])
        others = sorted({n for n in [p['kjvName'], p['esvName']] + p['aliases'].split(',') if n.strip() and n.strip() != p['displayTitle']})
        if others:
            lines.append('Also called ' + ', '.join(others) + '.')
        out.append({
            'key': p['displayTitle'],
            'title': p['displayTitle'],
            'body': ''.join(f'<p>{html.escape(line, quote=False)}</p>' for line in lines),
            'verses': p['verses'].split(','),
            'point': [float(p['latitude']), float(p['longitude'])],
            'aliases': others,
        })
    # Two places of one name are told apart by the first verse that names each.
    seen = {}
    for place in out:
        seen[place['title']] = seen.get(place['title'], 0) + 1
    for place in out:
        if seen[place['title']] > 1:
            book, chapter, verse = place['verses'][0].split('.')
            place['title'] = f"{place['title']}, {book} {chapter}:{verse}"
    return out, names


def events(names):
    out = []
    for e in rows('Events'):
        years = years_of(e['startDate'], e['duration'])
        where = [names[x] for x in e['locations'].split(',') if x in names]
        body = f'<p>Where: {html.escape(", ".join(dict.fromkeys(where)), quote=False)}</p>' if where else ''
        out.append({'key': e['eventID'], 'title': e['title'], 'body': body, 'verses': e['verses'].split(','), 'years': years})
    return out


def main():
    found, names = places()
    data = {'places': found, 'events': events(names), 'shapes': shapes()}
    OUT.write_text(json.dumps(data, ensure_ascii=False, separators=(',', ':')), encoding='utf-8')
    print(f'{OUT}: {len(data["places"])} places, {len(data["events"])} events, {len(data["shapes"])} outlines, {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()
