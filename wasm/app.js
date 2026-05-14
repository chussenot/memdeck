/* ── Embedded stack data ──────────────────────────────────────────────────── */
const STACKS_TSV = {
  Aronson: `# Aronson Stack - Simon Aronson
# position\tcard\tmnemonic
1\tJS
2\tKC
3\t5C
4\t2H
5\t9S
6\tAS
7\t3H
8\t6C
9\t8D
10\tAC
11\t10S
12\t5H
13\t2D
14\tKD
15\t7D
16\t8C
17\t3S
18\tAD
19\t7S
20\t5S
21\tQD
22\tAH
23\t8S
24\t3D
25\t7H
26\tQH
27\t5D
28\t7C
29\t4D
30\tKH
31\t4H
32\tJD
33\t9C
34\tJH
35\tQS
36\t10D
37\t4C
38\tKS
39\tJC
40\t9H
41\t10C
42\t2S
43\t6H
44\t3C
45\tQC
46\t8H
47\t6D
48\t6S
49\t4S
50\t9D
51\t2C
52\t10H`,

  Mnemonica: `# Mnemonica Stack - Juan Tamariz
# position\tcard\tmnemonic
1\t4C
2\t2H
3\t7D
4\t3C
5\t4H
6\t6D
7\tAS
8\t5H
9\t9S
10\t2S
11\tQH
12\t3D
13\tQC
14\t8H
15\t6S
16\t5S
17\t9H
18\tKC
19\t2D
20\tJH
21\t3S
22\t8S
23\t6H
24\t10C
25\t5D
26\tJS
27\t3H
28\tJC
29\t7S
30\t10H
31\tQD
32\tAD
33\t5C
34\tJD
35\t7H
36\t10D
37\tKD
38\t7C
39\t8D
40\t4S
41\tKH
42\tAH
43\t9C
44\t2C
45\tQS
46\t6C
47\t4D
48\t9D
49\tAC
50\t10S
51\t8C
52\tKS`,

  Memorandum: `# Memorandum Stack - Woody Aragon
# position\tcard\tmnemonic
1\tJS
2\t7C
3\t10H
4\tAD
5\t4C
6\t7H
7\t4D
8\tAS
9\t4H
10\t7D
11\t4S
12\tAH
13\t10D
14\t7S
15\tJC
16\tKD
17\t10S
18\t8C
19\tJH
20\tAC
21\tKS
22\t5C
23\t8H
24\t3D
25\tQS
26\tKH
27\t2S
28\t5H
29\t8S
30\t3C
31\t6D
32\t9S
33\t2D
34\tQH
35\t5S
36\t9C
37\t6H
38\t3S
39\t2C
40\t8D
41\t5D
42\t9H
43\t6S
44\tQC
45\t2H
46\t3H
47\t6C
48\t9D
49\tJD
50\t10C
51\tQD
52\tKC`,
};

/* ── Learn content ─────────────────────────────────────────────────────────── */
const LEARN_CONTENT = `
# The Memorized Deck

## What is a Memorized Deck?

A memorized deck is a deck of cards in a specific, known order that a
magician has committed to memory. Knowing the exact position of every
card allows for incredibly powerful effects that seem impossible.

## Why Memorize a Deck?

With a memorized deck, you can:
- Name any card at any position
- Name the position of any card
- Know what card comes before or after any other card
- Perform miracles that require no sleight of hand
- Combine stack knowledge with other techniques

## Popular Stacks

### Aronson Stack
Created by Simon Aronson, this is one of the most popular memorized
stacks in use today. Published in his book "A Stack to Remember" and
later in "Bound to Please."

### Mnemonica
Created by Juan Tamariz, the Mnemonica stack is detailed in his book
of the same name. It has the remarkable property of being reachable
from new deck order through a series of shuffles.

### Memorandum
Created by Woody Aragon and detailed in his book "A Year in the Works."
A versatile stack that has many built-in properties.

## How to Memorize

### Method 1: Brute Force Repetition
Simply go through the stack repeatedly. Use the Study mode to browse
through the cards, then Practice mode to test yourself. Start with
positions 1-10, then add more as you master each group.

### Method 2: PAO (Person-Action-Object)
Assign a Person, Action, and Object to each card. Then create a
mental story linking the PAO to the position number.

### Method 3: Memory Palace (Method of Loci)
Place each card at a specific location in a familiar place (your
home, a route you walk). Walk through the palace to recall the
sequence.

### Method 4: Chunking
Break the deck into groups of 4-5 cards. Memorize each chunk as
a unit before moving on to the next. This reduces cognitive load
by focusing on smaller, manageable pieces.

## Practice Strategy

1. Start slow — accuracy over speed
2. Practice daily — short sessions beat marathon sessions
3. Use the streak feature to build consistency
4. Focus on your hardest positions (check the Progress screen)
5. Mix question types once you know positions well

---

Good luck! A memorized deck is one of magic's most powerful tools.
`;

