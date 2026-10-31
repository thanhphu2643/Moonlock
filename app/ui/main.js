// Moonlock — UI layer. All calendar logic lives in Rust (moonlock-core).
const { invoke } = window.__TAURI__.core;
const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

// ── Icons (Lucide, MIT) ──
const ICONS = {
  calendar: '<rect x="3" y="4" width="18" height="18" rx="3"/><path d="M16 2v4M8 2v4M3 10h18M8 14h.01M12 14h.01M16 14h.01M8 18h.01M12 18h.01"/>',
  list: '<path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01"/>',
  flame: '<path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.07-2.14-.22-4.05 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.15.43-2.29 1-3a2.5 2.5 0 0 0 2.5 2.5z"/>',
  shield: '<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="m9 12 2 2 4-4"/>',
  terminal: '<path d="m4 17 6-6-6-6M12 19h8"/>',
  search: '<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>',
  bell: '<path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/>',
  chevronLeft: '<path d="m15 18-6-6 6-6"/>',
  chevronRight: '<path d="m9 18 6-6-6-6"/>',
  chevronDown: '<path d="m6 9 6 6 6-6"/>',
  x: '<path d="M18 6 6 18M6 6l12 12"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  download: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 10 5 5 5-5M12 15V3"/>',
  clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
  moon: '<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/>',
  copy: '<rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>',
  trash: '<path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>',
  star: '<path d="m12 2 3.1 6.3 6.9 1-5 4.9 1.2 6.8L12 17.8 5.8 21l1.2-6.8-5-4.9 6.9-1z"/>',
  rocket: '<path d="M4.5 16.5c-1.5 1.3-2 5-2 5s3.7-.5 5-2c.7-.8.7-2.1-.1-2.9a2.2 2.2 0 0 0-2.9-.1z"/><path d="m12 15-3-3a22 22 0 0 1 2-3.9A12.9 12.9 0 0 1 22 2c0 2.7-.8 7.5-6 11a22.4 22.4 0 0 1-4 2z"/><path d="M9 12H4s.6-3 2-4c1.6-1.1 5 0 5 0M12 15v5s3-.6 4-2c1.1-1.6 0-5 0-5"/>',
};
const icon = (name) =>
  `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICONS[name]}</svg>`;
const hydrateIcons = (root = document) => $$("[data-icon]", root).forEach((el) => el.insertAdjacentHTML("afterbegin", icon(el.dataset.icon)));

// ── Vocabulary ──
const VERDICT = {
  ok: { title: "Deploy được", desc: "Không vướng ngày kiêng nào. Ship it.", c: "var(--green)" },
  warn: { title: "Deploy cẩn thận", desc: "Được phép, nhưng chuẩn bị sẵn rollback.", c: "var(--amber)" },
  block: { title: "Cấm deploy", desc: "Để production yên nghỉ hôm nay.", c: "var(--red)" },
};
const SHORT = {
  mung_mot: "Mùng 1", ram: "Rằm", tam_nuong: "Tam Nương", nguyet_ky: "Nguyệt Kỵ",
  hac_dao: "Hắc đạo", tet: "Tết", friday: "Thứ 6", weekend: "Cuối tuần",
};
const MONTHS = ["tháng 1", "tháng 2", "tháng 3", "tháng 4", "tháng 5", "tháng 6", "tháng 7", "tháng 8", "tháng 9", "tháng 10", "tháng 11", "tháng 12"];
const WD_SHORT = ["CN", "T2", "T3", "T4", "T5", "T6", "T7"];
const VIEWS = {
  day: null,
  upcoming: ["Sắp tới", "Rằm, mùng 1, giỗ và lễ trong 120 ngày"],
  gio: ["Giỗ chạp", "Tính theo âm lịch, tự lặp mỗi năm"],
  rules: ["Luật deploy", "Áp dụng cho cả app và CLI moonlock"],
  ci: ["Tích hợp", "CI/CD, iCalendar, Apple Calendar"],
};

const state = { year: 0, month: 0, cells: [], selected: null, today: null, config: null, rules: [], platform: "", view: "day", pop: null };

