#!/usr/bin/env bash
# Serves the interface in an ordinary browser with the backend stubbed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${1:-8741}"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

for entry in "$ROOT"/src/*; do
  name="$(basename "$entry")"
  case "$name" in
    *.html) cp "$entry" "$STAGE/$name" ;;
    *) ln -s "$entry" "$STAGE/$name" ;;
  esac
done
cp "$ROOT/scripts/preview-stub.js" "$STAGE/backend-stub.js"

python3 - "$ROOT/modules" "$STAGE/preview-data.js" <<'EXPORT'
import json, sqlite3, sys, pathlib
modules_dir, out = pathlib.Path(sys.argv[1]), sys.argv[2]
wanted = {'Gen': [1, 2, 3], 'Ps': [22, 23, 50, 51], 'Isa': [53], 'Matt': [5], 'John': [1, 3], 'Rom': [8]}
data = []
for path in sorted(modules_dir.glob('*.grtb')):
    db = sqlite3.connect(f'file:{path}?mode=ro', uri=True)
    meta = dict(db.execute('select key, value from meta'))
    books = [dict(index=i, osis=o, name=n, abbrev=a, chapters=c) for i, o, n, a, c in db.execute('select id, osis, name, abbrev, chapters from books order by position')]
    chapters = {}
    for b in books:
        for c in wanted.get(b['osis'], []):
            if c > b['chapters']:
                continue
            verses = [dict(verse=v, text=t, paragraph=bool(p)) for v, t, p in db.execute('select verse, text, paragraph from verses where book_id=? and chapter=? order by verse', (b['index'], c))]
            titles = [dict(chapter=c, verse=v, kind=k, text=t) for v, k, t in db.execute('select verse, kind, text from titles where book_id=? and chapter=?', (b['index'], c))]
            notes = [dict(chapter=c, verse=v, marker=m or '', text=t) for v, m, t in db.execute('select verse, marker, text from notes where book_id=? and chapter=?', (b['index'], c))]
            chapters[f"{b['osis']}.{c}"] = dict(verses=verses, titles=titles, notes=notes)
    data.append(dict(meta=meta, books=books, chapters=chapters))
catalog = json.loads((modules_dir.parent / 'catalog' / 'catalog.json').read_text())
videos = [dict(id=v['id'], title=v['title'], host='www.youtube.com', channel=v.get('channel', ''), category=v.get('category', ''),
               source=v.get('source', ''), language=v.get('language', ''), durationS=v.get('duration_s'), description='',
               thumb=None, refs=v.get('refs', []), tags=v.get('tags', [])) for v in catalog['videos']]
channels = [dict(c, videos=sum(1 for v in videos if v['channel'] == c['id'])) for c in catalog.get('channels', [])]
with open(out, 'w') as f:
    f.write('window.__PREVIEW_DATA__ = ' + json.dumps(data, ensure_ascii=False) + ';\n')
    f.write('window.__PREVIEW_VIDEOS__ = ' + json.dumps(dict(channels=channels, videos=videos), ensure_ascii=False) + ';\n')
EXPORT

python3 - "$STAGE/index.html" <<'PATCH'
import sys, pathlib, re
page = pathlib.Path(sys.argv[1])
text = page.read_text()
page.write_text(re.sub(r'(<script type="module" src="[^"]+"></script>)',
                       r'<script src="preview-data.js"></script>\n    <script src="backend-stub.js"></script>\n    \1', text, count=1))
PATCH

echo "Serving on http://localhost:$PORT with a stubbed backend. Ctrl+C to stop."
python3 -m http.server "$PORT" --directory "$STAGE" >/dev/null
