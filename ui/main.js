const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;

const errBox = document.getElementById('error');
const openPanels = new Set();

function clampInt(value, min, fallback) {
  const n = parseInt(value, 10);
  if (Number.isNaN(n)) return fallback;
  return Math.max(min, n);
}

function showError(e) {
  errBox.hidden = false;
  errBox.textContent = String(e);
  setTimeout(() => { errBox.hidden = true; }, 6000);
}

function statusText(v) {
  if (!v.found) return { text: 'Klasör bulunamadı ya da git deposu değil', warn: true };
  const parts = [];
  if (v.changed > 0) parts.push(`${v.changed} dosya değişti`);
  if (v.unpushed > 0) parts.push(`${v.unpushed} commit push bekliyor`);
  if (v.remote_ahead > 0) parts.push(`GitHub ${v.remote_ahead} commit ileride`);
  if (parts.length === 0) return { text: 'Temiz ✓', warn: false };
  return { text: parts.join(' · '), warn: true };
}

function render(views) {
  const list = document.getElementById('list');
  list.innerHTML = '';
  if (views.length === 0) {
    list.innerHTML = '<div class="empty">Henüz proje yok. "Proje Ekle" ile başla.</div>';
    return;
  }
  const tpl = document.getElementById('row');
  for (const v of views) {
    const p = v.project;
    const el = tpl.content.cloneNode(true);
    el.querySelector('.name').textContent = p.name;
    const st = statusText(v);
    const stEl = el.querySelector('.status');
    stEl.textContent = st.text;
    if (st.warn) stEl.classList.add('warn');
    el.querySelector('.path').textContent = p.path;
    el.querySelector('.threshold').value = p.threshold;
    el.querySelector('.interval-changes').value = p.interval_changes_minutes;
    el.querySelector('.interval-remote').value = p.interval_remote_minutes;
    el.querySelector('.backup').value = p.backup_time;
    el.querySelector('.r1').checked = p.rule_changes;
    el.querySelector('.r2').checked = p.rule_remote;
    el.querySelector('.r3').checked = p.rule_backup;

    const root = el.querySelector('.project');

    // Kural kapalıyken inputları soluklaştır ve kilitle.
    function syncRules() {
      for (const rule of root.querySelectorAll('.rule')) {
        const on = rule.querySelector('input[type="checkbox"]').checked;
        rule.classList.toggle('off', !on);
        for (const input of rule.querySelectorAll('.param input')) input.disabled = !on;
      }
    }
    for (const cb of root.querySelectorAll('.rule input[type="checkbox"]')) {
      cb.addEventListener('change', syncRules);
    }
    syncRules();

    // Panel durumu yenilemeler arasında korunur (kaydet sonrası kapanmasın).
    const settingsEl = root.querySelector('.settings');
    const toggleBtn = root.querySelector('.toggle-settings');
    settingsEl.hidden = !openPanels.has(p.path);
    toggleBtn.classList.toggle('active', !settingsEl.hidden);
    toggleBtn.addEventListener('click', () => {
      settingsEl.hidden = !settingsEl.hidden;
      toggleBtn.classList.toggle('active', !settingsEl.hidden);
      if (settingsEl.hidden) openPanels.delete(p.path);
      else openPanels.add(p.path);
    });

    el.querySelector('.remove').addEventListener('click', async () => {
      try { await invoke('remove_project', { path: p.path }); refresh(); }
      catch (e) { showError(e); }
    });
    el.querySelector('.save').addEventListener('click', async () => {
      const updated = {
        ...p,
        threshold: clampInt(root.querySelector('.threshold').value, 1, 10),
        interval_changes_minutes: clampInt(root.querySelector('.interval-changes').value, 5, 60),
        interval_remote_minutes: clampInt(root.querySelector('.interval-remote').value, 5, 60),
        backup_time: root.querySelector('.backup').value || '23:00',
        rule_changes: root.querySelector('.r1').checked,
        rule_remote: root.querySelector('.r2').checked,
        rule_backup: root.querySelector('.r3').checked,
      };
      try { await invoke('update_project', { project: updated }); refresh(); }
      catch (e) { showError(e); }
    });
    list.appendChild(el);
  }
}

async function refresh() {
  try { render(await invoke('list_projects')); }
  catch (e) { showError(e); }
}

document.getElementById('add').addEventListener('click', async () => {
  try {
    const dir = await open({ directory: true, title: 'Proje klasörü seç' });
    if (!dir) return;
    await invoke('add_project', { path: dir });
    refresh();
  } catch (e) { showError(e); }
});

document.getElementById('refresh').addEventListener('click', refresh);

const autoEl = document.getElementById('autostart');
autoEl.addEventListener('change', async () => {
  try { await invoke('set_autostart', { enabled: autoEl.checked }); }
  catch (e) { showError(e); }
});

(async () => {
  try { autoEl.checked = await invoke('get_autostart'); } catch (_) {}
  refresh();
})();
