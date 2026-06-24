import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const $text = document.getElementById('text');
const $source = document.getElementById('source');
const $hint = document.getElementById('hint');
const $refresh = document.getElementById('refresh');
const $share = document.getElementById('share');
const $history = document.getElementById('history');
const $historyPanel = document.getElementById('historyPanel');
const $historyClose = document.getElementById('historyClose');
const $historyList = document.getElementById('historyList');
const $settings = document.getElementById('settings');
const $settingsPanel = document.getElementById('settingsPanel');
const $settingsClose = document.getElementById('settingsClose');
const $settingsSave = document.getElementById('settingsSave');
const $tipOpen = document.getElementById('tipOpen');
const $tipPanel = document.getElementById('tipPanel');
const $tipClose = document.getElementById('tipClose');
const $personalNote = document.getElementById('personalNote');
const $styles = document.getElementById('styles');
const $toast = document.getElementById('toast');
const $body = document.body;

const SWAP_OUT_MS = 560;
const SHARE_COOLDOWN_MS = 1800;
const isTauri = Boolean(window.__TAURI_INTERNALS__);

const STYLE_LABELS = {
  commute: '通勤',
  night: '深夜',
  work: '上班',
  tender: '关系',
  weekend: '周末',
};

const RHYTHM_LABELS = {
  moments: '多时段',
  daily: '每日',
  manual: '手动',
};

const SHARPNESS_LABELS = {
  soft: '柔',
  quiet: '静',
  sharp: '刺',
};

const SLOT_LABELS = {
  after_midnight: '00:05',
  morning: '09:05',
  noon: '12:35',
  evening: '18:05',
  night: '23:20',
  daily: 'daily',
  manual: 'manual',
};

const demoEntry = {
  date: '2026-06-24',
  text: '电梯到十七层才停，镜子里的人终于把表情收回口袋。',
  source: 'preview',
  generated_at: new Date().toISOString(),
  style: 'commute',
  slot: 'evening',
};

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

let currentSig = '';
let currentEntry = null;
let swapping = false;
let sharing = false;
let shareLockedUntil = 0;
let shareStateTimer = 0;
let toastTimer = 0;
let settingsDraft = {
  style: 'commute',
  rhythm: 'moments',
  sharpness: 'quiet',
  personal_note: '',
};

async function call(command, args = {}) {
  if (isTauri) return invoke(command, args);

  if (command === 'get_settings') return { ...settingsDraft };
  if (command === 'get_history') {
    return [
      demoEntry,
      {
        ...demoEntry,
        text: '便利店的灯太亮，显得深夜买来的安慰也有保质期。',
        style: 'night',
        slot: 'night',
      },
      {
        ...demoEntry,
        text: '会议结束后，屏幕还亮着，像一份没来得及撤回的疲惫。',
        style: 'work',
        slot: 'noon',
      },
    ];
  }
  if (command === 'set_style') {
    settingsDraft.style = args.style;
    return {
      ...demoEntry,
      style: args.style,
      text: demoLine(args.style),
      generated_at: new Date().toISOString(),
    };
  }
  if (command === 'save_settings') {
    settingsDraft = { ...settingsDraft, ...args.settings };
    return {
      ...demoEntry,
      style: settingsDraft.style,
      text: demoLine(settingsDraft.style),
      generated_at: new Date().toISOString(),
    };
  }
  return { ...demoEntry, ...settingsDraft };
}

function demoLine(style) {
  const lines = {
    commute: '地铁进站前风先替人抵达，没人承认自己也被推着往前。',
    night: '冰箱在深夜低声运转，像替房间保存一点没说出口的清醒。',
    work: '表格保存成功的提示太短，装不下刚才那十分钟的自我怀疑。',
    tender: '消息框里的草稿删到最后，只剩一个标点还在替人犹豫。',
    weekend: '洗衣机停下后，周末终于露出一点不必解释的空白。',
  };
  return lines[style] || lines.commute;
}

function fill(entry) {
  currentEntry = entry;
  $text.textContent = entry.text;
  const style = STYLE_LABELS[entry.style] || STYLE_LABELS[settingsDraft.style] || '';
  $source.textContent = entry.source ? `via ${entry.source}${style ? ` · ${style}` : ''}` : style;
  const slot = SLOT_LABELS[entry.slot] || '';
  $hint.textContent = [entry.date, slot].filter(Boolean).join(' · ');
}

function fillError(msg) {
  $text.textContent = '今日尚未生成';
  $source.textContent = '';
  $hint.textContent = (msg || '').toString().slice(0, 80);
}

