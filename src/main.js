// Battery SipJuice — 前端逻辑
// 使用全局注入的 Tauri API（withGlobalTauri: true），无需打包器即可在 WebKitGTK 运行。
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const RING_CIRCUM = 2 * Math.PI * 52; // 与 styles.css 的 r=52 一致
let currentLanguage = "zh-CN";
let lastSnapshot = null;
let lastCpuPowerState = null;

const I18N = {
  "zh-CN": {
    "section.interface": "界面",
    "section.general": "常规",
    "section.energy": "能耗控制",
    "section.reminders": "电池养护提醒",
    "section.powerSource": "输入电源",
    "section.about": "关于",
    "tab.overview": "概览",
    "tab.monitor": "监测",
    "tab.health": "健康",
    "tab.settings": "设置",
    "settings.language.name": "界面语言",
    "settings.language.desc": "切换应用内显示文字",
    "settings.theme.name": "外观主题",
    "settings.theme.desc": "选择浅色、深色或跟随系统",
    "theme.system": "跟随系统",
    "theme.system.sub": "自动",
    "theme.light": "浅色",
    "theme.light.sub": "明亮",
    "theme.dark": "深色",
    "theme.dark.sub": "低亮度",
    "settings.autostart.name": "开机自启动",
    "settings.autostart.desc": "登录系统后自动启动 Battery SipJuice",
    "settings.silent.name": "静默启动",
    "settings.silent.desc": "启动时不显示窗口，仅在系统托盘运行",
    "settings.close.name": "关闭按钮行为",
    "settings.close.desc": "点击窗口关闭按钮(✕)时的动作",
    "settings.energy.desc": "比系统省电模式更强，直接限制 CPU/GPU 频率上限，适合后台编译、下载、低电量和发热时使用。",
    "settings.super.name": "超级省电模式",
    "settings.super.desc": "限制各组 CPU 核心和 GPU 最高频率，降低峰值功耗和发热。",
    "settings.super.checking": "正在读取 CPU/GPU 频率策略…",
    "settings.super.applying": "正在请求管理员权限并切换 CPU/GPU 频率上限…",
    "settings.super.active": "已开启 · {caps}",
    "settings.super.inactive": "未开启 · 当前上限 {caps}",
    "settings.super.wantOnMismatch": "你希望开启，但当前未生效 · 当前上限 {caps}",
    "settings.super.wantOffMismatch": "检测到系统当前仍在限频 · 当前上限 {caps}",
    "settings.super.reapply": "重新应用",
    "settings.super.restore": "恢复默认",
    "settings.super.unsupported": "当前系统没有暴露可调节的 CPU/GPU 频率策略。",
    "settings.super.error": "切换失败：{message}",
    "settings.reminders.desc": "按电量阈值弹出系统通知，纯软件实现，任何设备都能用，不改变充电行为。",
    "settings.low.name": "低电量提醒",
    "settings.low.desc": "放电时电量低于阈值，提醒接上电源",
    "settings.high.name": "高电量提醒",
    "settings.high.desc": "充电时电量高于阈值，提醒拔掉电源",
    "close.ask": "每次询问",
    "close.tray": "最小化到托盘",
    "close.exit": "退出应用",
    "stat.status": "状态",
    "stat.timeRemaining": "剩余时间",
    "stat.power": "功率",
    "stat.temperature": "温度",
    "stat.health": "容量健康",
    "stat.cycles": "循环次数",
    "stat.fullCapacity": "实际满电容量",
    "stat.designCapacity": "设计容量",
    "prediction.title": "当前功率预测",
    "prediction.note": "按当前 {power} 估算，负载变化会让结果跟着变化。",
    "prediction.note.empty": "缺少当前功率或容量数据，暂时无法估算。",
    "prediction.full": "预计充满",
    "prediction.high": "到达 {threshold}% 以上",
    "prediction.low": "低于 {threshold}% 以下",
    "prediction.empty": "预计没电",
    "prediction.reached.high": "已高于 {threshold}%",
    "prediction.reached.low": "已低于 {threshold}%",
    "prediction.direction.charging": "正在充电",
    "prediction.direction.discharging": "正在放电",
    "stat.capacityLost": "已损耗",
    "stat.chargeCycles": "充电循环",
    "stat.healthStatus": "健康状态",
    "stat.driverSoh": "驱动 SOH",
    "stat.technology": "电池技术",
    "stat.resistance": "内阻",
    "stat.voltageNow": "瞬时电压",
    "stat.ocv": "开路电压(OCV)",
    "stat.vmax": "最大电压",
    "stat.currentNow": "瞬时电流",
    "stat.current": "当前",
    "stat.min": "区间最小",
    "stat.max": "区间最大",
    "stat.avg": "区间平均",
    "health.note": "容量健康 = 当前满电容量 / 设计容量",
    "metric.cap": "电量",
    "metric.pow": "功率",
    "metric.temp": "温度",
    "metric.volt": "电压",
    "metric.curr": "电流",
    "range.5m": "5 分",
    "range.30m": "30 分",
    "range.6h": "6 时",
    "range.24h": "24 时",
    "range.7d": "7 天",
    "chart.empty": "正在采集数据…曲线会随时间逐渐填充。",
    "chart.chargingPeriod": "充电时段",
    "source.empty": "未检测到外接电源",
    "source.empty.battery": "未检测到外接电源（当前使用电池供电）",
    "source.online": "在线",
    "source.usb": "USB 充电",
    "source.usbc": "USB-C 充电",
    "source.usbpd": "USB PD 充电",
    "source.mains": "交流电源",
    "source.wireless": "无线充电",
    "source.fallback": "外接电源",
    "status.charging": "充电中",
    "status.discharging": "放电中",
    "status.full": "已充满",
    "status.notCharging": "未充电",
    "status.unknown": "未知",
    "status.noBattery": "无电池",
    "status.reading": "读取中…",
    "status.readFailed": "读取失败",
    "time.hour": "时",
    "time.minute": "分",
    "time.toFull": "充满约 {time}",
    "footer.updated": "更新于 {time}",
    "footer.privacy": "本机直读 sysfs · 无网络上报",
    "about.app": "应用",
    "about.version": "版本",
    "about.source": "数据来源",
    "about.sourceValue": "本机 sysfs · 无网络上报",
    "modal.title": "关闭 Battery SipJuice",
    "modal.body": "你希望退出应用，还是最小化到系统托盘继续后台监控？",
    "modal.remember": "记住我的选择，不再询问",
    "modal.cancel": "取消",
    "modal.tray": "最小化到托盘",
    "modal.quit": "退出",
  },
  "en-US": {
    "section.interface": "Interface",
    "section.general": "General",
    "section.energy": "Power Control",
    "section.reminders": "Battery Care Reminders",
    "section.powerSource": "Power Sources",
    "section.about": "About",
    "tab.overview": "Overview",
    "tab.monitor": "Monitor",
    "tab.health": "Health",
    "tab.settings": "Settings",
    "settings.language.name": "Interface Language",
    "settings.language.desc": "Change the text used in the app",
    "settings.theme.name": "Appearance Theme",
    "settings.theme.desc": "Choose light, dark, or follow the system",
    "theme.system": "System",
    "theme.system.sub": "Auto",
    "theme.light": "Light",
    "theme.light.sub": "Bright",
    "theme.dark": "Dark",
    "theme.dark.sub": "Low light",
    "settings.autostart.name": "Launch at Login",
    "settings.autostart.desc": "Start Battery SipJuice when you sign in",
    "settings.silent.name": "Silent Start",
    "settings.silent.desc": "Start hidden and keep running in the tray",
    "settings.close.name": "Close Button",
    "settings.close.desc": "What happens when you click the window close button",
    "settings.energy.desc": "Stronger than the system power saver. It directly caps CPU/GPU frequency for background compiling, downloads, low battery, and heat.",
    "settings.super.name": "Super Power Saver",
    "settings.super.desc": "Caps each CPU cluster and GPU to reduce peak power and heat.",
    "settings.super.checking": "Reading CPU/GPU frequency policies…",
    "settings.super.applying": "Requesting administrator permission and changing CPU/GPU limits…",
    "settings.super.active": "On · {caps}",
    "settings.super.inactive": "Off · Current caps {caps}",
    "settings.super.wantOnMismatch": "You want it on, but it is not active · Current caps {caps}",
    "settings.super.wantOffMismatch": "The system is still frequency-limited · Current caps {caps}",
    "settings.super.reapply": "Apply Again",
    "settings.super.restore": "Restore Defaults",
    "settings.super.unsupported": "This system does not expose adjustable CPU/GPU frequency policies.",
    "settings.super.error": "Failed to switch: {message}",
    "settings.reminders.desc": "Show system notifications at battery thresholds. Software only, works on any device, and does not change charging behavior.",
    "settings.low.name": "Low Battery Reminder",
    "settings.low.desc": "Notify when discharging below the threshold",
    "settings.high.name": "High Battery Reminder",
    "settings.high.desc": "Notify when charging above the threshold",
    "close.ask": "Ask Every Time",
    "close.tray": "Minimize to Tray",
    "close.exit": "Quit App",
    "stat.status": "Status",
    "stat.timeRemaining": "Time Remaining",
    "stat.power": "Power",
    "stat.temperature": "Temperature",
    "stat.health": "Capacity Health",
    "stat.cycles": "Cycle Count",
    "stat.fullCapacity": "Full Capacity",
    "stat.designCapacity": "Design Capacity",
    "prediction.title": "Current Power Forecast",
    "prediction.note": "Estimated from current {power}. It will change as the load changes.",
    "prediction.note.empty": "Missing current power or capacity data, so no forecast is available yet.",
    "prediction.full": "Full",
    "prediction.high": "Above {threshold}%",
    "prediction.low": "Below {threshold}%",
    "prediction.empty": "Empty",
    "prediction.reached.high": "Already above {threshold}%",
    "prediction.reached.low": "Already below {threshold}%",
    "prediction.direction.charging": "Charging",
    "prediction.direction.discharging": "Discharging",
    "stat.capacityLost": "Capacity Lost",
    "stat.chargeCycles": "Charge Cycles",
    "stat.healthStatus": "Health Status",
    "stat.driverSoh": "Driver SOH",
    "stat.technology": "Technology",
    "stat.resistance": "Resistance",
    "stat.voltageNow": "Instant Voltage",
    "stat.ocv": "Open-Circuit Voltage",
    "stat.vmax": "Max Voltage",
    "stat.currentNow": "Instant Current",
    "stat.current": "Current",
    "stat.min": "Range Min",
    "stat.max": "Range Max",
    "stat.avg": "Range Avg",
    "health.note": "Capacity health = current full capacity / design capacity",
    "metric.cap": "Battery",
    "metric.pow": "Power",
    "metric.temp": "Temperature",
    "metric.volt": "Voltage",
    "metric.curr": "Current",
    "range.5m": "5 min",
    "range.30m": "30 min",
    "range.6h": "6 h",
    "range.24h": "24 h",
    "range.7d": "7 days",
    "chart.empty": "Collecting data… the chart will fill in over time.",
    "chart.chargingPeriod": "Charging period",
    "source.empty": "No external power detected",
    "source.empty.battery": "No external power detected (currently on battery)",
    "source.online": "Online",
    "source.usb": "USB charging",
    "source.usbc": "USB-C charging",
    "source.usbpd": "USB PD charging",
    "source.mains": "AC power",
    "source.wireless": "Wireless charging",
    "source.fallback": "External power",
    "status.charging": "Charging",
    "status.discharging": "Discharging",
    "status.full": "Full",
    "status.notCharging": "Not charging",
    "status.unknown": "Unknown",
    "status.noBattery": "No battery",
    "status.reading": "Reading…",
    "status.readFailed": "Read failed",
    "time.hour": "h",
    "time.minute": "min",
    "time.toFull": "Full in about {time}",
    "footer.updated": "Updated at {time}",
    "footer.privacy": "Local sysfs only · no network reporting",
    "about.app": "App",
    "about.version": "Version",
    "about.source": "Data Source",
    "about.sourceValue": "Local sysfs · no network reporting",
    "modal.title": "Close Battery SipJuice",
    "modal.body": "Do you want to quit the app, or minimize it to the tray and keep monitoring?",
    "modal.remember": "Remember my choice and do not ask again",
    "modal.cancel": "Cancel",
    "modal.tray": "Minimize to Tray",
    "modal.quit": "Quit",
  },
};

