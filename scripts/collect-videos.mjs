// Lists the videos of the channels in catalog/channels.json into catalog/catalog.json.
//
//   node scripts/collect-videos.mjs
//   node scripts/collect-videos.mjs --raw listing.json
//
// Categories and references already in catalog.json are kept, so hand corrections survive a refresh.

import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { BOOKS } from '../src/js/books.js';

const root = new URL('..', import.meta.url);
const channelsFile = new URL('catalog/channels.json', root);
const catalogFile = new URL('catalog/catalog.json', root);
const canonFile = new URL('crates/grtb/data/canon.txt', root);

const HEADERS = {
  'User-Agent': 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36',
  'Accept-Language': 'en-US,en;q=0.9',
  Cookie: 'SOCS=CAI; CONSENT=YES+cb',
};

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function textOf(t) {
  if (!t) return '';
  if (typeof t === 'string') return t;
  if (t.simpleText) return t.simpleText;
  if (t.runs) return t.runs.map((r) => r.text || '').join('');
  if (t.content) return t.content;
  return '';
}

function seconds(label) {
  if (!label || !/^\d+(:\d+)*$/.test(label.trim())) return null;
  return label.trim().split(':').reduce((total, part) => total * 60 + Number(part), 0);
}

function walk(node, out) {
  if (Array.isArray(node)) {
    for (const child of node) walk(child, out);
  } else if (node && typeof node === 'object') {
    for (const [key, value] of Object.entries(node)) {
      if (['videoRenderer', 'gridVideoRenderer', 'lockupViewModel', 'continuationItemRenderer'].includes(key)) out.push([key, value]);
      else walk(value, out);
    }
  }
}

function item(kind, node) {
  if (kind === 'videoRenderer' || kind === 'gridVideoRenderer') {
    let length = textOf(node.lengthText);
    for (const overlay of node.thumbnailOverlays || []) {
      if (!length && overlay.thumbnailOverlayTimeStatusRenderer) length = textOf(overlay.thumbnailOverlayTimeStatusRenderer.text);
    }
    return { id: node.videoId, title: textOf(node.title), duration: seconds(length) };
  }
  if (kind === 'lockupViewModel') {
    if (node.contentType && node.contentType !== 'LOCKUP_CONTENT_TYPE_VIDEO') return null;
    const meta = node.metadata?.lockupMetadataViewModel || {};
    const badge = JSON.stringify(node.contentImage || {}).match(/"text":"(\d+(?::\d+)+)"/);
    return { id: node.contentId, title: textOf(meta.title), duration: badge ? seconds(badge[1]) : null };
  }
  return null;
}

async function fetchTab(handle, tab) {
  const html = await (await fetch(`https://www.youtube.com/${handle}/${tab}`, { headers: HEADERS })).text();
  const data = JSON.parse(html.match(/var ytInitialData = (\{.*?\});<\/script>/s)[1]);
  const key = html.match(/"INNERTUBE_API_KEY":"([^"]+)"/)?.[1];
  const version = html.match(/"INNERTUBE_CONTEXT_CLIENT_VERSION":"([^"]+)"/)?.[1] || '2.20250901.00.00';
  const meta = data.metadata?.channelMetadataRenderer || {};
  const channel = { id: meta.externalId, name: meta.title, url: `https://www.youtube.com/${handle}` };
  const videos = [];
  let nodes = [];
  walk(data.contents, nodes);
  for (;;) {
    let token = null;
    for (const [kind, node] of nodes) {
      if (kind === 'continuationItemRenderer') {
        token = node.continuationEndpoint?.continuationCommand?.token || token;
        continue;
      }
      const video = item(kind, node);
      if (video?.id) videos.push(video);
    }
    if (!token || !key) break;
    const body = JSON.stringify({ context: { client: { clientName: 'WEB', clientVersion: version, hl: 'en', gl: 'US' } }, continuation: token });
    let reply = null;
    for (let attempt = 0; attempt < 4 && !reply; attempt++) {
      try {
        const response = await fetch(`https://www.youtube.com/youtubei/v1/browse?key=${key}&prettyPrint=false`, {
          method: 'POST', headers: { ...HEADERS, 'Content-Type': 'application/json' }, body,
        });
        if (response.ok) reply = await response.json();
      } catch {
        await sleep(2000 + attempt * 3000);
      }
    }
    if (!reply) break;
    nodes = [];
    walk(reply.onResponseReceivedActions || [], nodes);
    await sleep(400);
  }
  return { channel, videos };
}

async function listing(handles) {
  const out = [];
  for (const handle of handles) {
    const entry = { handle, channel: null, videos: [], streams: [] };
    for (const tab of ['videos', 'streams']) {
      try {
        const { channel, videos } = await fetchTab(handle, tab);
        entry.channel = entry.channel || channel;
        entry[tab] = videos;
        console.error(`${handle}/${tab}: ${videos.length}`);
      } catch (error) {
        console.error(`${handle}/${tab}: ${error.message}`);
      }
    }
    if (entry.channel) out.push(entry);
  }
  return out;
}

// Categories

const DEBATE = /\b(debat\w*|vs\.?|versus|cross[- ]examination|rebut\w*|refut\w*|respond(s|ing)? to|response to|react(s|ing)? to|reaction to|debunk\w*|heated|exposed|destroy(s|ed)|disagree\w*|objections?|argu(e|es|ing)|critique)\b/i;

