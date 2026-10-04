/** One modal layer at a time, closed with Escape or a click outside. */

import { $, clear, h } from './dom.js';

let closeHandler = null;
let previousFocus = null;

export function isOpen() {
  return !$('#overlay').hidden;
}

export function openOverlay(content, { label, onClose = null, wide = false } = {}) {
  closeOverlay();
  const overlay = $('#overlay');
  previousFocus = document.activeElement;
  const dialog = h('div', { class: `dialog${wide ? ' wide' : ''}`, role: 'dialog', 'aria-modal': 'true', 'aria-label': label });
  dialog.append(content);
  // A dialog that fills a phone screen has nothing to tap outside it.
  dialog.append(h('button', { class: 'dialog-close quiet', 'aria-label': 'Close', onclick: closeOverlay }, '✕'));
  clear(overlay, dialog);
  overlay.hidden = false;
  closeHandler = onClose;
  overlay.onclick = (event) => {
    if (event.target === overlay) closeOverlay();
  };
  const focusable = dialog.querySelector('[autofocus], input, select, textarea, button');
  if (focusable) focusable.focus();
  return dialog;
}

export function closeOverlay() {
  const overlay = $('#overlay');
  if (!overlay || overlay.hidden) return;
  overlay.hidden = true;
  overlay.replaceChildren();
  const handler = closeHandler;
  closeHandler = null;
  if (handler) handler();
  if (previousFocus && document.contains(previousFocus)) previousFocus.focus();
}

let toastTimer = null;

export function toast(message, ms = 3200) {
  const el = $('#toast');
  el.textContent = message;
  el.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    el.hidden = true;
  }, ms);
}