const t = (key, params = {}) => {
  let text = I18N[currentLanguage]?.[key] ?? I18N["en-US"]?.[key] ?? key;
  Object.entries(params).forEach(([name, value]) => {
    text = text.replace(`{${name}}`, value);
  });
  return text;
};

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

function applyTheme(theme) {
  const next = ["system", "light", "dark"].includes(theme) ? theme : "system";
  if (next === "system") document.documentElement.removeAttribute("data-theme");
  else document.documentElement.dataset.theme = next;
}

function setSegmentedValue(groupId, value) {
  const group = $(groupId);
  group.querySelectorAll("[data-setting-value]").forEach((btn) => {
    const active = btn.dataset.settingValue === value;
    btn.classList.toggle("active", active);
    btn.setAttribute("aria-checked", String(active));
  });
}

function applyLanguage(language) {
  currentLanguage = I18N[language] ? language : "zh-CN";
  document.documentElement.lang = currentLanguage;
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    el.textContent = t(el.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-aria]").forEach((el) => {
    el.setAttribute("aria-label", t(el.dataset.i18nAria));
  });
  document.querySelectorAll("#metricChips [data-metric]").forEach((btn) => {
    const metric = METRICS[btn.dataset.metric];
    btn.textContent = `${t(metric.nameKey)} ${metric.unit}`;
  });
  setCloseActionValue(settings.close_action);
  if (lastCpuPowerState) renderSuperPowerSaver(lastCpuPowerState);
  if (lastSnapshot) {
    renderBattery(lastSnapshot.battery);
    renderSources(lastSnapshot.sources);
    const d = new Date(lastSnapshot.timestamp_ms);
    $("lastUpdate").textContent = t("footer.updated", { time: d.toLocaleTimeString(currentLanguage) });
  }
  if (isMonitorActive()) refreshChart();
}

