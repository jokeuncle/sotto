import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const $text = document.getElementById('text');
const $source = document.getElementById('source');
const $hint = document.getElementById('hint');

function render(entry) {
  if (!entry || !entry.text) return;
  $text.textContent = entry.text;
  $source.textContent = entry.source ? `via ${entry.source}` : '';
  $hint.textContent = entry.date || '';
  document.body.classList.remove('loading');
  document.body.classList.add('ready');
}

function showError(msg) {
  $text.textContent = '今日尚未生成';
  $source.textContent = '';
  $hint.textContent = (msg || '').toString().slice(0, 80);
  document.body.classList.remove('loading');
  document.body.classList.add('ready');
}

async function init() {
  document.body.classList.add('loading');
  try {
    const entry = await invoke('get_today');
    render(entry);
  } catch (e) {
    showError(e);
  }
}

listen('aphorism', (e) => render(e.payload));
listen('aphorism-error', (e) => showError(e.payload));

// 双击窗口可手动刷新（生成新的一句）
document.body.addEventListener('dblclick', async () => {
  document.body.classList.add('loading');
  try {
    const entry = await invoke('refresh');
    render(entry);
  } catch (e) {
    showError(e);
  }
});

init();