const fmtLong = (d) => `${d.day} ${MONTHS[d.month - 1]}, ${d.year}`;
const fmtShort = (d) => `${String(d.day).padStart(2, "0")}/${String(d.month).padStart(2, "0")}`;
const lunarLong = (d) => `${d.lunar.day} tháng ${d.lunar_month_name}`;
const isoShift = (iso, n) => {
  const [y, m, d] = iso.split("-").map(Number);
  const t = new Date(Date.UTC(y, m - 1, d + n));
  return t.toISOString().slice(0, 10);
};
const daysBetween = (a, b) => b.jd - a.jd;
const relDay = (n) => (n === 0 ? "hôm nay" : n === 1 ? "ngày mai" : n === -1 ? "hôm qua" : n > 0 ? `${n} ngày nữa` : `${-n} ngày trước`);

function cellLines(d) {
  const out = [];
  for (const e of d.events) out.push({ t: e, c: e.startsWith("Giỗ") ? "gio" : "event" });
  const covered = new Set(["gio", "le_lon"]);
  if (d.events.some((e) => /^(Mùng 1|Rằm)/.test(e))) covered.add("mung_mot").add("ram");
  for (const h of d.block_reasons) if (!covered.has(h.rule)) out.push({ t: SHORT[h.rule] || h.label, c: "block" });
  for (const h of d.warn_reasons) if (!covered.has(h.rule) && h.rule !== "hac_dao") out.push({ t: SHORT[h.rule] || h.label, c: "warn" });
  return out;
}

// ── Header ──
function renderHeader() {
  const h = new Date().getHours();
  const part = h < 11 ? "sáng" : h < 14 ? "trưa" : h < 18 ? "chiều" : "tối";
  $("#greeting").textContent = `Chào buổi ${part}, dev!`;
  const t = state.today;
  const v = VERDICT[t.verdict];
  $("#subline").innerHTML = `Hôm nay là <b>${esc(lunarLong(t))}</b> âm lịch, ngày ${esc(t.can_chi_day)}
    <span class="pill"><i class="v ${t.verdict}"></i>${v.title}</span>`;
}

// ── Calendar ──
function fillPickers() {
  $("#month-sel").innerHTML = MONTHS.map((m, i) => `<option value="${i + 1}">Tháng ${i + 1}</option>`).join("");
  const y0 = state.today.year;
  $("#year-sel").innerHTML = Array.from({ length: 21 }, (_, i) => y0 - 10 + i).map((y) => `<option>${y}</option>`).join("");
}

async function loadMonth() {
  const { year, month } = state;
  const prev = month === 1 ? [year - 1, 12] : [year, month - 1];
  const next = month === 12 ? [year + 1, 1] : [year, month + 1];
  const [a, cur, b] = await Promise.all([
    invoke("month_view", { year: prev[0], month: prev[1] }),
    invoke("month_view", { year, month }),
    invoke("month_view", { year: next[0], month: next[1] }),
  ]);
  const lead = (cur[0].weekday + 6) % 7;
  const head = lead ? a.slice(-lead) : [];
  const weeks = Math.ceil((lead + cur.length) / 7);
  const tail = b.slice(0, weeks * 7 - lead - cur.length);
  state.cells = [...head.map((d) => ({ ...d, out: true })), ...cur, ...tail.map((d) => ({ ...d, out: true }))];
  state.weeks = weeks;
  $("#month-sel").value = month;
  if (![...$("#year-sel").options].some((o) => +o.value === year)) $("#year-sel").insertAdjacentHTML("beforeend", `<option>${year}</option>`);
  $("#year-sel").value = year;
  renderGrid();
}

function renderGrid() {
  const max = state.weeks === 6 ? 2 : 3;
  $("#grid").style.gridTemplateRows = `repeat(${state.weeks}, minmax(0, 1fr))`;
  $("#grid").innerHTML = state.cells.map((d) => {
    const cls = ["cell", d.out && "out", d.date === state.today.date && "today", d.date === state.selected && "selected"].filter(Boolean).join(" ");
    const first = d.lunar.day === 1;
    const lunar = first ? `${d.lunar.day}/${d.lunar.month}${d.lunar.leap ? "N" : ""}` : d.lunar.day;
    const lines = cellLines(d);
    const shown = lines.length > max ? lines.slice(0, max - 1) : lines;
    const more = lines.length - shown.length;
    const tip = [...d.events, ...d.block_reasons.map((h) => h.label), ...d.warn_reasons.map((h) => h.label)].join("\n");
    return `<button class="${cls}" data-date="${d.date}" title="${esc(tip)}" aria-label="${esc(`${fmtLong(d)}, âm lịch ${d.lunar.day}/${d.lunar.month}, ${VERDICT[d.verdict].title}`)}">
      <div class="top"><span class="num">${d.day}</span><span class="lunar ${first ? "first" : ""}"><i class="v ${d.verdict}"></i>${lunar}</span></div>
      <div class="lines">
        ${shown.map((l) => `<div class="line c-${l.c}"><span>${esc(l.t)}</span></div>`).join("")}
        ${more ? `<div class="line more">+${more} nữa</div>` : ""}
      </div>
    </button>`;
  }).join("");
}