const DOCTRINE = new RegExp('\\b(' + [
  'eucharist\\w*', 'communion', 'real presence', 'transubstantiation', 'mass', 'masses', 'liturg\\w*', 'latin mass', 'tlm', 'novus ordo',
  'mary', "mary's", 'marian', 'virgin', 'immaculate', 'assumption', 'rosary', 'theotokos', 'mother of god',
  'pope\\w*', 'papa\\w*', 'vatican', 'magisterium', 'infallib\\w*', 'catechism', 'dogma\\w*', 'doctrine\\w*',
  'here(sy|sies|tic\\w*)', 'schism\\w*', 'apostolic succession', 'sacred tradition', 'sola scriptura', 'sola fide', 'faith alone',
  'justification', 'salvation', 'saved', 'grace', 'mortal sin', 'venial', 'original sin', 'sins?', 'purgatory', 'hell', 'heaven',
  'confession', 'penance', 'reconciliation', 'sacraments?', 'baptism', 'baptized', 'confirmation', 'anointing', 'holy orders',
  'priesthood', 'married priests', 'women priests', 'ordination', 'church fathers', 'early church', 'ecumenical councils?', 'nicaea',
  'annulment', 'divorce', 'contraception', 'nfp', 'ivf', 'abortion', 'euthanasia', 'celibacy', 'chastity',
  'intercession', 'communion of saints', 'relics?', 'indulgences?', 'canoniz\\w*', 'trinity', 'incarnation', 'predestination',
  'catholic church', 'church teaching', 'church teaches', 'deuterocanon\\w*', 'canon of scripture',
].join('|') + ')\\b', 'i');

export function categorise(title) {
  if (DEBATE.test(title)) return 'Debates';
  if (DOCTRINE.test(title)) return 'Doctrine';
  return 'Theology';
}

// Bible references named in a title, in English numbering

const canon = new Map(readFileSync(canonFile, 'utf8').trim().split(/\r?\n/).map((line) => {
  const [osis, ...counts] = line.split(' ');
  return [osis, counts.map(Number)];
}));

const NAMES = [];
for (const book of BOOKS) {
  const [english] = book.names.en;
  NAMES.push([english, book.osis]);
  if (book.osis === 'Ps') NAMES.push(['Psalm', 'Ps']);
  if (book.osis === 'Song') NAMES.push(['Song of Solomon', 'Song']);
  if (book.osis === 'Rev') NAMES.push(['Revelations', 'Rev']);
  const numbered = english.match(/^([123]) (.+)$/);
  if (numbered) {
    const ordinal = ['', 'First', 'Second', 'Third'][Number(numbered[1])];
    NAMES.push([`${ordinal} ${numbered[2]}`, book.osis], [`${numbered[1]}${numbered[2]}`, book.osis]);
  }
}
NAMES.sort((a, b) => b[0].length - a[0].length);
const REFERENCE = new RegExp(`\\b(${NAMES.map(([n]) => n.replace(/ /g, '\\s+')).join('|')})\\s+(\\d{1,3})(?::(\\d{1,3})(?:\\s*[-–]\\s*(\\d{1,3}))?)?(?![\\d:])`, 'gi');

export function referencesIn(title) {
  const refs = [];
  for (const match of title.matchAll(REFERENCE)) {
    const name = match[1].replace(/\s+/g, ' ').toLowerCase();
    const osis = NAMES.find(([n]) => n.toLowerCase() === name)?.[1];
    const chapter = Number(match[2]);
    const counts = canon.get(osis);
    if (!osis || !counts || chapter < 1 || chapter > counts.length + 1) continue;
    const last = counts[chapter - 1] || 1;
    if (match[3]) {
      const verse = Number(match[3]);
      const end = match[4] ? Number(match[4]) : null;
      refs.push(end && end > verse ? { start: `${osis}.${chapter}.${verse}`, end: `${osis}.${chapter}.${end}` } : { start: `${osis}.${chapter}.${verse}` });
    } else {
      refs.push({ start: `${osis}.${chapter}.1`, end: `${osis}.${chapter}.${last}` });
    }
  }
  return refs;
}

// Catalog

function build(raw, previous) {
  const kept = new Map((previous?.videos || []).map((v) => [v.id, v]));
  const channels = [];
  const videos = [];
  const seen = new Set();
  for (const entry of raw) {
    const channel = { id: entry.channel.id, name: entry.channel.name, url: `https://www.youtube.com/${entry.handle}` };
    channels.push(channel);
    for (const video of [...entry.videos, ...entry.streams]) {
      if (seen.has(video.id)) continue;
      seen.add(video.id);
      const old = kept.get(video.id);
      videos.push({
        id: video.id,
        title: video.title,
        url: `https://www.youtube.com/watch?v=${video.id}`,
        channel: channel.id,
        category: old?.category || categorise(video.title),
        source: channel.name,
        language: 'en',
        ...(video.duration ? { duration_s: video.duration } : {}),
        description: '',
        refs: old?.refs || referencesIn(video.title),
        tags: old?.tags || [],
      });
    }
  }
  return { version: 1, reference_scheme: 'english', channels, videos };
}

async function main() {
  const args = process.argv.slice(2);
  const rawAt = args.indexOf('--raw');
  const handles = JSON.parse(readFileSync(channelsFile, 'utf8')).channels;
  const raw = rawAt >= 0 ? JSON.parse(readFileSync(args[rawAt + 1], 'utf8')) : await listing(handles);
  const previous = existsSync(catalogFile) ? JSON.parse(readFileSync(catalogFile, 'utf8')) : null;
  const catalog = build(raw, previous);
  writeFileSync(catalogFile, `${JSON.stringify(catalog, null, 2)}\n`);
  const counts = {};
  for (const v of catalog.videos) counts[v.category] = (counts[v.category] || 0) + 1;
  console.error(`${catalog.videos.length} videos from ${catalog.channels.length} channels`, counts,
    `${catalog.videos.filter((v) => v.refs.length).length} with a Bible reference`);
}

if (process.argv[1] && import.meta.url.endsWith(process.argv[1].split('/').pop())) {
  main();
}
