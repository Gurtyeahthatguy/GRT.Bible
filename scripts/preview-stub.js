// Enough of the backend for the interface to run in a plain browser.
(() => {
  const data = window.__PREVIEW_DATA__ || [];
  const packs = window.__PREVIEW_PACKS__ || [];
  const modules = data.map((m) => ({
    kind: 'bible',
    fileName: `${m.meta.abbreviation}.grtb`,
    origin: null,
    features: [],
    entries: 0,
    abbreviation: m.meta.abbreviation,
    id: m.meta.id,
    title: m.meta.title,
    language: m.meta.language,
    direction: m.meta.direction,
    year: m.meta.year ? Number(m.meta.year) : null,
    versification: m.meta.versification,
    license: m.meta.license,
    source: m.meta.source,
    generatedAt: m.meta.generated_at,
    bundled: true,
    hasNotes: m.meta.source_notes === 'true',
    books: m.books,
    missingBooks: JSON.parse(m.meta.missing_books || '[]'),
    summary: 'Preview: a few chapters only.',
  }));
  for (const p of packs) {
    modules.push({
      kind: p.meta.kind,
      fileName: p.file,
      origin: { file: p.file, installed: '2026-10-27' },
      features: JSON.parse(p.meta.features || '[]'),
      entries: p.entries.length || p.links.length,
      abbreviation: p.meta.abbreviation,
      id: p.meta.id,
      title: p.meta.title,
      language: p.meta.language,
      direction: p.meta.direction,
      year: p.meta.year ? Number(p.meta.year) : null,
      versification: p.meta.versification,
      license: p.meta.license,
      source: p.meta.source,
      generatedAt: p.meta.generated_at,
      bundled: false,
      hasNotes: false,
      books: [],
      missingBooks: [],
      summary: 'Preview: a few chapters only.',
    });
  }
  const packOf = new Map(packs.map((p) => [p.meta.abbreviation, p]));
  const bookIndex = new Map(data.flatMap((m) => m.books.map((b) => [b.osis, b.index])));
  const bookOsis = new Map([...bookIndex].map(([o, i]) => [i, o]));
  const pack = (ref) => {
    const [b, c, v] = ref.split('.');
    return bookIndex.get(b) * 1_000_000 + Number(c) * 1000 + Number(v);
  };
  const unpack = (n) => `${bookOsis.get(Math.floor(n / 1_000_000))}.${Math.floor(n / 1000) % 1000}.${n % 1000}`;
  const range = (a, b) => (a === b ? unpack(a) : `${unpack(a)}-${unpack(b)}`);
  const entryRow = (p, e) => ({ ...e, spans: p.spans.filter((s) => s.entry === e.id).map((s) => range(s.first, s.last)) });
  const byAbbr = new Map(data.map((m) => [m.meta.abbreviation, m]));
  let user = { reading: { current: null, history: [] }, notes: [], highlights: [], bookmarks: [], saved_videos: [], video_lists: [], video_anchors: [] };
  let settings = {};
  const catalog = window.__PREVIEW_VIDEOS__ || { channels: [], videos: [] };
  const videos = catalog.videos;
  const touches = (video, book, chapter) => video.refs.some((r) => {
    const [b, c] = r.start.split('.');
    const [, e] = (r.end || r.start).split('.');
    return b === book && Number(c) <= chapter && Number(e) >= chapter;
  });

  const chapterOf = (abbr, book, chapter) => {
    const m = byAbbr.get(abbr);
    const own = m.books.find((b) => b.osis === book);
    const keys = Object.keys(m.chapters);
    let key = `${book}.${chapter}`;
    if (!m.chapters[key]) key = keys.find((k) => k.startsWith(`${book}.`)) || keys[0];
    const [osis, c] = key.split('.');
    const raw = m.chapters[key];
    const at = keys.indexOf(key);
    const place = (k) => (k ? { book: k.split('.')[0], chapter: Number(k.split('.')[1]), verse: 1 } : null);
    return {
      module: abbr,
      book: osis,
      chapter: Number(c),
      verses: raw.verses.map((v) => ({ ...v, canon: `${osis}.${c}.${v.verse}`, canonEnd: `${osis}.${c}.${v.verse}` })),
      titles: raw.titles,
      notes: raw.notes,
      prev: place(keys[at - 1]),
      next: place(keys[at + 1]),
      canonFirst: `${osis}.${c}.1`,
      canonLast: `${osis}.${c}.${raw.verses.length}`,
      videos: videos.filter((v) => touches(v, osis, Number(c))).map((v) => v.id),
      own,
    };
  };

  const resolve = (abbr, ref) => {
    const [book, chapter, verse] = ref.split('.');
    const m = byAbbr.get(abbr);
    if (!m || !m.books.some((b) => b.osis === book)) return null;
    return { book, chapter: Number(chapter), verse: Number(verse) };
  };

  const handlers = {
    startup: () => ({ version: '0.1.0', modules, settings, user, videos: videos.length, notices: [], schemes: ['hebrew', 'vulgate', 'english', 'lxx'] }),
    chapter: ({ module, book, chapter }) => chapterOf(module, book, chapter),
    resolve: ({ module, refs }) => refs.map((r) => resolve(module, r)),
    to_canon: ({ book, chapter, verse }) => [`${book}.${chapter}.${verse}`, `${book}.${chapter}.${verse}`],
    verse_texts: ({ module, refs }) => refs.map((r) => {
      const p = resolve(module, r);
      if (!p) return null;
      const ch = byAbbr.get(module).chapters[`${p.book}.${p.chapter}`];
      return ch ? ch.verses.find((v) => v.verse === p.verse)?.text ?? null : null;
    }),
    parallel: ({ modules: list, book, chapter }) => {
      const primary = chapterOf(list[0], book, chapter);
      const others = list.slice(1).map((m) => chapterOf(m, primary.book, primary.chapter));
      return {
        modules: list,
        primary,
        rows: primary.verses.map((v) => ({
          canon: v.canon,
          cells: [[{ verse: v.verse, chapter: primary.chapter, text: v.text }],
            ...others.map((o) => o.verses.filter((x) => x.verse === v.verse).map((x) => ({ verse: x.verse, chapter: o.chapter, text: x.text })))],
        })),
      };
    },
    search: ({ query, modules: list }) => {
      const needle = query.toLowerCase().normalize('NFD').replace(/\p{M}/gu, '').replace(/"/g, '');
      const hits = [];
      for (const m of data) {
        if (list.length && !list.includes(m.meta.abbreviation)) continue;
        for (const [key, ch] of Object.entries(m.chapters)) {
          for (const v of ch.verses) {
            if (v.text.toLowerCase().normalize('NFD').replace(/\p{M}/gu, '').includes(needle)) {
              const [book, chapter] = key.split('.');
              hits.push({ module: m.meta.abbreviation, book, chapter: Number(chapter), verse: v.verse, text: v.text });
            }
          }
        }
      }
      return { total: hits.length, hits: hits.slice(0, 50), terms: needle.split(/\s+/).filter(Boolean) };
    },
    votd: () => ['Ps.23.1', 'Ps.23.1'],
    user_save: ({ data: next }) => { user = next; },
    user_export: () => undefined,
    user_import: () => user,
    settings_save: ({ settings: next }) => { settings = next; },
    videos_list: () => catalog,
    channel_open: () => { throw new Error('The preview cannot open a browser.'); },
    videos_import: () => videos.length,
    video_open: () => { throw new Error('The preview cannot open a browser.'); },
    import_inspect: () => ({ kind: 'sword', moduleKind: 'crossrefs', kinds: ['crossrefs', 'commentary'], entries: 24000, title: 'Treasury of Scripture Knowledge', abbreviation: 'TSK2', language: 'en', versification: 'english', ranking: [['hebrew', 0], ['vulgate', 0], ['english', 0], ['lxx', 0]], books: 0, warnings: [] }),
    import_module: () => { throw new Error('The preview cannot convert files.'); },
    module_remove: () => undefined,
    module_report: () => null,
    module_install: () => { throw new Error('The preview cannot install files.'); },
    guide: () => window.__PREVIEW_GUIDE__ || '# No guide',
    save_text: () => undefined,
    print_page: () => { throw new Error('no native printing'); },
    passage: ({ module, first, last }) => {
      const out = [];
      for (const [key, ch] of Object.entries(byAbbr.get(module).chapters)) {
        const [book, c] = key.split('.');
        for (const v of ch.verses) {
          const ref = `${book}.${c}.${v.verse}`;
          if (bookIndex.has(book) && pack(ref) >= pack(first) && pack(ref) <= pack(last)) out.push({ book, chapter: Number(c), verse: v.verse, text: v.text, canon: ref });
        }
      }
      return out;
    },
    crossrefs: ({ at }) => packs.filter((p) => p.meta.kind === 'crossrefs').map((p) => ({
      module: p.meta.abbreviation,
      title: p.meta.title,
      groups: p.links.filter((l) => l.first <= pack(at) && l.last >= pack(at)).map((l) => ({ label: l.label, targets: l.targets.split(' ') })),
    })),
    commentary: ({ module, at }) => {
      const p = packOf.get(module);
      const n = pack(at);
      const chapter = Math.floor(n / 1000) * 1000;
      const book = Math.floor(n / 1_000_000) * 1_000_000;
      const ids = new Set(p.spans.filter((s) => (s.first <= n && s.last >= n && s.first >= book) || s.first === chapter || s.first === book).map((s) => s.entry));
      return p.entries.filter((e) => ids.has(e.id)).map((e) => entryRow(p, e));
    },
    commentary_step: () => null,
    entry: ({ module, id }) => {
      const p = packOf.get(module);
      const e = p.entries.find((x) => x.id === id);
      return e ? entryRow(p, e) : null;
    },
    entry_by_key: ({ module, key }) => {
      const p = packOf.get(module);
      const e = p.entries.find((x) => x.key.toLowerCase() === key.toLowerCase());
      return e ? entryRow(p, e) : null;
    },
    lookup: ({ module, text, limit }) => packOf.get(module).entries
      .filter((e) => e.key.toLowerCase().startsWith(text.toLowerCase())).slice(0, limit)
      .map((e) => ({ id: e.id, key: e.key, title: e.title, children: 0 })),
    sections: ({ module, parent }) => {
      const p = packOf.get(module);
      return p.entries.filter((e) => (e.parent ?? null) === (parent ?? null))
        .map((e) => ({ id: e.id, key: e.key, title: e.title, children: p.entries.filter((c) => c.parent === e.id).length }));
    },
    'plugin:dialog|open': () => null,
    'plugin:dialog|save': () => null,
    'plugin:dialog|ask': ({ options }) => window.confirm(options.message),
    'plugin:dialog|message': ({ options }) => window.alert(options.message),
  };

  window.__TAURI__ = {
    core: {
      invoke: async (command, args) => {
        const handler = handlers[command];
        if (!handler) throw new Error(`preview: no stub for ${command}`);
        return handler(args || {});
      },
    },
  };
})();
