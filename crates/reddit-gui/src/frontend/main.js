const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;

// Tab switching
document.querySelectorAll('.tab-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
    btn.classList.add('active');
    document.getElementById('tab-' + btn.dataset.tab).classList.add('active');
  });
});

// State
const state = {
  'process-submissions': [],
  'process-comments': [],
  'process-output': null,
  'filter-input': [],
};

async function pickFiles(key) {
  const files = await open({
    multiple: true,
    filters: [{ name: 'Data files', extensions: ['zst', 'jsonl'] }],
  });
  if (!files) return;
  const list = Array.isArray(files) ? files : [files];
  state[key] = list;
  document.getElementById(key + '-list').textContent = list.join('\n');
}

async function pickSaveFile(key) {
  const file = await save({
    filters: [{ name: 'Output', extensions: ['zst', 'jsonl'] }],
  });
  if (!file) return;
  state[key] = file;
  document.getElementById(key + '-display').textContent = file;
}

function setStatus(id, msg, kind) {
  const el = document.getElementById(id);
  el.textContent = msg;
  el.className = 'status ' + kind;
}

async function runProcess() {
  const submissions = state['process-submissions'];
  const comments = state['process-comments'];
  const output = state['process-output'];

  if (!submissions.length) return setStatus('process-status', 'Please select submission files.', 'err');
  if (!comments.length) return setStatus('process-status', 'Please select comment files.', 'err');
  if (!output) return setStatus('process-status', 'Please choose an output file.', 'err');

  setStatus('process-status', 'Running...', 'running');
  try {
    const result = await invoke('run_process', {
      args: {
        submissions,
        comments,
        output,
        includeScores: document.getElementById('process-include-scores').checked,
        compression: {
          level: parseInt(document.getElementById('process-level').value, 10),
          workers: parseInt(document.getElementById('process-workers').value, 10),
        },
      },
    });
    setStatus('process-status', result, 'ok');
  } catch (e) {
    setStatus('process-status', 'Error: ' + e, 'err');
  }
}

async function runFilter() {
  const input = state['filter-input'];
  const names = document.getElementById('filter-names').value
    .split('\n')
    .map(s => s.trim())
    .filter(Boolean);

  if (!input.length) return setStatus('filter-status', 'Please select input files.', 'err');
  if (!names.length) return setStatus('filter-status', 'Please enter at least one subreddit name.', 'err');

  const outputRaw = document.getElementById('filter-output').value.trim();

  setStatus('filter-status', 'Running...', 'running');
  try {
    const result = await invoke('run_filter', {
      args: {
        input,
        output: outputRaw || null,
        split: document.getElementById('filter-split').checked,
        name: names,
        compression: {
          level: parseInt(document.getElementById('filter-level').value, 10),
          workers: parseInt(document.getElementById('filter-workers').value, 10),
        },
      },
    });
    setStatus('filter-status', result, 'ok');
  } catch (e) {
    setStatus('filter-status', 'Error: ' + e, 'err');
  }
}
