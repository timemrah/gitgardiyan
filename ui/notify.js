const { invoke } = window.__TAURI__.core;
const appWindow = window.__TAURI__.webviewWindow.getCurrentWebviewWindow();

let payload = null;
let timerId = null;
let remaining = 0;
let acted = false;

function addBtn(text, cls, fn) {
  const b = document.createElement('button');
  b.textContent = text;
  if (cls) b.className = cls;
  b.addEventListener('click', fn);
  document.getElementById('buttons').appendChild(b);
}

function tick() {
  if (remaining <= 0) { act('proceed'); return; }
  const m = String(Math.floor(remaining / 60)).padStart(2, '0');
  const s = String(remaining % 60).padStart(2, '0');
  document.getElementById('timer').textContent = `${m}:${s}`;
  remaining--;
}

async function act(action) {
  if (acted) return;
  acted = true;
  clearInterval(timerId);
  document.getElementById('timer').textContent = '';
  document.getElementById('buttons').innerHTML = '';
  const result = document.getElementById('result');
  result.hidden = false;
  result.textContent = 'Çalışıyor…';
  let closeDelay = 6000;
  try {
    result.textContent = await invoke('notify_action', {
      path: payload.path,
      rule: payload.rule,
      action,
    });
    if (action === 'mute') closeDelay = 2500;
  } catch (e) {
    result.textContent = String(e);
    result.className = 'err';
    closeDelay = 15000;
  }
  setTimeout(() => appWindow.close(), closeDelay);
}

async function init() {
  payload = await invoke('get_notification', { label: appWindow.label });
  if (!payload) { appWindow.close(); return; }
  document.getElementById('msg').textContent = payload.message;

  if (payload.ptype === 'countdown') {
    remaining = payload.seconds;
    tick();
    timerId = setInterval(tick, 1000);
    addBtn('Şimdi yap', 'primary', () => act('proceed'));
    if (payload.rule === 1) addBtn('Bugün bir daha sorma', '', () => act('mute'));
    addBtn('İptal', '', () => appWindow.close());
  } else {
    addBtn('Çek', 'primary', () => act('pull'));
    addBtn('Boşver', '', () => appWindow.close());
  }
}

init();