/* ── Constants ─────────────────────────────────────────────────────────────── */
const Q_TYPE = {
  POS_TO_CARD: 0, CARD_TO_POS: 1, NEXT_CARD: 2,
  PREV_CARD: 3, SUIT_DRILL: 4, VALUE_DRILL: 5, MIXED: 6,
};
const Q_NAMES = [
  'Position → Card', 'Card → Position', 'Next Card',
  'Previous Card', 'Suit Drill', 'Value Drill', 'Mixed',
];
const Q_DESC = [
  'Given a position, name the card',
  'Given a card, name its position',
  'Name the card after a given card',
  'Name the card before a given card',
  'Name the suit at a position',
  'Name the value at a position',
  'Random mix of all question types',
];
const FILTER_NAMES = [
  'All', 'Black', 'Red', 'Hearts', 'Spades', 'Clubs', 'Diamonds', 'Face', 'Numbers',
];
const LIMIT_NAMES = ['None', 'Time', 'Questions', 'Lives'];
const LIMIT_DESC  = ['—', 'seconds', 'questions', 'lives'];

/* ── WASM module handle ────────────────────────────────────────────────────── */
let M; // MemDeck WASM module

function cstr(ptr) { return M.UTF8ToString(ptr); }

const md = {
  init()              { M._md_init(); },
  loadStack(n, tsv)   { return M.ccall('md_load_stack','number',['string','string'],[n,tsv]); },
  stackCount()        { return M._md_stack_count(); },
  stackName(i)        { return cstr(M._md_stack_name(i)); },
  stackSize(i)        { return M._md_stack_size(i); },
  currentStack()      { return M._md_get_current_stack(); },
  setStack(i)         { M._md_set_current_stack(i); },
  getEntry(si,ei)     { return JSON.parse(cstr(M._md_get_entry(si,ei))); },
  setSettings(qt,nc,rmin,rmax,lm,lv,cf,sm) {
    M._md_set_settings(qt,nc,rmin,rmax,lm,lv,cf,sm);
  },
  getSettings()       { return JSON.parse(cstr(M._md_get_settings())); },
  sessionStart()      { M._md_session_start(); },
  sessionQuestion()   { return JSON.parse(cstr(M._md_session_question())); },
  sessionAnswer(ci)   { M._md_session_answer(ci); },
  sessionNext()       { M._md_session_next(); },
  sessionIsOver()     { return M._md_session_is_over() !== 0; },
  sessionStats()      { return JSON.parse(cstr(M._md_session_stats())); },
  sessionComplete()   { M._md_session_complete(); },
  progressDump()      { return JSON.parse(cstr(M._md_progress_dump())); },
  progressLoad(json)  { M.ccall('md_progress_load','null',['string'],[json]); },
  progressReset()     { M._md_progress_reset(); },
  validateStack(i)    { return cstr(M._md_validate_stack(i)); },
};

/* ── Audio (Web Audio API chiptune sounds) ─────────────────────────────────── */
let audioCtx = null;

function ensureAudio() {
  if (!audioCtx) audioCtx = new (window.AudioContext || window.webkitAudioContext)();
  if (audioCtx.state === 'suspended') audioCtx.resume();
  return audioCtx;
}

