// Battery SipJuice — 前端逻辑
// 使用全局注入的 Tauri API（withGlobalTauri: true），无需打包器即可在 WebKitGTK 运行。
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const RING_CIRCUM = 2 * Math.PI * 52; // 与 styles.css 的 r=52 一致

// ---------- 标签切换 ----------
document.querySelectorAll(".tab").forEach((tab) => {
  tab.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach((t) => t.classList.remove("active"));
    document.querySelectorAll(".panel").forEach((p) => p.classList.remove("active"));
    tab.classList.add("active");
    document.getElementById(tab.dataset.tab).classList.add("active");
  });
});

// ---------- 工具函数 ----------
const $ = (id) => document.getElementById(id);
const fmt = (n, d = 0) => (n == null || isNaN(n) ? "—" : Number(n).toFixed(d));
const valueWithUnit = (v, d = 0) => (v ? `${fmt(v.value, d)} ${v.unit}` : "—");

function sourceLabel(kind) {
  return (
    {
      USB: "USB 充电",
      USB_C: "USB-C 充电",
      USB_PD: "USB PD 充电",
      Mains: "交流电源",
      Wireless: "无线充电",
    }[kind] || kind || "外接电源"
  );
}

function fmtTime(min) {
  if (!min || min <= 0) return "—";
  const h = Math.floor(min / 60);
  const m = min % 60;
  return h > 0 ? `${h} 时 ${m} 分` : `${m} 分`;
}

function statusLabel(s) {
  return (
    {
      Charging: "充电中",
      Discharging: "放电中",
      Full: "已充满",
      "Not charging": "未充电",
      Unknown: "未知",
    }[s] || s || "—"
  );
}

// ---------- 渲染 ----------
function renderBattery(b) {
  if (!b) {
    $("statusText").textContent = "无电池";
    return;
  }

  // 状态条
  $("modelName").textContent = b.model || b.manufacturer || b.device;
  const pill = $("statusPill");
  pill.className = "status-pill";
  if (b.status === "Charging") pill.classList.add("charging");
  else if (b.status === "Discharging") pill.classList.add("discharging");
  else if (b.status === "Full") pill.classList.add("full");
  $("statusText").textContent = statusLabel(b.status);

  // 环形进度
  $("capacity").textContent = b.capacity ?? "--";
  const ring = $("ring");
  const pct = b.capacity ?? 0;
  ring.style.strokeDashoffset = RING_CIRCUM * (1 - pct / 100);
  ring.style.stroke = pct <= 15 ? "var(--bad)" : pct <= 30 ? "var(--warn)" : "var(--accent)";

  // hero 元信息
  $("hStatus").textContent = statusLabel(b.status);
  $("hTime").textContent =
    b.status === "Charging" ? `充满约 ${fmtTime(b.time_to_full_min)}` : fmtTime(b.time_to_empty_min);
  $("hPower").textContent = b.power_now == null ? "—" : `${fmt(Math.abs(b.power_now), 2)} W`;
  $("hTemp").textContent = b.temperature == null ? "—" : `${fmt(b.temperature, 1)} °C`;

  // 概览摘要（仪表盘：寿命 + 容量，原始电气读数归监测页）
  $("oHealth").textContent = `${fmt(b.health_percent, 1)} %`;
  $("oCycles").textContent = b.cycle_count ?? "—";
  $("oCapNow").textContent = valueWithUnit(b.full_capacity, 0);
  $("oCapDesign").textContent = valueWithUnit(b.design_capacity, 0);

  // 健康页
  $("healthScore").innerHTML = `${fmt(b.health_percent, 1)}<small>%</small>`;
  $("healthBar").style.width = `${Math.min(b.health_percent ?? 0, 100)}%`;
  $("capNow").textContent = valueWithUnit(b.full_capacity, 0);
  $("capDesign").textContent = valueWithUnit(b.design_capacity, 0);
  const lost = b.full_capacity && b.design_capacity && b.full_capacity.unit === b.design_capacity.unit
    ? { value: b.design_capacity.value - b.full_capacity.value, unit: b.full_capacity.unit }
    : null;
  $("capLost").textContent = lost ? `${fmt(lost.value, 0)} ${lost.unit}` : "—";
  $("hCycles").textContent = b.cycle_count ?? "—";
  $("hHealthStatus").textContent = b.health_status || "—";
  $("hSoh").textContent = b.state_of_health == null ? "—" : `${b.state_of_health} %`;
  $("hTech").textContent = b.technology || "—";
  $("hResistance").textContent = b.internal_resistance == null ? "—" : `${fmt(b.internal_resistance, 0)} mΩ`;

  // 监测页（电池侧实时电气量）
  $("pVoltage").textContent = b.voltage_now == null ? "—" : `${fmt(b.voltage_now, 3)} V`;
  $("pOcv").textContent = b.voltage_ocv == null ? "—" : `${fmt(b.voltage_ocv, 3)} V`;
  $("pVmax").textContent = b.voltage_max == null ? "—" : `${fmt(b.voltage_max, 3)} V`;
  $("pCurrent").textContent = b.current_now == null ? "—" : `${fmt(b.current_now, 0)} mA`;
  $("pPower").textContent = b.power_now == null ? "—" : `${fmt(Math.abs(b.power_now), 2)} W`;
  $("pTemp").textContent = b.temperature == null ? "—" : `${fmt(b.temperature, 1)} °C`;
}

