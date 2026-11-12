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

function bust() {
  try {
    const links = [...document.querySelectorAll('link[rel="stylesheet"]')];
    for (const l of links) {
      if (typeof l.href==='string' && l.href.includes(DEV_CSS_PATH)) {
        const u = new URL(l.href, location.origin);
        u.searchParams.set('t', String(Date.now()));
        l.href=u.pathname+u.search;
      }
    }
  } catch {}
}

// initial attempt to ensure we have a fresh version after client connects
if (document.readyState !== 'loading') {
  bust();
} else {
  document.addEventListener('DOMContentLoaded', bust);
}

if (import.meta.hot){
  import.meta.hot.on('stylex:css-update', bust);
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