function sourceLabel(kind) {
  return (
    {
      USB: t("source.usb"),
      USB_C: t("source.usbc"),
      USB_PD: t("source.usbpd"),
      Mains: t("source.mains"),
      Wireless: t("source.wireless"),
    }[kind] || kind || t("source.fallback")
  );
}

function fmtTime(min) {
  if (!min || min <= 0) return "—";
  const h = Math.floor(min / 60);
  const m = min % 60;
  return h > 0 ? `${h} ${t("time.hour")} ${m} ${t("time.minute")}` : `${m} ${t("time.minute")}`;
}

function statusLabel(s) {
  return (
    {
      Charging: t("status.charging"),
      Discharging: t("status.discharging"),
      Full: t("status.full"),
      "Not charging": t("status.notCharging"),
      Unknown: t("status.unknown"),
    }[s] || s || "—"
  );
}

function fullEnergyWh(b) {
  const full = b?.full_capacity;
  if (!full) return null;
  if (full.unit === "Wh") return full.value;
  if (full.unit === "mAh") {
    const voltage = b.voltage_now ?? b.voltage_ocv ?? b.voltage_max;
    return voltage == null ? null : (full.value * voltage) / 1000;
  }
  return null;
}

// 仅用于任意阈值(用户自定义的低/高提醒百分比)的到达时间预测——
// 到 0%/100% 的用时已由后端 time_to_empty_min/time_to_full_min 给出，直接复用即可。
function minutesForPercentDelta(b, deltaPct) {
  const energyWh = fullEnergyWh(b);
  const powerW = b?.power_now == null ? null : Math.abs(b.power_now);
  if (energyWh == null || powerW == null || powerW < 0.01 || deltaPct <= 0) return null;
  return Math.round((energyWh * (deltaPct / 100) / powerW) * 60);
}