function renderSources(sources) {
  const list = $("sourceList");
  const online = sources.filter((s) => s.online === true);
  if (online.length === 0) {
    list.innerHTML = '<p class="empty-hint">未检测到外接电源（当前使用电池供电）</p>';
    return;
  }
  list.innerHTML = online
    .map((s) => {
      const icon = s.kind === "Wireless" ? "📡" : s.kind === "Mains" ? "⚡" : "🔌";
      const detail = `${fmt(s.voltage_now, 2)} V · ${fmt(s.current_now, 0)} mA${
        s.usb_type ? " · " + s.usb_type : ""
      }`;
      return `<div class="source-item">
        <div class="src-icon">${icon}</div>
        <div class="src-body">
          <div class="src-name">${sourceLabel(s.kind)}</div>
          <div class="src-detail">${detail}</div>
        </div>
        <div class="src-state on">在线</div>
      </div>`;
    })
    .join("");
}

let ccInitialized = false;
function renderChargeControl(cc) {
  if (!cc) return;
  $("ccNote").textContent = cc.experimental_note;
  const card = $("ccCard");
  const btn = $("applyBtn");

  if (!cc.supported) {
    card.classList.add("disabled");
    btn.disabled = true;
    return;
  }
  // 已支持：允许调节；写权限不足时按钮仍可点，但应用会如实报错
  if (!ccInitialized) {
    if (cc.end_threshold > 0) {
      $("endSlider").value = cc.end_threshold;
      $("endVal").textContent = cc.end_threshold;
    }
    if (cc.start_threshold > 0) {
      $("startSlider").value = cc.start_threshold;
      $("startVal").textContent = cc.start_threshold;
    }
    ccInitialized = true;
  }
  btn.disabled = false;
  if (!cc.writable) {
    $("ccNote").textContent = cc.experimental_note + "（当前无写入权限，应用时需提权）";
  }
}

// ---------- 充电控制交互 ----------
$("startSlider").addEventListener("input", (e) => {
  $("startVal").textContent = e.target.value;
});
$("endSlider").addEventListener("input", (e) => {
  $("endVal").textContent = e.target.value;
});

$("applyBtn").addEventListener("click", async () => {
  const start = parseInt($("startSlider").value, 10);
  const end = parseInt($("endSlider").value, 10);
  const result = $("applyResult");
  result.className = "apply-result";
  result.textContent = "正在写入…";
  try {
    const msg = await invoke("set_charge_threshold", { start, end });
    result.className = "apply-result ok";
    result.textContent = "✓ " + msg;
  } catch (err) {
    result.className = "apply-result err";
    result.textContent = "✗ " + err;
  }
});