const findCell = (iso) => state.cells.find((d) => d.date === iso);

async function select(iso, { reveal = true } = {}) {
  state.selected = iso;
  let d = findCell(iso);
  if (!d || (reveal && d.out)) {
    const [y, m] = iso.split("-").map(Number);
    state.year = y;
    state.month = m;
    await loadMonth();
    d = findCell(iso);
  } else renderGrid();
  if (state.view === "day") renderSide();
  return d;
}

// ── Side panel ──
function setSideHead(title, sub, showDayTools) {
  $("#side-title").textContent = title;
  $("#side-sub").textContent = sub;
  $("#day-tools").classList.toggle("hidden", !showDayTools);
}

async function renderSide() {
  const body = $("#side-body");
  const v = state.view;
  if (v === "day") {
    const d = findCell(state.selected) || (await invoke("day_at", { date: state.selected }));
    const isToday = d.date === state.today.date;
    setSideHead(isToday ? "Hôm nay" : `${d.weekday_name}`, `${fmtLong(d)} · ${relDay(daysBetween(state.today, d))}`, true);
    body.innerHTML = await dayView(d);
  } else {
    setSideHead(...VIEWS[v], false);
    body.innerHTML = await ({ upcoming: upcomingView, gio: gioView, rules: rulesView, ci: ciView })[v]();
  }
  hydrateIcons(body);
}

async function dayView(d) {
  const v = VERDICT[d.verdict];
  const reasons = [
    ...d.block_reasons.map((h) => ({ ...h, c: "var(--red)" })),
    ...d.warn_reasons.map((h) => ({ ...h, c: "var(--amber)" })),
  ];
  let next = "";
  if (d.verdict === "block") {
    const n = await nextWindowFrom(d);
    if (n) next = `<div class="row"><span class="l" data-icon="rocket">Lần deploy kế tiếp</span>
      <button class="chipbtn" data-goto="${n.date}">${WD_SHORT[n.weekday]} ${fmtShort(n)}</button></div>`;
  }
  const events = d.events.length
    ? d.events.map((e) => `<div class="card" style="--c:${e.startsWith("Giỗ") ? "var(--violet)" : "var(--sky)"}">
        <div class="bar"></div><h3>${esc(e)}</h3>
        <p class="desc">${esc(lunarLong(d))} âm lịch</p>
        <div class="rows"><div class="row"><span class="l" data-icon="clock">Nhắc trước</span><span class="r">${state.config.remind_days_before} ngày</span></div></div>
      </div>`).join("")
    : `<p class="empty">Không có rằm, giỗ hay lễ nào.</p>`;

  return `
    <div class="divider">Deploy</div>
    <div class="card" style="--c:${v.c}">
      <div class="bar"></div>
      <h3>${v.title}</h3>
      <p class="desc">${v.desc}</p>
      <div class="rows">
        ${reasons.map((h) => `<div class="row reason" style="--c:${h.c}"><span class="l">${esc(h.label)}</span></div>`).join("")}
        ${next}
      </div>
    </div>

    <div class="divider">Âm lịch</div>
    <div class="card" style="--c:var(--blue)">
      <div class="bar"></div>
      <h3>${esc(lunarLong(d))}${d.lunar.leap ? " (nhuận)" : ""}</h3>
      <p class="desc">Năm ${esc(d.can_chi_year)}</p>
      <div class="rows">
        <div class="row"><span class="l" data-icon="moon">Ngày</span><span class="r strong">${esc(d.can_chi_day)}</span></div>
        <div class="row"><span class="l" data-icon="calendar">Tháng</span><span class="r strong">${esc(d.can_chi_month)}</span></div>
        <div class="row"><span class="l" data-icon="star">Trực tinh</span><span class="r">${esc(d.star)} · ${d.hoang_dao ? "hoàng đạo" : "hắc đạo"}</span></div>
      </div>
    </div>

    <div class="divider">Sự kiện</div>
    ${events}
    <div class="actions"><button class="btn-ghost" data-add-gio="${d.date}" data-icon="plus">Thêm giỗ vào ngày này</button></div>`;
}

