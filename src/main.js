// 电源助手 — 前端逻辑
// 使用全局注入的 Tauri API（withGlobalTauri: true），无需打包器即可在 WebKitGTK 运行。
const { invoke } = window.__TAURI__.core;

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

  // 概览统计
  $("oVoltage").textContent = b.voltage_now == null ? "—" : `${fmt(b.voltage_now, 3)} V`;
  $("oCurrent").textContent = b.current_now == null ? "—" : `${fmt(b.current_now, 0)} mA`;
  $("oCycles").textContent = b.cycle_count ?? "—";
  $("oHealth").textContent = `${fmt(b.health_percent, 1)} %`;

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

  // 电源页（电池侧电气量）
  $("pVoltage").textContent = b.voltage_now == null ? "—" : `${fmt(b.voltage_now, 3)} V`;
  $("pOcv").textContent = b.voltage_ocv == null ? "—" : `${fmt(b.voltage_ocv, 3)} V`;
  $("pVmax").textContent = b.voltage_max == null ? "—" : `${fmt(b.voltage_max, 3)} V`;
  $("pCurrent").textContent = b.current_now == null ? "—" : `${fmt(b.current_now, 0)} mA`;
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

// ---------- 轮询 ----------
async function tick() {
  try {
    const snap = await invoke("get_snapshot");
    renderBattery(snap.battery);
    renderSources(snap.sources);
    renderChargeControl(snap.charge_control);
    const d = new Date(snap.timestamp_ms);
    $("lastUpdate").textContent = `更新于 ${d.toLocaleTimeString("zh-CN")}`;
  } catch (err) {
    $("statusText").textContent = "读取失败";
    console.error(err);
  }
}

tick();
setInterval(tick, 2000);