function renderPowerPrediction(b) {
  const ids = ["predFull", "predHigh", "predLow", "predEmpty"];
  const setAllEmpty = (note = t("prediction.note.empty")) => {
    ids.forEach((id) => ($(id).textContent = "—"));
    $("predictionNote").textContent = note;
  };

  const low = Number(settings.remind_charge_at ?? 30);
  const high = Number(settings.remind_unplug_at ?? 80);
  $("predHighLabel").textContent = t("prediction.high", { threshold: high });
  $("predLowLabel").textContent = t("prediction.low", { threshold: low });

  if (!b || b.capacity == null) {
    setAllEmpty();
    return;
  }

  const powerW = b.power_now == null ? null : Math.abs(b.power_now);
  if (powerW == null || powerW < 0.01 || fullEnergyWh(b) == null) {
    setAllEmpty();
    return;
  }

  const cap = Number(b.capacity);
  ids.forEach((id) => ($(id).textContent = "—"));
  $("predictionNote").textContent = t("prediction.note", { power: `${fmt(powerW, 2)} W` });

  if (b.status === "Charging") {
    $("predFull").textContent = fmtTime(b.time_to_full_min);
    $("predHigh").textContent =
      cap >= high
        ? t("prediction.reached.high", { threshold: high })
        : fmtTime(minutesForPercentDelta(b, high - cap));
  } else if (b.status === "Discharging") {
    $("predLow").textContent =
      cap <= low
        ? t("prediction.reached.low", { threshold: low })
        : fmtTime(minutesForPercentDelta(b, cap - low));
    $("predEmpty").textContent = fmtTime(b.time_to_empty_min);
  } else if (b.status === "Full") {
    $("predFull").textContent = statusLabel("Full");
    $("predHigh").textContent = t("prediction.reached.high", { threshold: high });
  }
}