async function nextWindowFrom(d) {
  for (let i = 1; i <= 60; i++) {
    const iso = isoShift(d.date, i);
    const n = findCell(iso) || (await invoke("day_at", { date: iso }));
    if (n && n.verdict !== "block") return n;
  }
  return null;
}

const miniItem = (d, title, sub, c) => `
  <div class="item" data-goto="${d.date}" style="--c:${c}">
    <div class="date"><b>${d.day}</b><small>${WD_SHORT[d.weekday]} · th${d.month}</small></div>
    <div class="t"><div>${esc(title)}</div><small>${esc(sub)}</small></div>
    <span class="tick"></span>
  </div>`;

async function upcomingView() {
  const list = await invoke("upcoming", { days: 120 });
  if (!list.length) return `<p class="empty">Không có sự kiện nào.</p>`;
  let html = "";
  let lastMonth = 0;
  for (const d of list) {
    if (d.month !== lastMonth) {
      html += `<div class="divider">Tháng ${d.month}/${d.year}</div>`;
      lastMonth = d.month;
    }
    const gio = d.events.some((e) => e.startsWith("Giỗ"));
    html += miniItem(d, d.events.join(" · "), `${d.lunar.day}/${d.lunar_month_name} âm · ${relDay(daysBetween(state.today, d))}`, gio ? "var(--violet)" : "var(--sky)");
  }
  return `<div class="mini">${html}</div>`;
}

async function gioView() {
  const g = state.config.gio;
  const nexts = await Promise.all(g.map((x) => invoke("find_lunar", { day: x.day, month: x.month, year: null })));
  const items = g.map((x, i) => {
    const n = nexts[i];
    return `<div class="card" style="--c:var(--violet)">
      <div class="bar"></div>
      <h3>${esc(x.name)}</h3>
      <p class="desc">Ngày ${x.day} tháng ${x.month} âm lịch${x.note ? ` · ${esc(x.note)}` : ""}</p>
      <div class="rows">
        <div class="row"><span class="l" data-icon="clock">${n ? `${fmtLong(n)}` : "—"}</span><span class="r">${n ? relDay(daysBetween(state.today, n)) : ""}</span></div>
        <div class="row">${n ? `<button class="chipbtn" data-goto="${n.date}" data-icon="calendar">Xem ngày</button>` : ""}
          <button class="chipbtn" data-del-gio="${i}" data-icon="trash">Xoá</button></div>
      </div>
    </div>`;
  });
  return `
    <div class="actions"><button class="btn-primary" data-add-gio="${state.selected}">Thêm ngày giỗ</button></div>
    ${items.join("") || `<p class="empty">Chưa có ngày giỗ nào. Ngày giỗ sẽ tự chặn deploy và nhắc trước.</p>`}
    <p class="note">Giỗ ngày 30 mà tháng thiếu sẽ tính vào ngày 29. Tháng nhuận cúng theo tháng thường.</p>`;
}

function rulesView() {
  const { block, warn } = state.config.deploy;
  const rows = state.rules.map((r) => {
    const cur = block.includes(r.id) ? "block" : warn.includes(r.id) ? "warn" : "off";
    const b = (v, t) => `<button data-rule="${r.id}" data-v="${v}" class="${cur === v ? "on" : ""}" aria-pressed="${cur === v}">${t}</button>`;
    const [name, detail] = r.title.replace(/\)$/, "").split(" (");
    return `<div class="rule"><span class="rule-name">${esc(name)}${detail ? `<small>${esc(detail)}</small>` : ""}</span><span class="seg">${b("block", "Chặn")}${b("warn", "Cảnh báo")}${b("off", "Tắt")}</span></div>`;
  });
  return `
    <div class="divider">Ngày kiêng</div>
    <div class="card" style="padding-bottom:4px">${rows.join("")}</div>
    <div class="divider">Nhắc nhở</div>
    <div class="card"><div class="rule" style="border:0"><span>Nhắc trước rằm, mùng 1, giỗ</span>
      <span><input id="remind" class="num-in" type="number" min="0" max="7" value="${state.config.remind_days_before}" /> ngày</span></div></div>
    <p class="note"><b>Chặn</b>: <code>moonlock deploy-check</code> trả exit 1. <b>Cảnh báo</b>: chỉ chặn khi chạy với <code>--strict</code>.</p>`;
}