// ---------- 监测曲线（实时滚动 + 多档分时）----------
const METRICS = {
  cap:  { name: "电量", unit: "%",  decimals: 0, signed: false, clamp: [0, 100] },
  pow:  { name: "功率", unit: "W",  decimals: 2, signed: true },
  temp: { name: "温度", unit: "°C", decimals: 1, signed: false },
  volt: { name: "电压", unit: "V",  decimals: 3, signed: false },
  curr: { name: "电流", unit: "mA", decimals: 0, signed: true },
};

const MONITOR = { metric: "cap", rangeMs: 300000 };
// 仅 5 分档从前端实时缓冲区绘制(2 秒一帧平滑滚动)；≥30 分一律查后端 RRD 归档
// (30s/5min 粒度足够，且能显示打开软件之前的历史)。
const LIVE_MAX_MS = 300000;   // ≤ 此范围用实时缓冲
const BUFFER_MAX_MS = 600000; // 缓冲保留 10 分钟，为 5 分窗口留 2× 余量
const HISTORY_FINE_STEP_MS = 30_000;
const HISTORY_COARSE_STEP_MS = 300_000;
const HISTORY_MAX_POINTS = 240;
const rtBuffer = [];
let lastSamples = [];

const isMonitorActive = () => $("monitor").classList.contains("active");

// 把一帧快照转成与后端 Sample 一致的样本（功率带符号：放电为负）。
function pushBuffer(b, tMs) {
  const chg = b.status === "Charging";
  const mag = b.power_now == null ? null : Math.abs(b.power_now);
  rtBuffer.push({
    t: tMs,
    cap: b.capacity ?? null,
    temp: b.temperature ?? null,
    pow: mag == null ? null : chg ? mag : -mag,
    volt: b.voltage_now ?? null,
    curr: b.current_now ?? null,
    chg,
  });
  const cutoff = tMs - BUFFER_MAX_MS;
  while (rtBuffer.length && rtBuffer[0].t < cutoff) rtBuffer.shift();
}

// 启动时用后端 RRD 归档预填实时缓冲区，使 5 分档一打开就能显示打开软件之前的数据。
// 后端为 30s 粒度的历史，随后由实时 tick 追加 2s 粒度的新点；二者按时间天然衔接。
async function seedBuffer() {
  try {
    const hist = await invoke("get_history", { rangeMs: BUFFER_MAX_MS });
    if (Array.isArray(hist) && hist.length) {
      // 仅插入早于当前缓冲最早点的历史，避免与已采集的实时点重叠/乱序。
      const earliest = rtBuffer.length ? rtBuffer[0].t : Infinity;
      const older = hist.filter((s) => s.t < earliest);
      if (older.length) rtBuffer.unshift(...older);
    }
  } catch (err) {
    console.error("预填历史失败:", err);
  }
}

function fmtAxisTime(ms, rangeMs) {
  const d = new Date(ms);
  const p = (n) => String(n).padStart(2, "0");
  if (rangeMs >= 604800000) return `${d.getMonth() + 1}/${d.getDate()}`;
  if (rangeMs >= 86400000) return `${d.getMonth() + 1}/${d.getDate()} ${p(d.getHours())}h`;
  return `${p(d.getHours())}:${p(d.getMinutes())}`;
}

async function refreshChart() {
  const r = MONITOR.rangeMs;
  let samples;
  if (r <= LIVE_MAX_MS) {
    const cutoff = Date.now() - r;
    samples = rtBuffer.filter((s) => s.t >= cutoff);
  } else {
    try {
      samples = await invoke("get_history", { rangeMs: r });
    } catch (err) {
      console.error(err);
      return;
    }
  }
  renderChart(samples);
}

function renderChart(samples) {
  const m = METRICS[MONITOR.metric];
  $("chartMetricName").textContent = m.name;
  const vals = samples.map((s) => s[MONITOR.metric]).filter((v) => v != null);

  if (samples.length < 2 || vals.length === 0) {
    $("chartSvg").innerHTML = "";
    $("chartEmpty").style.display = "block";
    ["sCur", "sMin", "sMax", "sAvg"].forEach((id) => ($(id).textContent = "—"));
    lastSamples = [];
    return;
  }
  $("chartEmpty").style.display = "none";
  lastSamples = samples;
  drawChart(samples, MONITOR.metric);

  const f = (v) => `${fmt(v, m.decimals)} ${m.unit}`;
  $("sCur").textContent = f(vals[vals.length - 1]);
  $("sMin").textContent = f(Math.min(...vals));
  $("sMax").textContent = f(Math.max(...vals));
  $("sAvg").textContent = f(vals.reduce((a, b) => a + b, 0) / vals.length);
}