function playSuccess() {
  try {
    const ctx = ensureAudio();
    [[523.25, 0], [659.25, 0.08], [783.99, 0.16]].forEach(([f, t]) => {
      const osc = ctx.createOscillator(), g = ctx.createGain();
      osc.connect(g); g.connect(ctx.destination);
      osc.type = 'square'; osc.frequency.value = f;
      const s = ctx.currentTime + t;
      g.gain.setValueAtTime(0, s);
      g.gain.linearRampToValueAtTime(0.08, s + 0.01);
      g.gain.exponentialRampToValueAtTime(0.001, s + 0.18);
      osc.start(s); osc.stop(s + 0.18);
    });
  } catch(e) { /* audio not critical */ }
}

function playFail() {
  try {
    const ctx = ensureAudio();
    const osc = ctx.createOscillator(), g = ctx.createGain();
    osc.connect(g); g.connect(ctx.destination);
    osc.type = 'sawtooth';
    osc.frequency.setValueAtTime(280, ctx.currentTime);
    osc.frequency.exponentialRampToValueAtTime(90, ctx.currentTime + 0.35);
    g.gain.setValueAtTime(0.09, ctx.currentTime);
    g.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.35);
    osc.start(ctx.currentTime); osc.stop(ctx.currentTime + 0.35);
  } catch(e) { /* audio not critical */ }
}

/* ── Toast ─────────────────────────────────────────────────────────────────── */
let toastTimer;
function toast(msg) {
  const el = document.getElementById('toast');
  el.textContent = msg;
  el.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.remove('show'), 2500);
}

/* ── Progress persistence ──────────────────────────────────────────────────── */
const PROGRESS_KEY = 'memdeck_progress';

function saveProgress() {
  localStorage.setItem(PROGRESS_KEY, JSON.stringify(md.progressDump()));
}

function loadProgress() {
  const raw = localStorage.getItem(PROGRESS_KEY);
  if (raw) md.progressLoad(raw);
}

/* ── Screen routing ────────────────────────────────────────────────────────── */
const screenEl = document.getElementById('screen');

function render(html) {
  screenEl.innerHTML = html;
  screenEl.classList.remove('hidden');
}

/* ── Logo animation ────────────────────────────────────────────────────────── */
const LOGO_LINES = [
  '  __  __                ____            _    ',
  ' |  \\/  | ___ _ __ ___|  _ \\  ___  ___| | __',
  ' | |\\/| |/ _ \\ \'_ ` _ \\ | | |/ _ \\/ __| |/ /',
  ' | |  | |  __/ | | | | | |_| |  __/ (__|   < ',
  ' |_|  |_|\\___|_| |_| |_|____/ \\___|\\___|_|\\_\\',
];

function buildLogo() {
  return LOGO_LINES.map((line, row) => {
    const chars = Array.from(line).map((ch, col) => {
      if (ch === ' ') return '<span> </span>';
      const delay = ((col + row * 3) % 6) * 0.3;
      return `<span class="logo-char" style="animation-delay:${delay}s">${escHtml(ch)}</span>`;
    }).join('');
    return `<span class="logo-line">${chars}</span>`;
  }).join('');
}