// ---------- 渲染 ----------
function renderBattery(b) {
  if (!b) {
    $("statusText").textContent = t("status.noBattery");
    renderPowerPrediction(null);
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
    b.status === "Charging"
      ? t("time.toFull", { time: fmtTime(b.time_to_full_min) })
      : fmtTime(b.time_to_empty_min);
  $("hPower").textContent = b.power_now == null ? "—" : `${fmt(b.power_now, 2)} W`;
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
  renderPowerPrediction(b);

  // 监测页（电池侧实时电气量）
  $("pVoltage").textContent = b.voltage_now == null ? "—" : `${fmt(b.voltage_now, 3)} V`;
  $("pOcv").textContent = b.voltage_ocv == null ? "—" : `${fmt(b.voltage_ocv, 3)} V`;
  $("pVmax").textContent = b.voltage_max == null ? "—" : `${fmt(b.voltage_max, 3)} V`;
  $("pCurrent").textContent = b.current_now == null ? "—" : `${fmt(b.current_now, 0)} mA`;
  $("pPower").textContent = b.power_now == null ? "—" : `${fmt(b.power_now, 2)} W`;
  $("pTemp").textContent = b.temperature == null ? "—" : `${fmt(b.temperature, 1)} °C`;
}

function renderSources(sources) {
  const list = $("sourceList");
  list.replaceChildren();
  const online = sources.filter((s) => s.online === true);
  if (online.length === 0) {
    const hint = document.createElement("p");
    hint.className = "empty-hint";
    hint.textContent = t("source.empty.battery");
    list.appendChild(hint);
    return;
  }
  online.forEach((s) => {
    const icon = s.kind === "Wireless" ? "📡" : s.kind === "Mains" ? "⚡" : "🔌";
    const detail = `${fmt(s.voltage_now, 2)} V · ${fmt(s.current_now, 0)} mA${
      s.usb_type ? " · " + s.usb_type : ""
    }`;

    const item = document.createElement("div");
    item.className = "source-item";

    const iconEl = document.createElement("div");
    iconEl.className = "src-icon";
    iconEl.textContent = icon;

    const body = document.createElement("div");
    body.className = "src-body";
    const name = document.createElement("div");
    name.className = "src-name";
    name.textContent = sourceLabel(s.kind);
    const detailEl = document.createElement("div");
    detailEl.className = "src-detail";
    detailEl.textContent = detail;
    body.append(name, detailEl);

    const state = document.createElement("div");
    state.className = "src-state on";
    state.textContent = t("source.online");

    item.append(iconEl, body, state);
    list.appendChild(item);
  });
}

// ---------- 监测曲线（实时滚动 + 多档分时）----------
const METRICS = {
  cap:  { nameKey: "metric.cap", unit: "%",  decimals: 0, signed: false, clamp: [0, 100] },
  pow:  { nameKey: "metric.pow", unit: "W",  decimals: 2, signed: true },
  temp: { nameKey: "metric.temp", unit: "°C", decimals: 1, signed: false },
  volt: { nameKey: "metric.volt", unit: "V",  decimals: 3, signed: false },
  curr: { nameKey: "metric.curr", unit: "mA", decimals: 0, signed: true },
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
  $("chartMetricName").textContent = t(m.nameKey);
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
    lastSnapshot = snap;
    renderBattery(snap.battery);
    renderSources(snap.sources);
    if (snap.battery) pushBuffer(snap.battery, snap.timestamp_ms);
    // 监测页可见时跟随主轮询实时刷新曲线（短档平滑滚动，长档查后端历史）。
    if (isMonitorActive()) refreshChart();
    const d = new Date(snap.timestamp_ms);
    $("lastUpdate").textContent = t("footer.updated", { time: d.toLocaleTimeString(currentLanguage) });
  } catch (err) {
    $("statusText").textContent = t("status.readFailed");
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
  language: "zh-CN",
  theme: "system",
  autostart: false,
  silent_start: false,
  close_action: "ask",
  super_power_saver: false,
  remind_charge: true,
  remind_charge_at: 30,
  remind_unplug: true,
  remind_unplug_at: 80,
};
const closeActionLabel = (value) =>
  ({
    ask: t("close.ask"),
    tray: t("close.tray"),
    exit: t("close.exit"),
  }[value]);

function setCloseActionDropdownOpen(open) {
  $("closeActionDropdown").classList.toggle("open", open);
  $("setCloseActionButton").setAttribute("aria-expanded", String(open));
}

function setCloseActionValue(value, shouldPersist = false) {
  const next = closeActionLabel(value) ? value : "ask";
  settings.close_action = next;
  $("setCloseAction").value = next;
  $("setCloseActionLabel").textContent = closeActionLabel(next);
  document.querySelectorAll("#setCloseActionMenu [data-value]").forEach((item) => {
    item.setAttribute("aria-selected", String(item.dataset.value === next));
    item.textContent = closeActionLabel(item.dataset.value);
  });
  if (shouldPersist) persistSettings();
}

async function loadSettings() {
  try {
    settings = await invoke("get_settings");
    settings.language = I18N[settings.language] ? settings.language : "zh-CN";
    settings.theme = ["system", "light", "dark"].includes(settings.theme) ? settings.theme : "system";
    setSegmentedValue("setLanguage", settings.language);
    setSegmentedValue("setTheme", settings.theme);
    applyTheme(settings.theme);
    applyLanguage(settings.language);
    $("setAutostart").checked = settings.autostart;
    $("setSilentStart").checked = settings.silent_start;
    setCloseActionValue(settings.close_action);
    $("setRemindCharge").checked = settings.remind_charge;
    $("setRemindChargeAt").value = settings.remind_charge_at;
    $("setRemindUnplug").checked = settings.remind_unplug;
    $("setRemindUnplugAt").value = settings.remind_unplug_at;
    syncReminderInputs();
    refreshSuperPowerSaver();
  } catch (err) {
    console.error(err);
  }
}

const freqLabel = (khz) => {
  if (khz == null || isNaN(khz)) return "—";
  return khz >= 1000000 ? `${(khz / 1000000).toFixed(2)} GHz` : `${Math.round(khz / 1000)} MHz`;
};

const freqHzLabel = (hz) => {
  if (hz == null || isNaN(hz)) return "—";
  return hz >= 1000000000 ? `${(hz / 1000000000).toFixed(2)} GHz` : `${Math.round(hz / 1000000)} MHz`;
};

const powerCapsLabel = (state) => {
  const parts = [];
  if (state?.policies?.length) {
    parts.push(`CPU ${state.policies.map((p) => freqLabel(p.max_freq)).join(" / ")}`);
  }
  if (state?.gpus?.length) {
    parts.push(`GPU ${state.gpus.map((g) => freqHzLabel(g.max_freq)).join(" / ")}`);
  }
  return parts.length ? parts.join(" · ") : "—";
};

function setSuperPowerStatus(text, className = "power-mode-status", action = null) {
  const status = $("superPowerStatus");
  status.className = className;
  status.replaceChildren();

  const label = document.createElement("span");
  label.textContent = text;
  status.appendChild(label);

  if (action) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "power-mode-action";
    button.textContent = action.label;
    button.addEventListener("click", action.onClick);
    status.appendChild(button);
  }
}