function drawChart(samples, metric) {
  const m = METRICS[metric];
  const svg = $("chartSvg");
  const W = svg.clientWidth || 600;
  const H = svg.clientHeight || 240;
  svg.setAttribute("viewBox", `0 0 ${W} ${H}`);

  const padL = 46, padR = 14, padT = 14, padB = 24;
  const plotW = W - padL - padR;
  const plotH = H - padT - padB;

  const ts = samples.map((s) => s.t);
  // 时间轴：右边缘钉在"现在"，窗口宽度固定为所选档位，曲线随时间往左滚动。
  const tMax = Date.now();
  const tMin = tMax - MONITOR.rangeMs;
  const tSpan = Math.max(1, tMax - tMin);
  const vals = samples.map((s) => s[metric]);
  const present = vals.filter((v) => v != null);

  let yMin = Math.min(...present), yMax = Math.max(...present);
  if (m.signed) { yMin = Math.min(yMin, 0); yMax = Math.max(yMax, 0); }
  if (yMin === yMax) { yMin -= 1; yMax += 1; }
  const padY = (yMax - yMin) * 0.1;
  yMin -= padY; yMax += padY;
  if (m.clamp) { yMin = Math.max(m.clamp[0], yMin); yMax = Math.min(m.clamp[1], yMax); }

  const X = (t) => padL + ((t - tMin) / tSpan) * plotW;
  const Y = (v) => padT + ((yMax - v) / (yMax - yMin)) * plotH;
  const gapThreshold = continuityGapThreshold();

  // 充电时段背景带
  let bands = "";
  for (let i = 0; i < samples.length - 1; i++) {
    if (samples[i].chg && ts[i + 1] - ts[i] <= gapThreshold) {
      const x0 = X(ts[i]), x1 = X(ts[i + 1]);
      bands += `<rect x="${x0.toFixed(1)}" y="${padT}" width="${Math.max(0.5, x1 - x0).toFixed(1)}" height="${plotH}" class="chg-band"/>`;
    }
  }

  // 网格线 + Y 轴标签
  let grid = "";
  for (const lv of [yMax, (yMax + yMin) / 2, yMin]) {
    const y = Y(lv);
    grid += `<line x1="${padL}" y1="${y.toFixed(1)}" x2="${W - padR}" y2="${y.toFixed(1)}" class="grid-line"/>`;
    grid += `<text x="${padL - 6}" y="${(y + 3.5).toFixed(1)}" text-anchor="end" class="axis-label">${fmt(lv, m.decimals)}</text>`;
  }

  // 0 基线（带符号指标）
  let zero = "";
  if (m.signed && yMin < 0 && yMax > 0) {
    const y0 = Y(0).toFixed(1);
    zero = `<line x1="${padL}" y1="${y0}" x2="${W - padR}" y2="${y0}" class="zero-line"/>`;
  }

  // 连续段（遇 null 断开）
  const segs = [];
  let seg = [];
  for (let i = 0; i < samples.length; i++) {
    const disconnected = i > 0 && ts[i] - ts[i - 1] > gapThreshold;
    if (vals[i] == null || disconnected) {
      if (seg.length) { segs.push(seg); seg = []; }
      if (vals[i] == null) continue;
    }
    seg.push([X(ts[i]), Y(vals[i])]);
  }
  if (seg.length) segs.push(seg);

  const baseY = m.signed && yMin < 0 && yMax > 0 ? Y(0) : padT + plotH;
  const pt = (p) => `${p[0].toFixed(1)} ${p[1].toFixed(1)}`;
  const line = segs.map((s) => "M" + s.map(pt).join(" L")).join(" ");
  const area = segs
    .map((s) => `M${s[0][0].toFixed(1)} ${baseY.toFixed(1)} L${s.map(pt).join(" L")} L${s[s.length - 1][0].toFixed(1)} ${baseY.toFixed(1)} Z`)
    .join(" ");

  // X 轴时间标签
  let xlabels = "";
  for (let i = 0; i <= 4; i++) {
    const t = tMin + (tSpan * i) / 4;
    const anchor = i === 0 ? "start" : i === 4 ? "end" : "middle";
    xlabels += `<text x="${X(t).toFixed(1)}" y="${H - 7}" text-anchor="${anchor}" class="axis-label">${fmtAxisTime(t, MONITOR.rangeMs)}</text>`;
  }

  const defs = `<defs><linearGradient id="areaGrad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="var(--accent)" stop-opacity="0.28"/>
    <stop offset="100%" stop-color="var(--accent)" stop-opacity="0.02"/>
  </linearGradient></defs>`;

  svg.innerHTML =
    defs + bands + grid + zero +
    `<path d="${area}" class="chart-area-fill"/>` +
    `<path d="${line}" class="chart-line"/>` +
    xlabels;
}

