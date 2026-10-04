/** The video section: links chosen by the author, opened in the system browser. */

import { api, onAndroid, pickFile } from './api.js';
import { clear, h } from './dom.js';
import { toast } from './overlay.js';

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

function card(video, channel, ctx) {
  const user = ctx.user();
  const saved = user.saved_videos.includes(video.id);
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
  return h('article', { class: 'card video', dataset: { video: video.id } },
    video.thumb ? h('img', { src: video.thumb, alt: '', class: 'thumb', loading: 'lazy' }) : null,
    h('div', { class: 'video-body' },
      h('h3', {}, video.title),
      h('p', { class: 'muted' }, [channel?.name || video.source, video.category, duration(video.durationS)].filter(Boolean).join(' · ')),
      video.description ? h('p', {}, video.description) : null,
      h('div', { class: 'row' }, open, save)));
}

function option(value, label) {
  return h('option', { value }, label);
}

export async function renderVideos(container, ctx, chapterOnly = false) {
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
    option('saved', 'Saved'));
  scope.value = chapterOnly && chapterIds.size ? 'chapter' : 'all';

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
    list.append(...next.map((v) => card(v, byChannel.get(v.channel), ctx)));
    drawn += next.length;
    more.hidden = drawn >= shown.length;
  };

  const draw = () => {
    const needle = filter.value.trim().toLowerCase();
    const saved = new Set(ctx.user().saved_videos);
    shown = videos.filter((v) =>
      (!category.value || v.category === category.value)
      && (!channel.value || v.channel === channel.value)
      && (scope.value !== 'chapter' || chapterIds.has(v.id))
      && (scope.value !== 'saved' || saved.has(v.id))
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

  container.append(
    h('div', { class: 'search-form' }, filter, h('div', { class: 'row' }, category, channel), h('div', { class: 'row' }, scope, importButton)),
    channelList, status, list, more, notice);
  draw();
}