async function ciView() {
  const [path, toml] = await invoke("config_info");
  return `
    <div class="divider">Xuất lịch</div>
    <div class="actions">
      <button class="btn-ghost" id="ics" data-icon="download">Export .ics</button>
      ${state.platform === "macos" ? `<button class="btn-ghost" id="apple" data-icon="calendar">Apple Calendar</button>` : ""}
    </div>
    <div class="toast" id="toast"></div>
    <div class="divider">.moonlock.toml</div>
    <pre class="code" id="toml">${esc(toml)}</pre>
    <button class="btn-ghost" id="copy-toml" data-icon="copy">Copy cấu hình</button>
    <div class="divider">GitHub Actions</div>
    <pre class="code">- uses: thanhphu2643/Moonlock@main
  with:
    strict: false
# khẩn cấp:
# env: { MOONLOCK_ALLOW_DEPLOY: "đã thắp hương" }</pre>
    <p class="note">Cấu hình app: ${esc(path)}</p>`;
}

// ── Popover: add giỗ ──
async function openPop(iso, anchor) {
  const d = findCell(iso) || (await invoke("day_at", { date: iso }));
  state.pop = { day: d.lunar.day, month: d.lunar.month };
  const pop = $("#pop");
  pop.classList.remove("hidden");
  await updatePop();
  // Place next to the anchor cell like the reference; fall back to centered.
  const cal = $(".cal").getBoundingClientRect();
  const w = pop.offsetWidth, h = pop.offsetHeight;
  let x = (cal.width - w) / 2, y = (cal.height - h) / 2;
  if (anchor) {
    const r = anchor.getBoundingClientRect();
    x = r.right - cal.left + 10;
    if (x + w > cal.width - 12) x = r.left - cal.left - w - 10;
    y = Math.min(Math.max(r.top - cal.top, 12), cal.height - h - 12);
  }
  pop.style.left = `${Math.max(12, x)}px`;
  pop.style.top = `${Math.max(12, y)}px`;
  $("input[name=name]", pop).focus();
}

async function updatePop() {
  const { day, month } = state.pop;
  $("#pop-day").textContent = day;
  $("#pop-month").textContent = month;
  const n = await invoke("find_lunar", { day, month, year: null });
  $("#pop-solar").textContent = n ? `${fmtLong(n)} · ${relDay(daysBetween(state.today, n))}` : "—";
}

const closePop = () => {
  $("#pop").classList.add("hidden");
  $("#gio-form").reset();
  state.pop = null;
};

// ── Actions ──
async function saveConfig() {
  await invoke("save_config", { config: state.config });
  await refresh();
}

async function refresh() {
  state.today = await invoke("today");
  renderHeader();
  await loadMonth();
  await renderSide();
  const soon = await invoke("upcoming", { days: 4 });
  $("#bell-dot").classList.toggle("hidden", !soon.length);
}

function setView(v) {
  state.view = v;
  $$(".rail-btn").forEach((b) => b.classList.toggle("active", b.dataset.view === v));
  renderSide();
}

const fold = (s) => s.normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/đ/gi, "d").toLowerCase().trim();

async function search(q) {
  q = q.trim();
  if (!q) return;
  let hit = null;
  const lunar = q.match(/^(\d{1,2})\s*[/.-]\s*(\d{1,2})(?:\s*[/.-]\s*(\d{4}))?$/);
  if (/^\d{4}-\d{2}-\d{2}$/.test(q)) hit = await invoke("day_at", { date: q });
  else if (lunar) hit = await invoke("find_lunar", { day: +lunar[1], month: +lunar[2], year: lunar[3] ? +lunar[3] : null });
  else if (["hom nay", "today"].includes(fold(q))) hit = state.today;
  else {
    const g = state.config.gio.find((x) => fold(x.name).includes(fold(q)) || fold(`gio ${x.name}`).includes(fold(q)));
    if (g) hit = await invoke("find_lunar", { day: g.day, month: g.month, year: null });
  }
  if (!hit) {
    $("#search").classList.remove("miss");
    void $("#search").offsetWidth;
    $("#search").classList.add("miss");
    return;
  }
  setView("day");
  await select(hit.date);
}