function continuityGapThreshold() {
  const sourceStep = MONITOR.rangeMs > 86400000 ? HISTORY_COARSE_STEP_MS : HISTORY_FINE_STEP_MS;
  const effectiveStep = Math.max(sourceStep, MONITOR.rangeMs / HISTORY_MAX_POINTS);
  return effectiveStep * 2.5;
}

// 指标 / 范围 chip 切换
function wireChips(rowId, key, parse) {
  $(rowId).addEventListener("click", (e) => {
    const btn = e.target.closest(".chip");
    if (!btn) return;
    [...$(rowId).children].forEach((c) => c.classList.remove("active"));
    btn.classList.add("active");
    MONITOR[key] = parse(btn.dataset[key === "metric" ? "metric" : "range"]);
    refreshChart();
  });
}
wireChips("metricChips", "metric", (v) => v);
wireChips("rangeChips", "rangeMs", (v) => parseInt(v, 10));

// 切到监测标签时立即刷新；窗口缩放时重绘
document.querySelectorAll(".tab").forEach((tab) => {
  if (tab.dataset.tab === "monitor") tab.addEventListener("click", refreshChart);
});
window.addEventListener("resize", () => {
  if (isMonitorActive() && lastSamples.length) drawChart(lastSamples, MONITOR.metric);
});

// ---------- 轮询 ----------
async function tick() {
  try {
    const snap = await invoke("get_snapshot");
    renderBattery(snap.battery);
    renderSources(snap.sources);
    renderChargeControl(snap.charge_control);
    if (snap.battery) pushBuffer(snap.battery, snap.timestamp_ms);
    // 监测页可见时跟随主轮询实时刷新曲线（短档平滑滚动，长档查后端历史）。
    if (isMonitorActive()) refreshChart();
    const d = new Date(snap.timestamp_ms);
    $("lastUpdate").textContent = `更新于 ${d.toLocaleTimeString("zh-CN")}`;
  } catch (err) {
    $("statusText").textContent = "读取失败";
    console.error(err);
  }
}

tick();
setInterval(tick, 2000);

// 预填历史缓冲，完成后若监测页可见则立即重绘短档曲线。
seedBuffer().then(() => {
  if (isMonitorActive()) refreshChart();
});

// ---------- 设置 ----------
let settings = {
  autostart: false,
  silent_start: false,
  close_action: "ask",
  remind_charge: true,
  remind_charge_at: 30,
  remind_unplug: true,
  remind_unplug_at: 80,
};
const CLOSE_ACTION_LABELS = {
  ask: "每次询问",
  tray: "最小化到托盘",
  exit: "退出应用",
};

function setCloseActionDropdownOpen(open) {
  $("closeActionDropdown").classList.toggle("open", open);
  $("setCloseActionButton").setAttribute("aria-expanded", String(open));
}

