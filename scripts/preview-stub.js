// Enough of the backend for the interface to run in a plain browser.
(() => {
  const data = window.__PREVIEW_DATA__ || [];
  const modules = data.map((m) => ({
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
  const byAbbr = new Map(data.map((m) => [m.meta.abbreviation, m]));
  let user = { reading: { current: null, history: [] }, notes: [], highlights: [], bookmarks: [], saved_videos: [] };
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
    import_inspect: () => ({ kind: 'usfm', title: 'Sample', abbreviation: 'SMP', language: 'en', versification: 'english', ranking: [['english', 0], ['hebrew', 12]], books: 66, warnings: [] }),
    import_module: () => { throw new Error('The preview cannot convert files.'); },
    module_remove: () => undefined,
    module_report: () => null,
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
