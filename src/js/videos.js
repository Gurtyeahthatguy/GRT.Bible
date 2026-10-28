/** The video section: links chosen by the author, opened in the system browser. */

import { api, onAndroid, pickFile } from './api.js';
import { clear, h } from './dom.js';
import { toast } from './overlay.js';
import { compareOsis } from './reference.js';

const PAGE = 40;
export const CATEGORIES = ['Theology', 'Doctrine', 'Debates'];

let cache = null;

export function forgetVideos() {
  cache = null;
}

async function catalog() {
  if (!cache) {
    const reply = await api.videos();
    cache = Array.isArray(reply) ? { channels: [], videos: reply } : reply;
  }
  return cache;
}

function newId(prefix) {
  return `${prefix}${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
}

const overlaps = (start, end, a, b) => compareOsis(start, b) <= 0 && compareOsis(end, a) >= 0;

/** For each verse of a chapter, the videos the reader linked starting there or earlier. */
export function anchorsByVerse(anchors, verses) {
  const out = new Map();
  for (const anchor of anchors || []) {
    const verse = verses.find((v) => overlaps(v.canon, v.canonEnd || v.canon, anchor.start, anchor.end));
    if (!verse) continue;
    const list = out.get(verse.verse) || [];
    if (!list.includes(anchor.video)) list.push(anchor.video);
    out.set(verse.verse, list);
  }
  return out;
}

/** Whether a video speaks about a range, by the catalogue or by the reader's own links. */
export function videoTouches(video, anchors, start, end) {
  const own = anchors.some((a) => a.video === video.id && overlaps(a.start, a.end, start, end));
  return own || (video.refs || []).some((r) => overlaps(r.start, r.end || r.start, start, end));
}

function duration(seconds) {
  if (!seconds) return '';
  const m = Math.floor(seconds / 60);
  const s = String(seconds % 60).padStart(2, '0');
  return m >= 60 ? `${Math.floor(m / 60)}:${String(m % 60).padStart(2, '0')}:${s}` : `${m}:${s}`;
}

const opened = (what) => (onAndroid() ? `Opened: ${what}` : `Opened in your browser: ${what}`);

async function openWith(button, action, done) {
  button.disabled = true;
  try {
    await action();
    toast(done);
  } catch (error) {
    toast(String(error), 5000);
  } finally {
    button.disabled = false;
  }
}

function card(video, channel, ctx, redraw) {
  const user = ctx.user();
  const saved = user.saved_videos.includes(video.id);
  const selection = ctx.selectionRange?.();
  const save = h('button', {
    class: 'quiet',
    'aria-pressed': saved ? 'true' : 'false',
    onclick: () => {
      const list = user.saved_videos;
      const at = list.indexOf(video.id);
      if (at >= 0) list.splice(at, 1);
      else list.push(video.id);
      ctx.saveUser();
      save.setAttribute('aria-pressed', at >= 0 ? 'false' : 'true');
      save.textContent = at >= 0 ? 'Save' : 'Saved';
    },
  }, saved ? 'Saved' : 'Save');
  const open = h('button', { onclick: () => openWith(open, () => api.openVideo(video.id), opened(video.host)) }, onAndroid() ? 'Watch' : 'Open in browser');
  const anchors = user.video_anchors.filter((a) => a.video === video.id);
  const linked = selection && anchors.some((a) => a.start === selection.start && a.end === selection.end);
  const link = selection && !linked ? h('button', {
    class: 'quiet',
    title: 'Show this video beside the verses you selected',
    onclick: () => {
      user.video_anchors.push({ id: newId('a'), video: video.id, start: selection.start, end: selection.end });
      ctx.saveUser();
      ctx.refresh();
      toast(`Linked to ${selection.label}`);
    },
  }, `Link to ${selection.label}`) : null;
  const lists = user.video_lists;
  const listPicker = lists.length ? h('select', { 'aria-label': 'Add to a list' },
    h('option', { value: '' }, 'Add to a list…'),
    lists.map((l) => h('option', { value: l.id }, `${l.videos.includes(video.id) ? '✓ ' : ''}${l.name}`))) : null;
  listPicker?.addEventListener('change', () => {
    const list = lists.find((l) => l.id === listPicker.value);
    if (!list) return;
    const at = list.videos.indexOf(video.id);
    if (at >= 0) list.videos.splice(at, 1);
    else list.videos.push(video.id);
    ctx.saveUser();
    toast(at >= 0 ? `Removed from ${list.name}` : `Added to ${list.name}`);
    redraw();
  });
  return h('article', { class: 'card video', dataset: { video: video.id } },
    video.thumb ? h('img', { src: video.thumb, alt: '', class: 'thumb', loading: 'lazy' }) : null,
    h('div', { class: 'video-body' },
      h('h3', {}, video.title),
      h('p', { class: 'muted' }, [channel?.name || video.source, video.category, duration(video.durationS)].filter(Boolean).join(' · ')),
      video.description ? h('p', {}, video.description) : null,
      anchors.length ? h('p', { class: 'tags' }, anchors.map((a) => h('span', { class: 'tag' },
        h('button', { class: 'link', onclick: () => ctx.openRange(a.start, a.end) }, ctx.osisLabel(a.start, a.end)),
        h('button', {
          class: 'link',
          'aria-label': 'Remove this link',
          title: 'Remove this link',
          onclick: () => {
            user.video_anchors = user.video_anchors.filter((x) => x.id !== a.id);
            ctx.saveUser();
            ctx.refresh();
          },
        }, ' ×')))) : null,
      h('div', { class: 'row' }, open, save, link, listPicker)));
}

/** The reader's own lists: made, renamed and deleted here. */
function listManager(ctx, redraw) {
  const user = ctx.user();
  const name = h('input', { type: 'text', placeholder: 'Name of a new list', 'aria-label': 'Name of a new list', maxlength: 60 });
  const create = h('button', {
    onclick: () => {
      const text = name.value.trim();
      if (!text) return;
      user.video_lists.push({ id: newId('l'), name: text, videos: [] });
      ctx.saveUser();
      redraw();
    },
  }, 'Create');
  return h('details', { class: 'channels' },
    h('summary', {}, `Your lists (${user.video_lists.length})`),
    h('ul', {}, user.video_lists.map((l) => h('li', {},
      h('span', {}, l.name), h('span', { class: 'muted' }, ` ${l.videos.length} videos`),
      h('button', {
        class: 'quiet danger',
        onclick: () => {
          user.video_lists = user.video_lists.filter((x) => x.id !== l.id);
          ctx.saveUser();
          redraw();
        },
      }, 'Delete')))),
    h('div', { class: 'row' }, name, create));
}

function option(value, label) {
  return h('option', { value }, label);
}

export async function renderVideos(container, ctx, options = {}) {
  let data;
  try {
    data = await catalog();
  } catch (error) {
    container.append(h('p', { class: 'empty-state' }, `The catalog could not be read: ${error}`));
    return;
  }
  const importButton = h('button', {
    class: 'quiet',
    onclick: async () => {
      const path = await pickFile('Import a video catalog', [{ name: 'GRT video catalog', extensions: ['grt'] }]);
      if (!path) return;
      try {
        const count = await api.importVideos(path);
        forgetVideos();
        toast(`Catalog imported: ${count} videos.`);
        ctx.refresh();
      } catch (error) {
        toast(String(error), 6000);
      }
    },
  }, 'Import a catalog…');
  const notice = h('p', { class: 'fine' }, onAndroid()
    ? 'Videos open in your browser or video app. GRT Bible itself has no permission to use the internet, and is not affiliated with the channels it lists.'
    : 'Videos open in your web browser. GRT Bible itself does not connect to the internet, and is not affiliated with the channels it lists.');

  const { channels, videos } = data;
  if (!videos.length) {
    container.append(
      h('p', { class: 'empty-state' }, 'The catalog is empty. A catalog package adds videos about the passages you read.'),
      importButton, notice);
    return;
  }

  const byChannel = new Map(channels.map((c) => [c.id, c]));
  const chapterIds = new Set(ctx.chapterVideos());
  const user = ctx.user();
  const selection = ctx.selectionRange?.();
  const at = options.at || null;
  const categories = CATEGORIES.filter((c) => videos.some((v) => v.category === c));
  for (const v of videos) {
    if (v.category && !categories.includes(v.category)) categories.push(v.category);
  }
  const count = (test) => videos.filter(test).length;

  const filter = h('input', { type: 'search', placeholder: 'Filter by title', 'aria-label': 'Filter videos', autocapitalize: 'none' });
  const category = h('select', { 'aria-label': 'Category' },
    option('', `All categories (${videos.length})`),
    categories.map((c) => option(c, `${c} (${count((v) => v.category === c)})`)));
  const channel = h('select', { 'aria-label': 'Channel' },
    option('', 'All channels'),
    channels.map((c) => option(c.id, `${c.name} (${c.videos})`)));
  const scope = h('select', { 'aria-label': 'Which videos' },
    option('all', 'All videos'),
    option('chapter', `About this chapter (${chapterIds.size})`),
    selection ? option('selection', `About ${selection.label}`) : null,
    at ? option('verse', 'About this verse') : null,
    option('saved', 'Saved'),
    user.video_lists.map((l) => option(`list:${l.id}`, `${l.name} (${l.videos.length})`)));
  scope.value = options.scope === 'verse' && at ? 'verse' : options.scope === 'chapter' && chapterIds.size ? 'chapter' : 'all';

  const channelList = h('details', { class: 'channels' },
    h('summary', {}, `Channels (${channels.length})`),
    h('ul', {}, channels.map((c) => {
      const open = h('button', { class: 'quiet', onclick: () => openWith(open, () => api.openChannel(c.id), opened(c.name)) }, 'Open channel');
      return h('li', {},
        h('button', {
          class: 'link',
          onclick: () => {
            channel.value = c.id;
            draw();
          },
        }, c.name),
        h('span', { class: 'muted' }, ` ${c.videos} videos`),
        open);
    })));

  const status = h('p', { class: 'search-status', 'aria-live': 'polite' });
  const list = h('div', { class: 'video-list' });
  const more = h('button', { class: 'more', hidden: true }, 'More videos');
  let shown = [];
  let drawn = 0;

  const page = () => {
    const next = shown.slice(drawn, drawn + PAGE);
    list.append(...next.map((v) => card(v, byChannel.get(v.channel), ctx, draw)));
    drawn += next.length;
    more.hidden = drawn >= shown.length;
  };

  const draw = () => {
    const needle = filter.value.trim().toLowerCase();
    const saved = new Set(user.saved_videos);
    const listed = scope.value.startsWith('list:') ? new Set(user.video_lists.find((l) => `list:${l.id}` === scope.value)?.videos || []) : null;
    shown = videos.filter((v) =>
      (!category.value || v.category === category.value)
      && (!channel.value || v.channel === channel.value)
      && (scope.value !== 'chapter' || chapterIds.has(v.id))
      && (scope.value !== 'selection' || videoTouches(v, user.video_anchors, selection.start, selection.end))
      && (scope.value !== 'verse' || videoTouches(v, user.video_anchors, at, at))
      && (scope.value !== 'saved' || saved.has(v.id))
      && (!listed || listed.has(v.id))
      && (!needle || v.title.toLowerCase().includes(needle)));
    clear(list);
    drawn = 0;
    status.textContent = shown.length === 1 ? '1 video' : `${shown.length} videos`;
    if (!shown.length) {
      list.append(h('p', { class: 'empty-state' }, 'No videos here.'));
      more.hidden = true;
      return;
    }
    page();
  };

  filter.addEventListener('input', draw);
  for (const control of [category, channel, scope]) control.addEventListener('change', draw);
  more.addEventListener('click', page);

  const lists = listManager(ctx, () => ctx.refresh());
  container.append(
    h('div', { class: 'search-form' }, filter, h('div', { class: 'row' }, category, channel), h('div', { class: 'row' }, scope, importButton)),
    selection ? h('p', { class: 'fine' }, `Selected: ${selection.label}. A video's “Link to” button shows it beside these verses.`) : null,
    channelList, lists, status, list, more, notice);
  draw();
}