async function setEntry(entry) {
  if (!entry || !entry.text) return;
  const sig = `${entry.generated_at || entry.date}|${entry.text}`;
  if (sig === currentSig) {
    $body.classList.add('ready');
    return;
  }
  const isFirst = !$body.classList.contains('ready');
  currentSig = sig;

  if (entry.style) {
    settingsDraft.style = entry.style;
    syncControls();
  }

  if (isFirst) {
    fill(entry);
    $body.classList.add('ready');
    return;
  }

  if (swapping) return;
  swapping = true;
  try {
    $body.classList.add('swap-out');
    await sleep(SWAP_OUT_MS);
    $body.classList.remove('swap-out');
    $body.classList.add('swap-in');
    fill(entry);
    void $text.offsetWidth;
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

function showToast(message) {
  clearTimeout(toastTimer);
  $toast.textContent = message;
  $body.classList.add('toast-on');
  toastTimer = window.setTimeout(() => {
    $body.classList.remove('toast-on');
  }, 1500);
}

function setShareState(state) {
  clearTimeout(shareStateTimer);
  $share.dataset.state = state;
  $share.classList.toggle('is-busy', state === 'busy');
  $share.classList.toggle('is-done', state === 'done');
  $share.disabled = state === 'busy' || state === 'done';

  const labels = {
    busy: '正在生成分享卡片',
    done: '已下载分享卡片',
    idle: '下载分享卡片',
  };
  $share.setAttribute('aria-label', labels[state] || labels.idle);
  $share.title = labels[state] || labels.idle;
}

async function loadSettings() {
  const settings = await call('get_settings');
  settingsDraft = {
    style: settings.style || 'commute',
    rhythm: settings.rhythm || 'moments',
    sharpness: settings.sharpness || 'quiet',
    personal_note: settings.personal_note || '',
  };
  $personalNote.value = settingsDraft.personal_note;
  syncControls();
}

async function init() {
  try {
    await loadSettings();
    const entry = await call('get_today');
    await setEntry(entry);
  } catch (e) {
    showError(e);
  }
}

async function doRefresh() {
  if ($body.classList.contains('loading')) return;
  $body.classList.add('loading');
  try {
    const entry = await call('refresh');
    await setEntry(entry);
    await loadHistory();
  } catch (e) {
    showError(e);
  } finally {
    $body.classList.remove('loading');
  }
}

async function changeStyle(style) {
  if ($body.classList.contains('loading')) return;
  settingsDraft.style = style;
  syncControls();
  $body.classList.add('loading');
  try {
    const entry = await call('set_style', { style });
    await setEntry(entry);
    await loadSettings();
    await loadHistory();
  } catch (e) {
    showError(e);
  } finally {
    $body.classList.remove('loading');
  }
}

async function saveSettings() {
  if ($body.classList.contains('loading')) return;
  settingsDraft.personal_note = $personalNote.value.trim();
  syncControls();
  $body.classList.add('loading');
  try {
    const entry = await call('save_settings', { settings: settingsDraft });
    await setEntry(entry);
    await loadSettings();
    await loadHistory();
    closePanel('settings');
  } catch (e) {
    showError(e);
  } finally {
    $body.classList.remove('loading');
  }
}

function syncControls() {
  document.querySelectorAll('[data-style]').forEach((button) => {
    const active = button.dataset.style === settingsDraft.style;
    button.classList.toggle('active', active);
    button.setAttribute('aria-selected', active ? 'true' : 'false');
  });

  document.querySelectorAll('.choice-row').forEach((row) => {
    const key = row.dataset.setting;
    row.querySelectorAll('button').forEach((button) => {
      const active = button.dataset.value === settingsDraft[key];
      button.classList.toggle('active', active);
      button.setAttribute('aria-pressed', active ? 'true' : 'false');
    });
  });
}

function openPanel(name) {
  const isTip = name === 'tip';
  if (isTip) {
    $body.classList.add('tip-open');
    $tipPanel.setAttribute('aria-hidden', 'false');
    $settingsPanel.setAttribute('aria-hidden', 'true');
    $historyPanel.setAttribute('aria-hidden', 'true');
    return;
  }

  const isHistory = name === 'history';
  $body.classList.toggle('history-open', isHistory);
  $body.classList.toggle('settings-open', !isHistory);
  $body.classList.remove('tip-open');
  $historyPanel.setAttribute('aria-hidden', isHistory ? 'false' : 'true');
  $settingsPanel.setAttribute('aria-hidden', isHistory ? 'true' : 'false');
  $tipPanel.setAttribute('aria-hidden', 'true');
}

function closePanel(name) {
  if (!name || name === 'history') {
    $body.classList.remove('history-open');
    $historyPanel.setAttribute('aria-hidden', 'true');
  }
  if (!name || name === 'settings') {
    $body.classList.remove('settings-open');
    $settingsPanel.setAttribute('aria-hidden', 'true');
  }
  if (!name || name === 'tip') {
    $body.classList.remove('tip-open');
    $tipPanel.setAttribute('aria-hidden', 'true');
    if ($body.classList.contains('settings-open')) {
      $settingsPanel.setAttribute('aria-hidden', 'false');
    }
  }
}

async function loadHistory() {
  const entries = await call('get_history');
  $historyList.replaceChildren();

  if (!entries.length) {
    const empty = document.createElement('div');
    empty.className = 'history-empty';
    empty.textContent = '暂无';
    $historyList.append(empty);
    return;
  }

  for (const entry of entries) {
    const row = document.createElement('button');
    row.type = 'button';
    row.className = 'history-row';
    row.innerHTML = `<span>${escapeHtml(entry.text)}</span><small>${escapeHtml(
      [entry.date, STYLE_LABELS[entry.style] || ''].filter(Boolean).join(' · '),
    )}</small>`;
    row.addEventListener('click', () => {
      setEntry(entry);
      closePanel('history');
    });
    $historyList.append(row);
  }
}

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

async function downloadShareCard() {
  if (!currentEntry?.text) {
    showToast('暂无内容');
    return;
  }

  const now = Date.now();
  if (sharing || now < shareLockedUntil) {
    showToast(sharing ? '正在生成' : '已保存到下载');
    return;
  }

  sharing = true;
  setShareState('busy');
  showToast('正在生成');

  try {
    const canvas = drawShareCard(currentEntry);
    const blob = await new Promise((resolve) => canvas.toBlob(resolve, 'image/png', 0.96));
    if (!blob) throw new Error('share-card-empty');
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `sotto-${currentEntry.date || new Date().toISOString().slice(0, 10)}-${
      currentEntry.style || settingsDraft.style
    }.png`;
    document.body.append(link);
    link.click();
    link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1200);

    shareLockedUntil = Date.now() + SHARE_COOLDOWN_MS;
    setShareState('done');
    showToast('已保存到下载');
    shareStateTimer = window.setTimeout(() => setShareState('idle'), 1200);
  } catch (e) {
    setShareState('idle');
    showToast('下载失败');
  } finally {
    sharing = false;
  }
}

function drawShareCard(entry) {
  const canvas = document.createElement('canvas');
  canvas.width = 1080;
  canvas.height = 1440;
  const ctx = canvas.getContext('2d');

  ctx.fillStyle = '#f7f7f4';
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  ctx.fillStyle = 'rgba(21, 22, 24, 0.04)';
  for (let i = 0; i < 1400; i += 1) {
    const x = Math.random() * canvas.width;
    const y = Math.random() * canvas.height;
    ctx.fillRect(x, y, 1, 1);
  }

  ctx.fillStyle = '#17181a';
  ctx.textAlign = 'left';
  ctx.font = '22px "SF Mono", "JetBrains Mono", monospace';
  ctx.fillText('SOTTO', 96, 116);
  ctx.fillStyle = 'rgba(23, 24, 26, 0.42)';
  ctx.fillText('sotto voce', 96, 150);

  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  let fontSize = 50;
  let lines = [];
  do {
    ctx.font = `${fontSize}px "Songti SC", "Noto Serif SC", "STSong", serif`;
    lines = wrapText(ctx, entry.text, 640, 4);
    fontSize -= 2;
  } while (lines.length > 4 && fontSize >= 34);
  fontSize += 2;

  const lineHeight = Math.round(fontSize * 1.72);
  const centerY = 690;
  const startY = centerY - ((lines.length - 1) * lineHeight) / 2;
  ctx.fillStyle = '#111214';
  lines.forEach((line, index) => {
    ctx.fillText(line, canvas.width / 2, startY + index * lineHeight);
  });

  ctx.fillStyle = 'rgba(156, 38, 32, 0.82)';
  ctx.beginPath();
  ctx.arc(
    canvas.width / 2,
    startY + (lines.length - 1) * lineHeight + Math.round(lineHeight * 0.9),
    4.5,
    0,
    Math.PI * 2,
  );
  ctx.fill();

  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.font = '24px "SF Mono", "JetBrains Mono", monospace';
  ctx.fillStyle = 'rgba(23, 24, 26, 0.38)';
  ctx.fillText(entry.date || '', canvas.width / 2, 1244);
  ctx.font = '24px "Songti SC", "Noto Serif SC", "STSong", serif';
  ctx.fillStyle = 'rgba(23, 24, 26, 0.54)';
  ctx.fillText('安装 Sotto', canvas.width / 2, 1286);
  ctx.font = '20px "SF Mono", "JetBrains Mono", monospace';
  ctx.fillStyle = 'rgba(23, 24, 26, 0.4)';
  ctx.fillText('brew install --cask jokeuncle/sotto/sotto', canvas.width / 2, 1326);

  return canvas;
}

function wrapText(ctx, text, maxWidth, maxLines) {
  const chars = Array.from(text.replace(/\s+/g, ' ').trim());
  const leadingPunctuation = '，。！？；：、,.!?;:）)]】》”’';
  const weakLineStarts = '的了着过吗呢啊呀吧嘛么';
  const sentenceStops = '，。！？；：,.!?;:';

  if (ctx.measureText(chars.join('')).width <= maxWidth) return [chars.join('')];

  const widthCache = new Map();
  const widthOf = (from, to) => {
    const key = `${from}:${to}`;
    if (!widthCache.has(key)) {
      widthCache.set(key, ctx.measureText(chars.slice(from, to).join('')).width);
    }
    return widthCache.get(key);
  };

  let best = null;
  const minLines = Math.max(2, Math.ceil(widthOf(0, chars.length) / maxWidth));
  const lineCounts = Array.from(
    { length: Math.max(0, maxLines - minLines + 1) },
    (_, index) => minLines + index,
  );

  for (const lineCount of lineCounts) {
    const ideal = widthOf(0, chars.length) / lineCount;
    const search = (start, lineIndex, lines) => {
      if (lineIndex === lineCount - 1) {
        const width = widthOf(start, chars.length);
        if (width <= 0 || width > maxWidth) return;
        const nextLines = [...lines, chars.slice(start).join('')];
        scoreLines(nextLines, lineCount, ideal);
        return;
      }

      const remainingLines = lineCount - lineIndex - 1;
      const minEnd = start + 1;
      const maxEnd = chars.length - remainingLines;
      for (let end = minEnd; end <= maxEnd; end += 1) {
        if (leadingPunctuation.includes(chars[end])) continue;
        const width = widthOf(start, end);
        if (width > maxWidth) break;
        search(end, lineIndex + 1, [...lines, chars.slice(start, end).join('')]);
      }
    };

    const scoreLines = (candidate, lineCount, ideal) => {
      const widths = candidate.map((line) => ctx.measureText(line).width);
      const rag = widths.reduce((sum, width) => sum + (width - ideal) ** 2, 0);
      const shortLast = Math.max(0, ideal * 0.62 - widths[widths.length - 1]) ** 2;
      const complexity = (lineCount - minLines) * 1600;
      const grammar = candidate.reduce((sum, line, index) => {
        if (index === 0) return sum;
        return sum + (weakLineStarts.includes(line[0]) ? 52000 : 0);
      }, 0);
      const naturalStops = candidate.reduce((sum, line, index) => {
        if (index === candidate.length - 1) return sum;
        return sum + (sentenceStops.includes(line[line.length - 1]) ? -9000 : 0);
      }, 0);
      const score = rag + shortLast + complexity + grammar + naturalStops;
      if (!best || score < best.score) best = { lines: candidate, score };
    };

    search(0, 0, []);
  }

  return best?.lines || greedyWrapText(ctx, chars, maxWidth);
}

function greedyWrapText(ctx, chars, maxWidth) {
  const lines = [];
  let line = '';
  const leadingPunctuation = '，。！？；：、,.!?;:）)]】》”’';
  for (const char of chars) {
    const trial = line + char;
    if (ctx.measureText(trial).width <= maxWidth || !line) {
      line = trial;
      continue;
    }
    lines.push(line);
    if (leadingPunctuation.includes(char)) {
      lines[lines.length - 1] += char;
      line = '';
      continue;
    }
    line = char;
  }
  if (line) lines.push(line);
  return lines;
}

if (isTauri) {
  listen('aphorism', (e) => setEntry(e.payload));
  listen('aphorism-error', (e) => showError(e.payload));
}

$refresh.addEventListener('click', (e) => {
  e.stopPropagation();
  doRefresh();
});

$share.addEventListener('click', (e) => {
  e.stopPropagation();
  downloadShareCard();
});

$history.addEventListener('click', async (e) => {
  e.stopPropagation();
  await loadHistory();
  openPanel('history');
});

$historyClose.addEventListener('click', () => closePanel('history'));
$settings.addEventListener('click', () => openPanel('settings'));
$settingsClose.addEventListener('click', () => closePanel('settings'));
$settingsSave.addEventListener('click', saveSettings);
$tipOpen.addEventListener('click', () => openPanel('tip'));
$tipClose.addEventListener('click', () => closePanel('tip'));

$styles.addEventListener('click', (e) => {
  const button = e.target.closest('[data-style]');
  if (!button) return;
  changeStyle(button.dataset.style);
});

document.querySelectorAll('.choice-row').forEach((row) => {
  row.addEventListener('click', (e) => {
    const button = e.target.closest('[data-value]');
    if (!button) return;
    settingsDraft[row.dataset.setting] = button.dataset.value;
    syncControls();
  });
});

document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') closePanel();
});

init();
