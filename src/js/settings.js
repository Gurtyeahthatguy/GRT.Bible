/** Interface preferences and their limits. */

export const THEMES = [
  { id: 'system', label: 'System' },
  { id: 'light', label: 'Light' },
  { id: 'sepia', label: 'Sepia' },
  { id: 'dark', label: 'Dark' },
];

export const HIGHLIGHT_COLORS = ['yellow', 'green', 'blue', 'pink'];

export const DEFAULTS = {
  theme: 'system',
  fontSize: 19,
  lineWidth: 66,
  layout: 'paragraphs',
  verseNumbers: true,
  sourceNotes: false,
  titles: true,
  module: 'MAR',
  copy: { refPosition: 'after', includeModule: true, includeNumbers: false },
  parallel: { enabled: false, modules: [] },
  videos: null,
  panel: null,
  searchScope: 'module',
  library: { tab: 'passage', commentary: null, dictionary: null, book: null },
  print: { paper: 'a4', size: 11, numbers: true, headings: true, editionNotes: false, myNotes: false },
};

export const PAPERS = {
  a4: { label: 'A4', width: 210, height: 297 },
  a5: { label: 'A5', width: 148, height: 210 },
  letter: { label: 'Letter', width: 215.9, height: 279.4 },
};

const clamp = (n, lo, hi) => Math.min(hi, Math.max(lo, n));

export function normalise(raw = {}) {
  const s = structuredClone(DEFAULTS);
  if (!raw || typeof raw !== 'object') return s;
  if (THEMES.some((t) => t.id === raw.theme)) s.theme = raw.theme;
  if (Number.isFinite(raw.fontSize)) s.fontSize = clamp(Math.round(raw.fontSize), 12, 36);
  if (Number.isFinite(raw.lineWidth)) s.lineWidth = clamp(Math.round(raw.lineWidth), 60, 70);
  if (raw.layout === 'paragraphs' || raw.layout === 'verses') s.layout = raw.layout;
  for (const key of ['verseNumbers', 'sourceNotes', 'titles']) {
    if (typeof raw[key] === 'boolean') s[key] = raw[key];
  }
  if (typeof raw.module === 'string' && raw.module) s.module = raw.module;
  if (raw.copy && typeof raw.copy === 'object') {
    if (raw.copy.refPosition === 'before' || raw.copy.refPosition === 'after') s.copy.refPosition = raw.copy.refPosition;
    if (typeof raw.copy.includeModule === 'boolean') s.copy.includeModule = raw.copy.includeModule;
    if (typeof raw.copy.includeNumbers === 'boolean') s.copy.includeNumbers = raw.copy.includeNumbers;
  }
  if (raw.parallel && typeof raw.parallel === 'object') {
    s.parallel.enabled = raw.parallel.enabled === true;
    if (Array.isArray(raw.parallel.modules)) {
      s.parallel.modules = raw.parallel.modules.filter((m) => typeof m === 'string').slice(0, 3);
    }
  }
  if (raw.videos === true || raw.videos === false) s.videos = raw.videos;
  if (typeof raw.panel === 'string' || raw.panel === null) s.panel = raw.panel ?? null;
  if (raw.searchScope === 'module' || raw.searchScope === 'all') s.searchScope = raw.searchScope;
  if (raw.library && typeof raw.library === 'object') {
    if (['passage', 'dictionary', 'books', 'installed'].includes(raw.library.tab)) s.library.tab = raw.library.tab;
    for (const key of ['commentary', 'dictionary', 'book']) {
      if (typeof raw.library[key] === 'string') s.library[key] = raw.library[key];
    }
  }
  if (raw.print && typeof raw.print === 'object') {
    if (PAPERS[raw.print.paper]) s.print.paper = raw.print.paper;
    if (Number.isFinite(raw.print.size)) s.print.size = clamp(Math.round(raw.print.size), 8, 16);
    for (const key of ['numbers', 'headings', 'editionNotes', 'myNotes']) {
      if (typeof raw.print[key] === 'boolean') s.print[key] = raw.print[key];
    }
  }
  return s;
}

export function applyAppearance(settings, root = document.documentElement) {
  if (settings.theme === 'system') root.removeAttribute('data-theme');
  else root.setAttribute('data-theme', settings.theme);
  root.style.setProperty('--reading-size', `${settings.fontSize}px`);
  root.style.setProperty('--reading-width', `${settings.lineWidth}ch`);
  paintSystemBars(root);
}

/** On Android, gives the strips behind the status and navigation bars the colour of the toolbars. */
export function paintSystemBars(root = document.documentElement) {
  const bars = window.grtSystemBars;
  if (!bars) return;
  const style = getComputedStyle(root);
  bars.paint(style.getPropertyValue('--bg-raised').trim(), style.getPropertyValue('color-scheme').trim() === 'dark');
}
