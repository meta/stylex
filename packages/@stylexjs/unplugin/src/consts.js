/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

export const DEV_CSS_PATH = '/virtual:stylex.css';
export const DEV_RUNTIME_PATH = '/virtual:stylex.js';
export const DEV_AFTER_UPDATE_DELAY = 180; // ms

export const VIRTUAL_STYLEX_RUNTIME_SCRIPT = `
const STYLE_ID = '__stylex_virtual__';
const DEV_CSS_PATH = '${DEV_CSS_PATH}';
const AFTER_UPDATE_DELAY = ${DEV_AFTER_UPDATE_DELAY};
let lastCSS = '';

function ensure(){
  let el=document.getElementById(STYLE_ID);
  if (!el) { 
    el=document.createElement('style');
    el.id=STYLE_ID;document.head.appendChild(el);
  }
  return el;
}

function disableLink() {
  try{
    const links = [...document.querySelectorAll('link[rel="stylesheet"]')];
    for (const l of links) {
      if(typeof l.href==='string' && l.href.includes(DEV_CSS_PATH)) {
        // Keep server-rendered link attributes unchanged during hydration.
        if (l.sheet) {
          l.sheet.disabled = true;
        } else {
          l.addEventListener('load', () => {
            if (l.sheet) l.sheet.disabled = true;
          }, {once: true});
        }
      }
    }
  } catch {}
}

async function fetchCSS() {
  const t = Date.now();
  const r = await fetch(DEV_CSS_PATH + '?t=' + t, {cache: 'no-store'});
  return r.text();
}

async function update(){
  try {
    const css = await fetchCSS();
    if (css!==lastCSS) {
      ensure().textContent = css;
      disableLink();
      lastCSS = css;
    }
  } catch {}
}

update();

if(import.meta.hot){
  import.meta.hot.on('stylex:css-update', update);
  import.meta.hot.on('vite:afterUpdate', () => setTimeout(update, AFTER_UPDATE_DELAY));
  import.meta.hot.dispose(() => {
    const el = document.getElementById(STYLE_ID);
    if (el && el.parentNode) el.parentNode.removeChild(el);
  });
}

export {};
`;

export const VIRTUAL_STYLEX_CSS_ONLY_SCRIPT = `
const DEV_CSS_PATH='${DEV_CSS_PATH}';
const pending = new Map();
const pendingLinks = new Set();
let timestamp = 0;
let disposed = false;

function bust() {
  if (disposed) return;
  try {
    timestamp = Math.max(Date.now(), timestamp + 1);
    const links = [...document.querySelectorAll('link[rel="stylesheet"]')]
      .filter(link => !pendingLinks.has(link));
    for (const link of links) {
      const url = new URL(link.href, location.origin);
      if (url.origin !== location.origin || !url.pathname.endsWith(DEV_CSS_PATH)) continue;
      pending.get(link)?.cancel();
      if (!link.parentNode || link.disabled || link.sheet?.disabled) continue;

      const markup = link.outerHTML;
      const replacement = link.cloneNode();
      url.searchParams.set('t', String(timestamp));
      replacement.href = url.pathname + url.search;
      replacement.disabled = link.disabled;

      function cleanup() {
        replacement.removeEventListener('load', onLoad);
        replacement.removeEventListener('error', cancel);
        pendingLinks.delete(replacement);
        if (pending.get(link) === entry) pending.delete(link);
      }
      function cancel() {
        cleanup();
        replacement.remove();
      }
      function onLoad() {
        if (pending.get(link) !== entry) return;
        if (
          !link.isConnected || link.nextSibling !== replacement ||
          link.outerHTML !== markup || link.disabled || link.sheet?.disabled
        ) {
          cancel();
          return;
        }
        cleanup();
        link.remove();
      }
      const entry = {cancel};
      pending.set(link, entry);
      pendingLinks.add(replacement);
      replacement.addEventListener('load', onLoad);
      replacement.addEventListener('error', cancel);
      // Keep the old sheet effective and preserve its position in the cascade.
      link.after(replacement);
    }
  } catch {}
}

// initial attempt to ensure we have a fresh version after client connects
if (document.readyState !== 'loading') {
  bust();
} else {
  document.addEventListener('DOMContentLoaded', bust, {once: true});
}

if (import.meta.hot){
  import.meta.hot.on('stylex:css-update', bust);
  import.meta.hot.dispose(() => {
    disposed = true;
    import.meta.hot.off('stylex:css-update', bust);
    document.removeEventListener('DOMContentLoaded', bust);
    for (const entry of pending.values()) entry.cancel();
  });
}

export {};
`;

export const DEV_RUNTIME_SCRIPT = `
const STYLE_ID='__stylex_virtual__';
const DEV_CSS_PATH = '${DEV_CSS_PATH}';
const AFTER_UPDATE_DELAY = 180;
const POLL_INTERVAL = 800;
let lastCSS='';

function ensure() {
  let el = document.getElementById(STYLE_ID);
  if(!el){
    el = document.createElement('style');
    el.id = STYLE_ID;
    document.head.appendChild(el);
  }
  return el;
}
function disableLink() {
  try {
    const links = [ ...document.querySelectorAll('link[rel="stylesheet"]') ];
    for(const l of links) {
      if(typeof l.href==='string' && l.href.includes(DEV_CSS_PATH)) {
        // Keep server-rendered link attributes unchanged during hydration.
        if (l.sheet) {
          l.sheet.disabled = true;
        } else {
          l.addEventListener('load', () => {
            if (l.sheet) l.sheet.disabled = true;
          }, {once: true});
        }
      }
    }
  } catch {
    // ignore
  }
}
async function fetchCSS(){
  const t=Date.now();
  const r=await fetch(DEV_CSS_PATH+'?t='+t,{cache:'no-store'});
  return r.text();
}
async function update(){
  try {
    const css=await fetchCSS();
    if(css!==lastCSS){
      ensure().textContent = css;
      disableLink();
      lastCSS = css;
    }
  } catch {}
}
update();

if(import.meta.hot){
  import.meta.hot.on('stylex:css-update', update);
  import.meta.hot.on('vite:afterUpdate',() => setTimeout(update, AFTER_UPDATE_DELAY));
}
`;