function renderSuperPowerSaver(state, busy = false) {
  const toggle = $("setSuperPowerSaver");
  const desired = !!settings.super_power_saver;
  if (busy) {
    toggle.disabled = true;
    toggle.checked = desired;
    setSuperPowerStatus(t("settings.super.applying"));
    return;
  }
  if (!state?.supported) {
    lastCpuPowerState = state;
    toggle.checked = false;
    toggle.disabled = true;
    setSuperPowerStatus(t("settings.super.unsupported"), "power-mode-status warning");
    return;
  }
  toggle.disabled = false;
  toggle.checked = desired;
  lastCpuPowerState = state;
  const actual = !!state.active;
  const caps = powerCapsLabel(state);

  if (desired && actual) {
    setSuperPowerStatus(t("settings.super.active", { caps }), "power-mode-status active");
  } else if (desired && !actual) {
    setSuperPowerStatus(
      t("settings.super.wantOnMismatch", { caps }),
      "power-mode-status warning",
      {
        label: t("settings.super.reapply"),
        onClick: () => applySuperPowerSaver(true),
      }
    );
  } else if (!desired && actual) {
    setSuperPowerStatus(
      t("settings.super.wantOffMismatch", { caps }),
      "power-mode-status warning",
      {
        label: t("settings.super.restore"),
        onClick: () => applySuperPowerSaver(false),
      }
    );
  } else {
    setSuperPowerStatus(t("settings.super.inactive", { caps }));
  }
}

