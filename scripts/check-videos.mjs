// Reports the links in catalog/catalog.json that no longer work.
//
//   node scripts/check-videos.mjs [catalog.json]

import { readFileSync } from 'node:fs';

const path = process.argv[2] || new URL('../catalog/catalog.json', import.meta.url);
const catalog = JSON.parse(readFileSync(path, 'utf8'));

// YouTube answers 200 for removed videos, so its oEmbed endpoint is asked instead.
function probeUrl(url) {
  const host = new URL(url).hostname;
  if (/(^|\.)youtube(-nocookie)?\.com$|^youtu\.be$/.test(host)) {
    return `https://www.youtube.com/oembed?format=json&url=${encodeURIComponent(url)}`;
  }
  return url;
}

let broken = 0;
for (const video of catalog.videos) {
  let status;
  try {
    const response = await fetch(probeUrl(video.url), { method: 'GET', redirect: 'follow' });
    status = response.status;
  } catch (error) {
    status = error.cause?.code || 'error';
  }
  const ok = status === 200;
  if (!ok) broken += 1;
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${status} ${video.id} ${video.url}`);
}
console.log(`${catalog.videos.length} checked, ${broken} broken.`);
process.exit(broken ? 1 : 0);
