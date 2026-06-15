import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const $text = document.getElementById('text');
const $source = document.getElementById('source');
const $hint = document.getElementById('hint');
const $refresh = document.getElementById('refresh');
const $body = document.body;

const SWAP_OUT_MS = 560;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

let currentSig = '';
let swapping = false;

function fill(entry) {
  $text.textContent = entry.text;
  $source.textContent = entry.source ? `via ${entry.source}` : '';
  $hint.textContent = entry.date || '';
}

function fillError(msg) {
  $text.textContent = '今日尚未生成';
  $source.textContent = '';
  $hint.textContent = (msg || '').toString().slice(0, 80);
}

async function setEntry(entry) {
  if (!entry || !entry.text) return;
  const sig = `${entry.date}|${entry.text}`;
  if (sig === currentSig) {
    $body.classList.add('ready');
    return;
  }
  const isFirst = !$body.classList.contains('ready');
  currentSig = sig;

  if (isFirst) {
    fill(entry);
    $body.classList.add('ready');
    return;
  }

  if (swapping) return;
  swapping = true;
  try {
    // fade old up & out
    $body.classList.add('swap-out');
    await sleep(SWAP_OUT_MS);

    // replace content while invisible & shifted down
    $body.classList.remove('swap-out');
    $body.classList.add('swap-in');
    fill(entry);
    // force reflow so the swap-in styles commit before we drop them
    void $text.offsetWidth;

    // unleash transitions back, settle into place
    $body.classList.remove('swap-in');
  } finally {
    swapping = false;
  }
}

function showError(msg) {
  fillError(msg);
  $body.classList.remove('swap-out', 'swap-in', 'loading');
  $body.classList.add('ready');
}

async function init() {
  try {
    const entry = await invoke('get_today');
    await setEntry(entry);
  } catch (e) {
    showError(e);
  }
}

async function doRefresh() {
  if ($body.classList.contains('loading')) return;
  $body.classList.add('loading');
  try {
    const entry = await invoke('refresh');
    await setEntry(entry);
  } catch (e) {
    showError(e);
  } finally {
    $body.classList.remove('loading');
  }
}

listen('aphorism', (e) => setEntry(e.payload));
listen('aphorism-error', (e) => showError(e.payload));

$refresh.addEventListener('click', (e) => {
  e.stopPropagation();
  doRefresh();
});
// 双击空白处也可触发
$body.addEventListener('dblclick', (e) => {
  if ($refresh.contains(e.target)) return;
  doRefresh();
});

init();