function setCloseActionValue(value, shouldPersist = false) {
  const next = CLOSE_ACTION_LABELS[value] ? value : "ask";
  settings.close_action = next;
  $("setCloseAction").value = next;
  $("setCloseActionLabel").textContent = CLOSE_ACTION_LABELS[next];
  document.querySelectorAll("#setCloseActionMenu [data-value]").forEach((item) => {
    item.setAttribute("aria-selected", String(item.dataset.value === next));
  });
  if (shouldPersist) persistSettings();
}

async function loadSettings() {
  try {
    settings = await invoke("get_settings");
    $("setAutostart").checked = settings.autostart;
    $("setSilentStart").checked = settings.silent_start;
    setCloseActionValue(settings.close_action);
    $("setRemindCharge").checked = settings.remind_charge;
    $("setRemindChargeAt").value = settings.remind_charge_at;
    $("setRemindUnplug").checked = settings.remind_unplug;
    $("setRemindUnplugAt").value = settings.remind_unplug_at;
    syncReminderInputs();
  } catch (err) {
    console.error(err);
  }
}

// 阈值输入框仅在对应提醒开启时可编辑。
function syncReminderInputs() {
  $("setRemindChargeAt").disabled = !settings.remind_charge;
  $("setRemindUnplugAt").disabled = !settings.remind_unplug;
}

async function persistSettings() {
  try {
    await invoke("save_settings", { newSettings: settings });
  } catch (err) {
    console.error("保存设置失败:", err);
  }
}

$("setAutostart").addEventListener("change", (e) => {
  settings.autostart = e.target.checked;
  persistSettings();
});
$("setSilentStart").addEventListener("change", (e) => {
  settings.silent_start = e.target.checked;
  persistSettings();
});

// 阈值输入框失焦时夹取到合法范围并保存。
function commitThreshold(inputId, key, lo, hi) {
  const el = $(inputId);
  let v = parseInt(el.value, 10);
  if (isNaN(v)) v = settings[key];
  v = Math.min(hi, Math.max(lo, v));
  el.value = v;
  settings[key] = v;
  persistSettings();
}

$("setRemindCharge").addEventListener("change", (e) => {
  settings.remind_charge = e.target.checked;
  syncReminderInputs();
  persistSettings();
});
$("setRemindUnplug").addEventListener("change", (e) => {
  settings.remind_unplug = e.target.checked;
  syncReminderInputs();
  persistSettings();
});
$("setRemindChargeAt").addEventListener("change", () =>
  commitThreshold("setRemindChargeAt", "remind_charge_at", 1, 99)
);
$("setRemindUnplugAt").addEventListener("change", () =>
  commitThreshold("setRemindUnplugAt", "remind_unplug_at", 1, 100)
);
$("setCloseActionButton").addEventListener("click", () => {
  setCloseActionDropdownOpen(!$("closeActionDropdown").classList.contains("open"));
});
$("setCloseActionMenu").addEventListener("click", (e) => {
  const item = e.target.closest("[data-value]");
  if (!item) return;
  setCloseActionValue(item.dataset.value, true);
  setCloseActionDropdownOpen(false);
});
document.addEventListener("click", (e) => {
  if (!$("closeActionDropdown").contains(e.target)) setCloseActionDropdownOpen(false);
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") setCloseActionDropdownOpen(false);
});

// ---------- 关闭确认弹框 ----------
const closeModal = $("closeModal");
const showModal = () => closeModal.classList.add("show");
const hideModal = () => closeModal.classList.remove("show");

// 后端拦截窗口关闭后发来事件 → 弹框询问（仅 close_action=ask 时触发）。
listen("close-requested", () => showModal());

$("closeCancel").addEventListener("click", hideModal);

$("closeTray").addEventListener("click", () => {
  if ($("closeRemember").checked) {
    settings.close_action = "tray";
    setCloseActionValue("tray");
    persistSettings();
  }
  hideModal();
  invoke("hide_window");
});

$("closeQuit").addEventListener("click", () => {
  if ($("closeRemember").checked) {
    settings.close_action = "exit";
    setCloseActionValue("exit");
    persistSettings();
  }
  invoke("quit_app");
});

loadSettings();