// ── Wiring ──
function wire() {
  $$(".rail-btn").forEach((b) => (b.onclick = () => setView(b.dataset.view)));
  $("#bell").onclick = () => setView("upcoming");
  $("#prev").onclick = () => { state.month === 1 ? (state.year--, (state.month = 12)) : state.month--; loadMonth(); };
  $("#next").onclick = () => { state.month === 12 ? (state.year++, (state.month = 1)) : state.month++; loadMonth(); };
  $("#month-sel").onchange = (e) => { state.month = +e.target.value; loadMonth(); };
  $("#year-sel").onchange = (e) => { state.year = +e.target.value; loadMonth(); };
  $("#go-today").onclick = () => select(state.today.date);
  $("#day-prev").onclick = () => select(isoShift(state.selected, -1));
  $("#day-next").onclick = () => select(isoShift(state.selected, 1));
  $("#search").onsubmit = (e) => { e.preventDefault(); search($("#q").value); };

  $("#grid").addEventListener("click", (e) => {
    const c = e.target.closest(".cell");
    if (!c) return;
    if (state.view !== "day") setView("day");
    select(c.dataset.date, { reveal: true });
  });
  $("#grid").addEventListener("dblclick", (e) => {
    const c = e.target.closest(".cell");
    if (c) openPop(c.dataset.date, c);
  });

  $("#side-body").addEventListener("click", async (e) => {
    const t = e.target.closest("button, .item");
    if (!t) return;
    if (t.dataset.goto) {
      setView("day");
      return select(t.dataset.goto);
    }
    if (t.dataset.addGio) return openPop(t.dataset.addGio, null);
    if (t.dataset.delGio) {
      state.config.gio.splice(+t.dataset.delGio, 1);
      return saveConfig();
    }
    if (t.dataset.rule) {
      const { rule, v } = t.dataset;
      const p = state.config.deploy;
      p.block = p.block.filter((r) => r !== rule);
      p.warn = p.warn.filter((r) => r !== rule);
      if (v !== "off") p[v].push(rule);
      return saveConfig();
    }
    const toast = (m) => ($("#toast").textContent = m);
    if (t.id === "ics") {
      try { toast(`Đã lưu ${await invoke("export_ics", { days: 365 })}. Mở file để import vào Google, Apple hoặc Outlook Calendar.`); }
      catch (err) { toast(`Lỗi: ${err}`); }
    }
    if (t.id === "apple") {
      toast("Đang ghi vào Apple Calendar. Lần đầu macOS sẽ hỏi quyền truy cập Lịch.");
      try { toast(`Đã thêm ${await invoke("sync_apple_calendar", { days: 365 })} sự kiện vào lịch "Moonlock".`); }
      catch (err) { toast(String(err)); }
    }
    if (t.id === "copy-toml") {
      await navigator.clipboard.writeText($("#toml").textContent);
      t.lastChild.textContent = "Đã copy";
      setTimeout(() => (t.lastChild.textContent = "Copy cấu hình"), 1500);
    }
  });
  $("#side-body").addEventListener("change", (e) => {
    if (e.target.id !== "remind") return;
    state.config.remind_days_before = Math.max(0, Math.min(7, +e.target.value || 0));
    saveConfig();
  });

  // Popover
  $("#pop-close").onclick = closePop;
  $("#pop").addEventListener("click", (e) => {
    const b = e.target.closest("[data-step]");
    if (!b) return;
    const [k, n] = b.dataset.step.split(":");
    const lim = k === "day" ? 30 : 12;
    state.pop[k] = ((state.pop[k] - 1 + +n + lim) % lim) + 1;
    updatePop();
  });
  $("#gio-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    const f = new FormData(e.target);
    const note = f.get("note").trim();
    state.config.gio.push({ name: f.get("name").trim(), day: state.pop.day, month: state.pop.month, ...(note && { note }) });
    closePop();
    await saveConfig();
  });
  document.addEventListener("mousedown", (e) => {
    if (state.pop && !e.target.closest("#pop") && !e.target.closest("[data-add-gio]")) closePop();
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && state.pop) return closePop();
    if (["INPUT", "SELECT"].includes(e.target.tagName)) return;
    if (e.key === "ArrowLeft") $("#day-prev").click();
    if (e.key === "ArrowRight") $("#day-next").click();
    if (e.key === "ArrowUp") select(isoShift(state.selected, -7));
    if (e.key === "ArrowDown") select(isoShift(state.selected, 7));
    if (e.key === "t") $("#go-today").click();
    if (e.key === "/") { e.preventDefault(); $("#q").focus(); }
  });
}

// ── Boot ──
(async () => {
  hydrateIcons();
  const [config, rules, platform, today] = await Promise.all([invoke("get_config"), invoke("rules"), invoke("platform"), invoke("today")]);
  Object.assign(state, { config, rules, platform, today, year: today.year, month: today.month, selected: today.date });
  fillPickers();
  wire();
  await refresh();
  setInterval(async () => {
    const t = await invoke("today");
    if (t.date !== state.today.date) refresh();
    else renderHeader();
  }, 60_000);
})();