/* ── HTML helpers ──────────────────────────────────────────────────────────── */
function escHtml(s) {
  return String(s)
    .replace(/&/g,'&amp;').replace(/</g,'&lt;')
    .replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

function cardHtml(entry) {
  const isRed = entry.suit === 1 || entry.suit === 3; // hearts or diamonds
  const cls   = isRed ? 'red' : '';
  return `<div class="card-art ${cls}">
    <div class="ca-rank-tl">${escHtml(entry.rank_name[0] === 'T' ? '10' : entry.rank_name[0])}</div>
    <div class="ca-suit-tl">${entry.suit_symbol}</div>
    <div class="ca-center">${entry.suit_symbol}</div>
    <div class="ca-rank-br">${escHtml(entry.rank_name[0] === 'T' ? '10' : entry.rank_name[0])}</div>
  </div>`;
}

function suitClass(suit) {
  return ['spades','hearts','clubs','diamonds'][suit] || '';
}

/* ── Screen: Menu ──────────────────────────────────────────────────────────── */
function showMenu() {
  const items = [
    {icon:'🃏', label:'Play',     key:'play'},
    {icon:'📖', label:'Study',    key:'study'},
    {icon:'🗂', label:'Stacks',   key:'stacks'},
    {icon:'📊', label:'Progress', key:'progress'},
    {icon:'⚙️', label:'Settings', key:'settings'},
    {icon:'📚', label:'Learn',    key:'learn'},
  ];

  render(`
    <div class="content">
      <div class="logo-wrap">${buildLogo()}</div>
      <p class="subtitle">Memorized Deck Trainer</p>
      <nav class="menu-list">
        ${items.map(it => `
          <button class="menu-btn" onclick="handleMenu('${it.key}')">
            <span class="icon">${it.icon}</span>${escHtml(it.label)}
          </button>`).join('')}
      </nav>
    </div>
    <div id="toast"></div>
  `);
}

function handleMenu(key) {
  ensureAudio(); // unlock audio on first interaction
  switch(key) {
    case 'play':     showPlayMenu(); break;
    case 'study':    showStudy(0);   break;
    case 'stacks':   showStacks();   break;
    case 'progress': showProgress(); break;
    case 'settings': showSettings(); break;
    case 'learn':    showLearn();    break;
  }
}

/* ── Screen: Play (mode selection) ────────────────────────────────────────── */
function showPlayMenu() {
  const modes = Q_NAMES.map((name, i) => `
    <button class="mode-btn" onclick="startPractice(${i})">
      <div class="mode-title">${escHtml(name)}</div>
      <div class="mode-desc">${escHtml(Q_DESC[i])}</div>
    </button>`).join('');

  render(`
    <div class="title-bar">
      <span>PRACTICE MODE</span>
      <button class="back-btn" onclick="showMenu()">← Back</button>
    </div>
    <div class="content">
      <div class="mode-grid">${modes}</div>
    </div>
    <div id="toast"></div>
  `);
}

/* ── Screen: Practice ──────────────────────────────────────────────────────── */
function startPractice(qtype) {
  const s = md.getSettings();
  md.setSettings(qtype, s.num_choices, s.range_min, s.range_max,
                 s.limit_mode, s.limit_value, s.card_filter, s.show_mnemonic);
  md.sessionStart();
  if (md.sessionIsOver()) {
    toast('No cards match those settings. Check your range/filter.');
    return;
  }
  renderPractice();
}

function renderPractice() {
  if (md.sessionIsOver()) {
    showComplete();
    return;
  }
  const q = md.sessionQuestion();
  if (!q.active) { showComplete(); return; }

  const lim = (function() {
    const s = md.getSettings();
    if (s.limit_mode === 1) return `⏱ ${s.limit_value}s`;
    if (s.limit_mode === 2) return `Q ${q.questions_asked+1}/${s.limit_value}`;
    if (s.limit_mode === 3) return `❤ ${q.lives}`;
    return '';
  })();

  render(`
    <div class="title-bar">
      <span>PRACTICE${lim ? ' — ' + lim : ''}</span>
      <button class="back-btn" onclick="confirmQuit()">✕ Quit</button>
    </div>
    <div class="content">
      <div class="practice-wrap">
        <div class="stat-bar">
          <span class="stat-pill correct">✓ <span class="val">${q.correct}</span></span>
          <span class="stat-pill wrong">✗ <span class="val">${q.incorrect}</span></span>
          <span class="stat-pill">🔥 <span class="val">${q.streak}</span></span>
        </div>
        <div class="question-box">${escHtml(q.text)}</div>
        <div class="choices-grid" id="choices">
          ${buildChoices(q)}
        </div>
        <div id="feedback"></div>
        <div class="action-row" id="actions"></div>
      </div>
    </div>
    <div id="toast"></div>
  `);
}

function buildChoices(q) {
  return q.choices.map((choice, i) => {
    const label = choiceLabel(choice, q.display_type);
    return `<button class="choice-btn" id="choice-${i}" onclick="pickAnswer(${i})">${label}</button>`;
  }).join('');
}

function choiceLabel(choice, displayType) {
  switch (displayType) {
    case 0: { // card
      const cls = suitClass(choice.suit);
      return `<span class="card-display ${cls}">${escHtml(choice.display)}</span>
              <span class="choice-sub">${escHtml(choice.rank_name)} of ${escHtml(choice.suit_name)}</span>`;
    }
    case 1: // position
      return `<span class="card-display">${escHtml(String(choice.pos))}</span>`;
    case 2: { // suit
      const cls = suitClass(choice.suit);
      return `<span class="card-display ${cls}">${escHtml(choice.suit_symbol)}</span>
              <span class="choice-sub">${escHtml(choice.suit_name)}</span>`;
    }
    case 3: // value
      return `<span class="card-display">${escHtml(choice.rank_name)}</span>`;
    default:
      return escHtml(choice.display);
  }
}

function pickAnswer(choiceIdx) {
  md.sessionAnswer(choiceIdx);
  const q = md.sessionQuestion();

  // Highlight choices
  for (let i = 0; i < q.num_choices; i++) {
    const btn = document.getElementById(`choice-${i}`);
    if (!btn) continue;
    btn.disabled = true;
    if (i === q.correct_slot)  btn.classList.add('correct-choice');
    if (i === choiceIdx && !q.last_correct) btn.classList.add('wrong-choice');
  }

  // Feedback + sound
  const fb = document.getElementById('feedback');
  if (q.last_correct) {
    playSuccess();
    fb.innerHTML = '<div class="feedback-bar correct">✓ Correct!</div>';
  } else {
    playFail();
    const correct = q.choices[q.correct_slot];
    fb.innerHTML = `<div class="feedback-bar wrong">✗ Wrong — ${escHtml(correct.display)} at #${correct.pos}</div>`;
  }

  saveProgress();

  const acts = document.getElementById('actions');
  if (md.sessionIsOver()) {
    acts.innerHTML = `<button class="btn primary" onclick="showComplete()">See Results</button>`;
  } else {
    acts.innerHTML = `<button class="btn primary" onclick="nextQuestion()">Next →</button>`;
  }
}

function nextQuestion() {
  md.sessionNext();
  renderPractice();
}

function confirmQuit() {
  if (confirm('Quit this session?')) { md.sessionComplete(); saveProgress(); showMenu(); }
}

/* ── Screen: Complete ──────────────────────────────────────────────────────── */
function showComplete() {
  md.sessionComplete();
  saveProgress();
  const s = md.sessionStats();
  const p = md.progressDump();
  const pct = s.questions_asked > 0
    ? Math.round(100 * s.correct / s.questions_asked) : 0;
  const mm = Math.floor(s.elapsed_sec / 60);
  const ss_ = s.elapsed_sec % 60;
  const timeStr = `${mm}:${String(ss_).padStart(2,'0')}`;
  const icon = pct >= 80 ? '🏆' : pct >= 50 ? '👍' : '💪';

  render(`
    <div class="title-bar"><span>SESSION COMPLETE</span></div>
    <div class="content">
      <div class="complete-icon">${icon}</div>
      <table class="stat-table">
        <tr><td>Correct</td><td>${s.correct}</td></tr>
        <tr><td>Incorrect</td><td>${s.incorrect}</td></tr>
        <tr><td>Accuracy</td><td>${pct}%</td></tr>
        <tr><td>Best streak</td><td>${s.best_streak}</td></tr>
        <tr><td>Time</td><td>${timeStr}</td></tr>
        <tr><td>Total sessions</td><td>${p.total_sessions}</td></tr>
      </table>
      <div class="action-row" style="margin-top:16px">
        <button class="btn primary" onclick="showPlayMenu()">Play Again</button>
        <button class="btn" onclick="showMenu()">Menu</button>
      </div>
    </div>
    <div id="toast"></div>
  `);
}

/* ── Screen: Settings ──────────────────────────────────────────────────────── */
function showSettings() {
  const s = md.getSettings();
  renderSettings(s);
}

function renderSettings(s) {
  function seg(key, vals, labels, current) {
    return vals.map((v, i) =>
      `<button class="seg-btn${current === v ? ' active' : ''}"
         onclick="updateSetting('${key}', ${v})">${escHtml(labels[i])}</button>`
    ).join('');
  }

  render(`
    <div class="title-bar">
      <span>SETTINGS</span>
      <button class="back-btn" onclick="showMenu()">← Back</button>
    </div>
    <div class="content">
      <div class="settings-form">

        <div class="setting-row">
          <span class="setting-label">Question type</span>
          <div class="setting-control">
            ${seg('question_type',
              [0,1,2,3,4,5,6],
              ['P→C','C→P','Next','Prev','Suit','Val','Mix'],
              s.question_type)}
          </div>
        </div>

        <div class="setting-row">
          <span class="setting-label">Choices</span>
          <div class="setting-control">
            ${seg('num_choices', [2,3,4,5,6], ['2','3','4','5','6'], s.num_choices)}
          </div>
        </div>

        <div class="setting-row">
          <span class="setting-label">Card filter</span>
          <div class="setting-control" style="flex-wrap:wrap; gap:4px;">
            ${seg('card_filter',
              [0,1,2,3,4,5,6,7,8],
              ['All','Black','Red','♥','♠','♣','♦','Face','Num'],
              s.card_filter)}
          </div>
        </div>

        <div class="setting-row">
          <span class="setting-label">Range min</span>
          <div class="setting-control">
            ${seg('range_min',
              [1,5,10,14,27,40],
              ['1','5','10','14','27','40'],
              [1,5,10,14,27,40].includes(s.range_min) ? s.range_min : 1)}
            <span style="font-size:0.78rem;color:var(--text-dim)"> (${s.range_min})</span>
          </div>
        </div>

        <div class="setting-row">
          <span class="setting-label">Range max</span>
          <div class="setting-control">
            ${seg('range_max',
              [13,26,39,52],
              ['13','26','39','52'],
              [13,26,39,52].includes(s.range_max) ? s.range_max : 52)}
            <span style="font-size:0.78rem;color:var(--text-dim)"> (${s.range_max})</span>
          </div>
        </div>

        <div class="setting-row">
          <span class="setting-label">Limit mode</span>
          <div class="setting-control">
            ${seg('limit_mode', [0,1,2,3], ['None','Time','Qs','Lives'], s.limit_mode)}
          </div>
        </div>

        ${s.limit_mode !== 0 ? `
        <div class="setting-row">
          <span class="setting-label">Limit value (${LIMIT_DESC[s.limit_mode]})</span>
          <div class="setting-control">
            ${seg('limit_value',
              s.limit_mode === 1 ? [30,60,120,300] : s.limit_mode === 2 ? [5,10,20,50] : [1,3,5],
              s.limit_mode === 1 ? ['30s','1m','2m','5m'] : s.limit_mode === 2 ? ['5','10','20','50'] : ['1','3','5'],
              s.limit_value)}
          </div>
        </div>` : ''}

        <div class="setting-row">
          <span class="setting-label">Show mnemonic after answer</span>
          <div class="setting-control">
            ${seg('show_mnemonic', [0,1], ['Off','On'], s.show_mnemonic)}
          </div>
        </div>
      </div>
    </div>
    <div id="toast"></div>
  `);
}

function updateSetting(key, value) {
  const s = md.getSettings();
  s[key] = value;
  md.setSettings(s.question_type, s.num_choices, s.range_min, s.range_max,
                 s.limit_mode, s.limit_value, s.card_filter, s.show_mnemonic);
  renderSettings(md.getSettings());
}

/* ── Screen: Study ─────────────────────────────────────────────────────────── */
let studyShowMnemonic = false;

function showStudy(entryIdx) {
  const si   = md.currentStack();
  const size = md.stackSize(si);
  if (size === 0) { toast('No stack loaded.'); return; }
  const idx  = Math.max(0, Math.min(entryIdx, size - 1));
  const e    = md.getEntry(si, idx);
  studyShowMnemonic = false;
  renderStudy(idx, e, si, size);
}

function renderStudy(idx, e, si, size) {
  render(`
    <div class="title-bar">
      <span>STUDY — ${escHtml(md.stackName(si))}</span>
      <button class="back-btn" onclick="showMenu()">← Back</button>
    </div>
    <div class="content">
      <div class="study-wrap">
        <div>
          <div class="pos-label">Position</div>
          <div class="pos-display">${e.pos}</div>
        </div>
        ${cardHtml(e)}
        <div style="text-align:center; font-size:1.2rem; color:var(--text)">
          ${escHtml(e.rank_name)} of ${escHtml(e.suit_name)}
        </div>
        <div class="mnemonic-box${e.mnemonic && !studyShowMnemonic ? ' hidden-text' : ''}"
             id="mnemonic-box"
             onclick="toggleMnemonic(${idx}, ${si}, ${size})">
          ${e.mnemonic
            ? (studyShowMnemonic ? escHtml(e.mnemonic) : '[ click to reveal mnemonic ]')
            : '<em style="color:var(--text-dim)">No mnemonic</em>'}
        </div>
        <div class="study-nav">
          <button class="btn" onclick="showStudy(${idx - 1})" ${idx === 0 ? 'disabled' : ''}>← Prev</button>
          <input class="study-pos-input" id="study-goto" type="number"
                 min="0" max="${size - 1}" value="${idx}"
                 onchange="showStudy(parseInt(this.value))"
                 title="Entry index (0-based)" />
          <button class="btn" onclick="showStudy(${idx + 1})" ${idx === size - 1 ? 'disabled' : ''}>Next →</button>
        </div>
        <div style="font-size:0.78rem; color:var(--text-dim)">${idx + 1} / ${size}</div>
      </div>
    </div>
    <div id="toast"></div>
  `);
}

function toggleMnemonic(idx, si, size) {
  studyShowMnemonic = !studyShowMnemonic;
  const e = md.getEntry(si, idx);
  renderStudy(idx, e, si, size);
}

/* ── Screen: Stacks ────────────────────────────────────────────────────────── */
function showStacks() {
  const count   = md.stackCount();
  const current = md.currentStack();

  const cards = Array.from({length: count}, (_, i) => {
    const name   = md.stackName(i);
    const size   = md.stackSize(i);
    const active = i === current;
    return `
      <div class="stack-card${active ? ' active' : ''}">
        <div>
          <div class="stack-name">${escHtml(name)}</div>
          <div class="stack-meta">${size} cards${active ? ' — active' : ''}</div>
        </div>
        <div class="stack-actions">
          <button class="sm-btn" onclick="validateStack(${i})">Validate</button>
          ${!active
            ? `<button class="sm-btn" onclick="selectStack(${i})">Select</button>`
            : `<button class="sm-btn active-btn" disabled>✓ Active</button>`}
        </div>
      </div>`;
  }).join('');

  render(`
    <div class="title-bar">
      <span>STACKS</span>
      <button class="back-btn" onclick="showMenu()">← Back</button>
    </div>
    <div class="content">
      <div class="stack-list">${cards || '<p style="color:var(--text-dim)">No stacks loaded.</p>'}</div>
    </div>
    <div id="toast"></div>
  `);
}

function selectStack(i) {
  md.setStack(i);
  showStacks();
  toast(`Switched to ${md.stackName(i)}`);
}

function validateStack(i) {
  const err = md.validateStack(i);
  toast(err ? `✗ ${err}` : `✓ ${md.stackName(i)} is valid`);
}

/* ── Screen: Progress ──────────────────────────────────────────────────────── */
function showProgress() {
  const p = md.progressDump();
  const total = p.total_correct + p.total_incorrect;
  const acc = total > 0 ? Math.round(100 * p.total_correct / total) : 0;

  const si   = md.currentStack();
  const size = md.stackSize(si);

  const heatCells = Array.from({length: size}, (_, i) => {
    const errs = p.card_errors[i]  || 0;
    const cors = p.card_correct[i] || 0;
    const tot  = errs + cors;
    let cls = '';
    if (tot > 0) {
      const rate = errs / tot;
      if      (rate === 0)   cls = 'err-0';
      else if (rate < 0.15)  cls = 'err-1';
      else if (rate < 0.35)  cls = 'err-2';
      else if (rate < 0.6)   cls = 'err-3';
      else                   cls = 'err-4';
    }
    const e = (size === 52) ? md.getEntry(si, i) : null;
    const tip = e ? `${e.code} pos ${e.pos}: ${cors}✓ ${errs}✗` : `idx ${i}: ${cors}✓ ${errs}✗`;
    return `<div class="hm-cell ${cls}" title="${escHtml(tip)}"></div>`;
  }).join('');

  render(`
    <div class="title-bar">
      <span>PROGRESS</span>
      <button class="back-btn" onclick="showMenu()">← Back</button>
    </div>
    <div class="content">
      <div class="progress-grid">
        <div class="prog-tile"><div class="pt-value">${p.total_sessions}</div><div class="pt-label">Sessions</div></div>
        <div class="prog-tile"><div class="pt-value">${acc}%</div><div class="pt-label">Accuracy</div></div>
        <div class="prog-tile"><div class="pt-value">${p.current_streak}</div><div class="pt-label">Streak</div></div>
        <div class="prog-tile"><div class="pt-value">${p.best_streak}</div><div class="pt-label">Best streak</div></div>
        <div class="prog-tile"><div class="pt-value">${p.total_correct}</div><div class="pt-label">Correct</div></div>
        <div class="prog-tile"><div class="pt-value">${p.total_incorrect}</div><div class="pt-label">Incorrect</div></div>
      </div>
      <span class="heatmap-label">Per-card accuracy (${md.stackName(si)})</span>
      <div class="heatmap">${heatCells}</div>
      <div style="margin-top:8px; font-size:0.72rem; color:var(--text-dim); text-align:center">
        🟢 good &nbsp; 🟡 fair &nbsp; 🔴 needs work &nbsp; ⬛ not yet seen
      </div>
      <div class="action-row" style="margin-top:24px">
        <button class="btn" onclick="resetProgressConfirm()" style="color:var(--red); border-color:var(--red)">
          Reset Progress
        </button>
      </div>
    </div>
    <div id="toast"></div>
  `);
}

function resetProgressConfirm() {
  if (confirm('Reset all progress? This cannot be undone.')) {
    md.progressReset();
    localStorage.removeItem(PROGRESS_KEY);
    showProgress();
    toast('Progress reset.');
  }
}

/* ── Screen: Learn ─────────────────────────────────────────────────────────── */
function showLearn() {
  render(`
    <div class="title-bar">
      <span>LEARN THE METHOD</span>
      <button class="back-btn" onclick="showMenu()">← Back</button>
    </div>
    <div class="content">
      <div class="learn-content">${markdownToHtml(LEARN_CONTENT)}</div>
    </div>
    <div id="toast"></div>
  `);
}

/** Very minimal Markdown→HTML (only what we need for LEARN_CONTENT) */
function markdownToHtml(md_) {
  return md_
    .split('\n')
    .map(line => {
      if (line.startsWith('### ')) return `<h3>${escHtml(line.slice(4))}</h3>`;
      if (line.startsWith('## '))  return `<h2>${escHtml(line.slice(3))}</h2>`;
      if (line.startsWith('# '))   return `<h1>${escHtml(line.slice(2))}</h1>`;
      if (line.startsWith('---'))  return '<hr>';
      if (line.startsWith('- '))   return `<li>${escHtml(line.slice(2))}</li>`;
      if (line.trim() === '')       return '';
      return `<p>${escHtml(line)}</p>`;
    })
    .join('\n')
    .replace(/(<li>.*<\/li>\n?)+/g, m => `<ul>${m}</ul>`);
}

/* ── Bootstrap ─────────────────────────────────────────────────────────────── */
function initApp() {
  md.init();

  // Load built-in stacks
  for (const [name, tsv] of Object.entries(STACKS_TSV)) {
    md.loadStack(name, tsv);
  }

  // Restore saved progress
  loadProgress();

  // Hide loading, show menu
  document.getElementById('loading').classList.add('hidden');
  showMenu();
}

/* Wait for MemDeckModule (Emscripten WASM) to be ready */
if (typeof MemDeckModule !== 'undefined') {
  MemDeckModule().then(mod => { M = mod; initApp(); });
} else {
  /* Development fallback: show helpful message if WASM isn't built yet */
  document.getElementById('loading').innerHTML = `
    <div style="text-align:center; padding:40px; max-width:500px">
      <div class="loading-logo">MemDeck</div>
      <p style="margin-top:24px; color:var(--text-muted)">
        WebAssembly module not found.<br>
        Run <code style="color:var(--accent)">make</code> inside the
        <code style="color:var(--accent)">wasm/</code> directory to build it,
        then serve the app with <code style="color:var(--accent)">make serve</code>.
      </p>
      <p style="margin-top:16px; font-size:0.8rem; color:var(--text-dim)">
        Requires <a href="https://emscripten.org" style="color:var(--accent)">Emscripten</a>
        (emcc) to be installed.
      </p>
    </div>`;
}
