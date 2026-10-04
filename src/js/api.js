/** The only bridge between the interface and the backend. */

function invoke(command, args = {}) {
  return window.__TAURI__.core.invoke(command, args);
}

let platform = '';

/** Remembers the operating system the backend reported. */
export function setPlatform(name) {
  platform = name || '';
}

export const onAndroid = () => platform === 'android';

function toPath(result) {
  if (!result) return null;
  if (typeof result === 'string') return result;
  if (Array.isArray(result)) return toPath(result[0]);
  if (typeof result === 'object' && typeof result.path === 'string') return result.path;
  return null;
}

export const api = {
  startup: () => invoke('startup'),
  chapter: (module, book, chapter) => invoke('chapter', { module, book, chapter }),
  resolve: (module, refs) => invoke('resolve', { module, refs }),
  toCanon: (module, book, chapter, verse) => invoke('to_canon', { module, book, chapter, verse }),
  verseTexts: (module, refs) => invoke('verse_texts', { module, refs }),
  parallel: (modules, book, chapter) => invoke('parallel', { modules, book, chapter }),
  search: (query, modules, books, offset, limit) => invoke('search', { query, modules, books, offset, limit }),
  votd: (day) => invoke('votd', { day }),
  saveUser: (data) => invoke('user_save', { data }),
  exportUser: (path) => invoke('user_export', { path }),
  importUser: (path) => invoke('user_import', { path }),
  saveSettings: (settings) => invoke('settings_save', { settings }),
  videos: () => invoke('videos_list'),
  importVideos: (path) => invoke('videos_import', { path }),
  openVideo: (id) => invoke('video_open', { id }),
  openChannel: (id) => invoke('channel_open', { id }),
  inspectImport: (path) => invoke('import_inspect', { path }),
  importModule: (path, choices) => invoke('import_module', { path, choices }),
  removeModule: (module) => invoke('module_remove', { module }),
  moduleReport: (module) => invoke('module_report', { module }),
};

// Android's picker filters by MIME type, and these file types have none, so it offers every file.
export async function pickFile(title, filters) {
  return toPath(await invoke('plugin:dialog|open', { options: { title, multiple: false, directory: false, filters: onAndroid() ? [] : filters } }));
}

export async function pickFolder(title) {
  return toPath(await invoke('plugin:dialog|open', { options: { title, multiple: false, directory: true } }));
}

export async function pickSave(title, defaultPath, filters) {
  const kinds = onAndroid() ? [{ name: filters[0]?.name || 'File', extensions: ['application/octet-stream'] }] : filters;
  return toPath(await invoke('plugin:dialog|save', { options: { title, defaultPath, filters: kinds } }));
}

export async function ask(message, title = 'GRT Bible') {
  try {
    return await invoke('plugin:dialog|ask', { options: { message, title, kind: 'warning' } });
  } catch {
    return false;
  }
}

export async function tell(message, title = 'GRT Bible') {
  try {
    await invoke('plugin:dialog|message', { options: { message, title } });
  } catch {
    // A dialog that will not open must not stop the program.
  }
}