async function refreshSuperPowerSaver() {
  try {
    setSuperPowerStatus(t("settings.super.checking"));
    const state = await invoke("get_cpu_power_state");
    renderSuperPowerSaver(state);
  } catch (err) {
    setSuperPowerStatus(t("settings.super.error", { message: String(err) }), "power-mode-status warning");
  }
}

async function applySuperPowerSaver(enabled) {
  const previousDesired = !!settings.super_power_saver;
  settings.super_power_saver = enabled;
  renderSuperPowerSaver(lastCpuPowerState, true);
  try {
    const state = await invoke("set_super_power_saver", { enabled });
    renderSuperPowerSaver(state);
    persistSettings();
  } catch (err) {
    settings.super_power_saver = previousDesired;
    setSuperPowerStatus(t("settings.super.error", { message: String(err) }), "power-mode-status warning");
    await refreshSuperPowerSaver();
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
$("setSuperPowerSaver").addEventListener("change", async (e) => {
  applySuperPowerSaver(e.target.checked);
});

function wireSegmentedSetting(groupId, key, apply) {
  $(groupId).addEventListener("click", (e) => {
    const btn = e.target.closest("[data-setting-value]");
    if (!btn) return;
    const value = btn.dataset.settingValue;
    settings[key] = value;
    setSegmentedValue(groupId, value);
    apply(value);
    persistSettings();
  });
}

wireSegmentedSetting("setLanguage", "language", applyLanguage);
wireSegmentedSetting("setTheme", "theme", applyTheme);

// 阈值输入框失焦时夹取到合法范围并保存。
function commitThreshold(inputId, key, lo, hi) {
  const el = $(inputId);
  let v = parseInt(el.value, 10);
  if (isNaN(v)) v = settings[key];
  v = Math.min(hi, Math.max(lo, v));
  el.value = v;
  settings[key] = v;
  if (lastSnapshot?.battery) renderPowerPrediction(lastSnapshot.battery);
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
