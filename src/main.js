// Battery SipJuice — 前端逻辑
// 使用全局注入的 Tauri API（withGlobalTauri: true），无需打包器即可在 WebKitGTK 运行。
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const RING_CIRCUMFERENCE = 2 * Math.PI * 52; // 与 styles.css 的 r=52 一致
let currentLanguage = "zh-CN";
let lastSnapshot = null;
let lastCpuPowerState = null;

const translations = {
  "zh-CN": {
    "section.interface": "界面",
    "section.general": "常规",
    "section.energy": "能耗控制",
    "section.reminders": "电池养护提醒",
    "section.powerSource": "输入电源",
    "section.appPower": "应用耗电估算",
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
    "stat.timeRemaining": "剩余使用时间",
    "stat.power": "功率",
    "stat.temperature": "温度",
    "stat.health": "容量健康",
    "stat.cycles": "循环次数",
    "stat.fullCapacity": "实际满电容量",
    "stat.designCapacity": "设计容量",
    "prediction.title": "平均功率预测",
    "prediction.note": "按近 {window} 平均 {power} 估算，负载变化会让结果跟着变化。",
    "prediction.note.empty": "平均功率样本不足，暂时无法估算。",
    "prediction.full": "预计充满",
    "prediction.high": "到达 {threshold}% 以上",
    "prediction.low": "低于 {threshold}% 以下",
    "prediction.empty": "剩余使用时间",
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
    "stat.batteryVoltage": "电池电压",
    "stat.ocv": "开路电压(OCV)",
    "stat.vmax": "最大电压",
    "stat.currentNow": "瞬时电流",
    "stat.batteryCurrent": "电池电流",
    "stat.batteryPower": "电池功率",
    "stat.batteryTemperature": "电池温度",
    "stat.current": "当前",
    "stat.minimum": "区间最小",
    "stat.maximum": "区间最大",
    "stat.average": "区间平均",
    "health.note": "容量健康 = 当前满电容量 / 设计容量",
    "metric.battery.capacity": "电池电量",
    "metric.battery.power": "电池功率",
    "metric.battery.temperature": "电池温度",
    "metric.battery.voltage": "电池电压",
    "metric.battery.current": "电池电流",
    "metric.input.energy": "输入电量",
    "metric.input.power": "输入功率",
    "metric.input.temperature": "输入温度",
    "metric.input.voltage": "输入电压",
    "metric.input.current": "输入电流",
    "metric.unsupported": "不支持",
    "range.5m": "5 分",
    "range.30m": "30 分",
    "range.6h": "6 时",
    "range.24h": "24 时",
    "range.7d": "7 天",
    "chart.empty": "正在采集数据…曲线会随时间逐渐填充。",
    "chart.empty.battery": "正在采集电池数据…曲线会随时间逐渐填充。",
    "chart.empty.input": "未检测到可记录的输入源数据。",
    "chart.source.battery": "电池侧",
    "chart.source.input": "输入侧",
    "chart.inputSource": "输入来源",
    "chart.input.total": "总输入",
    "chart.chargingPeriod": "充电时段",
    "chart.onlinePeriod": "在线时段",
    "source.empty": "未检测到外接电源",
    "source.empty.battery": "未检测到外接电源（当前使用电池供电）",
    "appPower.hint": "按 CPU 占用时间估算，非硬件精确测量；累计值从本次应用启动开始计算。",
    "appPower.empty": "正在采集数据…",
    "appPower.totalEnergy": "本次运行累计耗电",
    "appPower.currentPower": "当前估算功率",
    "appPower.processCount": "{count} 个进程",
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
    "about.app": "应用",
    "about.version": "版本",
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
    "section.appPower": "Estimated App Power Usage",
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
    "stat.timeRemaining": "Remaining Use Time",
    "stat.power": "Power",
    "stat.temperature": "Temperature",
    "stat.health": "Capacity Health",
    "stat.cycles": "Cycle Count",
    "stat.fullCapacity": "Full Capacity",
    "stat.designCapacity": "Design Capacity",
    "prediction.title": "Average Power Forecast",
    "prediction.note": "Estimated from the last {window} average {power}. It will change as the load changes.",
    "prediction.note.empty": "Not enough average power samples yet, so no forecast is available.",
    "prediction.full": "Full",
    "prediction.high": "Above {threshold}%",
    "prediction.low": "Below {threshold}%",
    "prediction.empty": "Remaining Use Time",
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
    "stat.batteryVoltage": "Battery Voltage",
    "stat.ocv": "Open-Circuit Voltage",
    "stat.vmax": "Max Voltage",
    "stat.currentNow": "Instant Current",
    "stat.batteryCurrent": "Battery Current",
    "stat.batteryPower": "Battery Power",
    "stat.batteryTemperature": "Battery Temperature",
    "stat.current": "Current",
    "stat.minimum": "Range Min",
    "stat.maximum": "Range Max",
    "stat.average": "Range Avg",
    "health.note": "Capacity health = current full capacity / design capacity",
    "metric.battery.capacity": "Battery Charge",
    "metric.battery.power": "Battery Power",
    "metric.battery.temperature": "Battery Temperature",
    "metric.battery.voltage": "Battery Voltage",
    "metric.battery.current": "Battery Current",
    "metric.input.energy": "Input Energy",
    "metric.input.power": "Input Power",
    "metric.input.temperature": "Input Temperature",
    "metric.input.voltage": "Input Voltage",
    "metric.input.current": "Input Current",
    "metric.unsupported": "Unsupported",
    "range.5m": "5 min",
    "range.30m": "30 min",
    "range.6h": "6 h",
    "range.24h": "24 h",
    "range.7d": "7 days",
    "chart.empty": "Collecting data… the chart will fill in over time.",
    "chart.empty.battery": "Collecting battery data… the chart will fill in over time.",
    "chart.empty.input": "No recordable input source data detected.",
    "chart.source.battery": "Battery Side",
    "chart.source.input": "Input Side",
    "chart.inputSource": "Input Source",
    "chart.input.total": "Total Input",
    "chart.chargingPeriod": "Charging period",
    "chart.onlinePeriod": "Online period",
    "source.empty": "No external power detected",
    "source.empty.battery": "No external power detected (currently on battery)",
    "appPower.hint": "Estimated from CPU time share, not exact hardware measurement; totals start from this app launch.",
    "appPower.empty": "Collecting data…",
    "appPower.totalEnergy": "Total Energy This Run",
    "appPower.currentPower": "Current Estimated Power",
    "appPower.processCount": "{count} processes",
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
    "about.app": "App",
    "about.version": "Version",
    "modal.title": "Close Battery SipJuice",
    "modal.body": "Do you want to quit the app, or minimize it to the tray and keep monitoring?",
    "modal.remember": "Remember my choice and do not ask again",
    "modal.cancel": "Cancel",
    "modal.tray": "Minimize to Tray",
    "modal.quit": "Quit",
  },
};

const translate = (key, params = {}) => {
  let text = translations[currentLanguage]?.[key] ?? translations["en-US"]?.[key] ?? key;
  Object.entries(params).forEach(([name, value]) => {
    text = text.replace(`{${name}}`, value);
  });
  return text;
};

// ---------- 标签切换 ----------
document.querySelectorAll(".tab").forEach((tab) => {
  tab.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach((tabButton) => tabButton.classList.remove("active"));
    document.querySelectorAll(".panel").forEach((panel) => panel.classList.remove("active"));
    tab.classList.add("active");
    document.getElementById(tab.dataset.tab).classList.add("active");
  });
});

// ---------- 工具函数 ----------
const byId = (id) => document.getElementById(id);
const formatNumber = (value, decimals = 0) =>
  value == null || isNaN(value) ? "—" : Number(value).toFixed(decimals);
const formatValueWithUnit = (quantity, decimals = 0) =>
  quantity ? `${formatNumber(quantity.value, decimals)} ${quantity.unit}` : "—";

function applyTheme(theme) {
  const next = ["system", "light", "dark"].includes(theme) ? theme : "system";
  if (next === "system") document.documentElement.removeAttribute("data-theme");
  else document.documentElement.dataset.theme = next;
}

function setSegmentedValue(groupId, value) {
  const group = byId(groupId);
  group.querySelectorAll("[data-setting-value]").forEach((btn) => {
    const active = btn.dataset.settingValue === value;
    btn.classList.toggle("active", active);
    btn.setAttribute("aria-checked", String(active));
  });
}

function applyLanguage(language) {
  currentLanguage = translations[language] ? language : "zh-CN";
  document.documentElement.lang = currentLanguage;
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    el.textContent = translate(el.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-aria]").forEach((el) => {
    el.setAttribute("aria-label", translate(el.dataset.i18nAria));
  });
  renderChartSourceChips();
  renderMetricChips();
  renderInputSourceOptions();
  renderChartLegend();
  setCloseActionValue(settings.close_action);
  if (lastCpuPowerState) renderSuperPowerSaver(lastCpuPowerState);
  if (lastSnapshot) {
    renderBattery(lastSnapshot.battery);
    renderSources(lastSnapshot.sources);
    const date = new Date(lastSnapshot.timestamp_ms);
    byId("lastUpdate").textContent = translate("footer.updated", { time: date.toLocaleTimeString(currentLanguage) });
  }
  if (isMonitorActive()) refreshChart();
}

function sourceLabel(kind) {
  return (
    {
      USB: translate("source.usb"),
      USB_C: translate("source.usbc"),
      USB_PD: translate("source.usbpd"),
      Mains: translate("source.mains"),
      Wireless: translate("source.wireless"),
    }[kind] || kind || translate("source.fallback")
  );
}

function formatDuration(minutes) {
  if (!minutes || minutes <= 0) return "—";
  const hours = Math.floor(minutes / 60);
  const minutePart = minutes % 60;
  return hours > 0
    ? `${hours} ${translate("time.hour")} ${minutePart} ${translate("time.minute")}`
    : `${minutePart} ${translate("time.minute")}`;
}

function statusLabel(status) {
  return (
    {
      Charging: translate("status.charging"),
      Discharging: translate("status.discharging"),
      Full: translate("status.full"),
      "Not charging": translate("status.notCharging"),
      Unknown: translate("status.unknown"),
    }[status] || status || "—"
  );
}

function fullEnergyWh(battery) {
  const full = battery?.full_capacity;
  if (!full) return null;
  if (full.unit === "Wh") return full.value;
  if (full.unit === "mAh") {
    const voltage = battery.voltage_now ?? battery.voltage_ocv ?? battery.voltage_max;
    return voltage == null ? null : (full.value * voltage) / 1000;
  }
  return null;
}

const TIME_ESTIMATE_WINDOW_MS = 5 * 60_000;
const TIME_ESTIMATE_MIN_SAMPLES = 5;

function formatEstimateWindow(ms) {
  const minutes = Math.round(ms / 60_000);
  return currentLanguage === "zh-CN" ? `${minutes} 分钟` : `${minutes} min`;
}

function averagePowerForStatus(status) {
  const direction = status === "Charging" ? 1 : status === "Discharging" ? -1 : 0;
  if (!direction) return null;
  const cutoff = Date.now() - TIME_ESTIMATE_WINDOW_MS;
  const samples = batteryRtBuffer.filter((sample) => {
    if (sample.timestamp_ms < cutoff || sample.power_w == null) return false;
    return direction > 0 ? sample.power_w > 0 : sample.power_w < 0;
  });
  if (samples.length < TIME_ESTIMATE_MIN_SAMPLES) return null;
  const averagePowerW = samples.reduce((sum, sample) => sum + Math.abs(sample.power_w), 0) / samples.length;
  return averagePowerW >= 0.01 ? averagePowerW : null;
}

function minutesForPercentDelta(battery, percentDelta, averagePowerW) {
  const energyWh = fullEnergyWh(battery);
  if (energyWh == null || averagePowerW == null || averagePowerW < 0.01 || percentDelta <= 0) return null;
  return Math.round((energyWh * (percentDelta / 100) / averagePowerW) * 60);
}

function remainingUseMinutes(battery) {
  if (battery?.status !== "Discharging" || battery.capacity == null) return null;
  return minutesForPercentDelta(battery, Number(battery.capacity), averagePowerForStatus("Discharging"));
}

function renderPowerPrediction(battery) {
  const predictionIds = ["predictionFull", "predictionHigh", "predictionLow", "predictionEmpty"];
  const setAllEmpty = (note = translate("prediction.note.empty")) => {
    predictionIds.forEach((id) => (byId(id).textContent = "—"));
    byId("predictionNote").textContent = note;
  };

  const lowThreshold = Number(settings.remind_charge_at ?? 30);
  const highThreshold = Number(settings.remind_unplug_at ?? 80);
  byId("predictionHighLabel").textContent = translate("prediction.high", { threshold: highThreshold });
  byId("predictionLowLabel").textContent = translate("prediction.low", { threshold: lowThreshold });

  if (!battery || battery.capacity == null) {
    setAllEmpty();
    return;
  }

  const averagePowerW = averagePowerForStatus(battery.status);
  if (averagePowerW == null || fullEnergyWh(battery) == null) {
    setAllEmpty();
    return;
  }

  const capacity = Number(battery.capacity);
  predictionIds.forEach((id) => (byId(id).textContent = "—"));
  byId("predictionNote").textContent = translate("prediction.note", {
    window: formatEstimateWindow(TIME_ESTIMATE_WINDOW_MS),
    power: `${formatNumber(averagePowerW, 2)} W`,
  });

  if (battery.status === "Charging") {
    byId("predictionFull").textContent = formatDuration(minutesForPercentDelta(battery, 100 - capacity, averagePowerW));
    byId("predictionHigh").textContent =
      capacity >= highThreshold
        ? translate("prediction.reached.high", { threshold: highThreshold })
        : formatDuration(minutesForPercentDelta(battery, highThreshold - capacity, averagePowerW));
  } else if (battery.status === "Discharging") {
    byId("predictionLow").textContent =
      capacity <= lowThreshold
        ? translate("prediction.reached.low", { threshold: lowThreshold })
        : formatDuration(minutesForPercentDelta(battery, capacity - lowThreshold, averagePowerW));
    byId("predictionEmpty").textContent = formatDuration(remainingUseMinutes(battery));
  } else if (battery.status === "Full") {
    byId("predictionFull").textContent = statusLabel("Full");
    byId("predictionHigh").textContent = translate("prediction.reached.high", { threshold: highThreshold });
  }
}

// ---------- 渲染 ----------
function renderBattery(battery) {
  if (!battery) {
    byId("statusText").textContent = translate("status.noBattery");
    renderPowerPrediction(null);
    return;
  }

  // 状态条
  byId("modelName").textContent = battery.model || battery.manufacturer || battery.device;
  const pill = byId("statusPill");
  pill.className = "status-pill";
  if (battery.status === "Charging") pill.classList.add("charging");
  else if (battery.status === "Discharging") pill.classList.add("discharging");
  else if (battery.status === "Full") pill.classList.add("full");
  byId("statusText").textContent = statusLabel(battery.status);

  // 环形进度
  byId("capacity").textContent = battery.capacity ?? "--";
  const ring = byId("ring");
  const percentage = battery.capacity ?? 0;
  ring.style.strokeDashoffset = RING_CIRCUMFERENCE * (1 - percentage / 100);
  ring.style.stroke = percentage <= 15 ? "var(--bad)" : percentage <= 30 ? "var(--warn)" : "var(--accent)";

  // hero 元信息
  byId("heroStatus").textContent = statusLabel(battery.status);
  byId("heroTimeRemaining").textContent = formatDuration(remainingUseMinutes(battery));
  byId("heroPower").textContent = battery.power_now == null ? "—" : `${formatNumber(battery.power_now, 2)} W`;
  byId("heroTemperature").textContent = battery.temperature == null ? "—" : `${formatNumber(battery.temperature, 1)} °C`;

  // 概览摘要（仪表盘：寿命 + 容量，原始电气读数归监测页）
  byId("overviewHealth").textContent = `${formatNumber(battery.health_percent, 1)} %`;
  byId("overviewCycles").textContent = battery.cycle_count ?? "—";
  byId("overviewFullCapacity").textContent = formatValueWithUnit(battery.full_capacity, 0);
  byId("overviewDesignCapacity").textContent = formatValueWithUnit(battery.design_capacity, 0);

  // 健康页
  byId("healthScore").innerHTML = `${formatNumber(battery.health_percent, 1)}<small>%</small>`;
  byId("healthBar").style.width = `${Math.min(battery.health_percent ?? 0, 100)}%`;
  byId("healthFullCapacity").textContent = formatValueWithUnit(battery.full_capacity, 0);
  byId("healthDesignCapacity").textContent = formatValueWithUnit(battery.design_capacity, 0);
  const lostCapacity = battery.full_capacity && battery.design_capacity && battery.full_capacity.unit === battery.design_capacity.unit
    ? { value: battery.design_capacity.value - battery.full_capacity.value, unit: battery.full_capacity.unit }
    : null;
  byId("healthCapacityLost").textContent = lostCapacity ? `${formatNumber(lostCapacity.value, 0)} ${lostCapacity.unit}` : "—";
  byId("healthCycles").textContent = battery.cycle_count ?? "—";
  byId("healthCondition").textContent = battery.health_status || "—";
  byId("healthStateOfHealth").textContent = battery.state_of_health == null ? "—" : `${battery.state_of_health} %`;
  byId("healthTechnology").textContent = battery.technology || "—";
  byId("healthResistance").textContent =
    battery.internal_resistance == null ? "—" : `${formatNumber(battery.internal_resistance, 0)} mΩ`;
  renderPowerPrediction(battery);

  // 监测页（电池侧实时电气量）
  byId("monitorVoltage").textContent = battery.voltage_now == null ? "—" : `${formatNumber(battery.voltage_now, 3)} V`;
  byId("monitorOpenCircuitVoltage").textContent = battery.voltage_ocv == null ? "—" : `${formatNumber(battery.voltage_ocv, 3)} V`;
  byId("monitorMaxVoltage").textContent = battery.voltage_max == null ? "—" : `${formatNumber(battery.voltage_max, 3)} V`;
  byId("monitorCurrent").textContent = battery.current_now == null ? "—" : `${formatNumber(battery.current_now, 0)} mA`;
  byId("monitorPower").textContent = battery.power_now == null ? "—" : `${formatNumber(battery.power_now, 2)} W`;
  byId("monitorTemperature").textContent = battery.temperature == null ? "—" : `${formatNumber(battery.temperature, 1)} °C`;
}

function renderSources(sources) {
  const list = byId("sourceList");
  list.replaceChildren();
  const onlineSources = sources.filter((source) => source.online === true);
  if (onlineSources.length === 0) {
    const hint = document.createElement("p");
    hint.className = "empty-hint";
    hint.textContent = translate("source.empty.battery");
    list.appendChild(hint);
    return;
  }
  onlineSources.forEach((source) => {
    const icon = source.kind === "Wireless" ? "📡" : source.kind === "Mains" ? "⚡" : "🔌";
    const detail = `${formatNumber(source.power_now, 2)} W · ${formatNumber(source.voltage_now, 2)} V · ${formatNumber(source.current_now, 0)} mA${
      source.usb_type ? " · " + source.usb_type : ""
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
    name.textContent = sourceLabel(source.kind);
    const detailEl = document.createElement("div");
    detailEl.className = "src-detail";
    detailEl.textContent = detail;
    body.append(name, detailEl);

    const state = document.createElement("div");
    state.className = "src-state on";
    state.textContent = translate("source.online");

    item.append(iconEl, body, state);
    list.appendChild(item);
  });
}

function formatEnergy(wh) {
  if (wh == null || isNaN(wh)) return "—";
  return wh >= 1 ? `${formatNumber(wh, 2)} Wh` : `${formatNumber(wh * 1000, 1)} mWh`;
}

function appPowerSubtitle(app) {
  return app.process_count > 1 ? translate("appPower.processCount", { count: app.process_count }) : "";
}

function renderAppPowerRows(listId, apps, metric, valueLabel, maxValue) {
  const list = byId(listId);
  list.replaceChildren();
  if (!apps || apps.length === 0) {
    const hint = document.createElement("p");
    hint.className = "empty-hint";
    hint.textContent = translate("appPower.empty");
    list.appendChild(hint);
    return;
  }
  const scale = Math.max(maxValue, 0.0001);
  apps.forEach((app) => {
    const item = document.createElement("div");
    item.className = "app-power-item";

    const body = document.createElement("div");
    body.className = "app-power-body";
    const name = document.createElement("div");
    name.className = "app-power-name";
    name.textContent = app.name || "—";
    name.title = app.name || "";
    const sub = document.createElement("div");
    sub.className = "app-power-sub";
    sub.textContent = appPowerSubtitle(app);
    const bar = document.createElement("div");
    bar.className = "app-power-bar";
    const barFill = document.createElement("div");
    barFill.className = "app-power-bar-fill";
    barFill.style.width = `${Math.min(100, (app[metric] / scale) * 100)}%`;
    bar.appendChild(barFill);
    body.append(name);
    if (sub.textContent) body.appendChild(sub);
    body.appendChild(bar);

    const value = document.createElement("div");
    value.className = "app-power-value";
    value.textContent = valueLabel(app);

    item.append(body, value);
    list.appendChild(item);
  });
}

function renderAppPowerReport(report) {
  const totalEnergy = report?.total_energy ?? [];
  const currentPower = report?.current_power ?? [];
  const maxEnergy = Math.max(...totalEnergy.map((app) => app.energy_wh), 0.0001);
  const maxPower = Math.max(...currentPower.map((app) => app.power_w), 0.0001);
  renderAppPowerRows("appEnergyList", totalEnergy, "energy_wh", (app) => formatEnergy(app.energy_wh), maxEnergy);
  renderAppPowerRows("appPowerList", currentPower, "power_w", (app) => `${formatNumber(app.power_w, 2)} W`, maxPower);
}

// ---------- 监测曲线（实时滚动 + 多档分时）----------
const metricConfigs = {
  battery: {
    capacity: { nameKey: "metric.battery.capacity", unit: "%", decimals: 0, signed: false, clamp: [0, 100] },
    power_w: { nameKey: "metric.battery.power", unit: "W", decimals: 2, signed: true },
    temperature: { nameKey: "metric.battery.temperature", unit: "°C", decimals: 1, signed: false },
    voltage: { nameKey: "metric.battery.voltage", unit: "V", decimals: 3, signed: false },
    current_ma: { nameKey: "metric.battery.current", unit: "mA", decimals: 0, signed: true },
  },
  input: {
    energy_wh: { nameKey: "metric.input.energy", unit: "Wh", decimals: 3, signed: false, formatter: formatEnergy },
    power_w: { nameKey: "metric.input.power", unit: "W", decimals: 2, signed: false },
    temperature: { nameKey: "metric.input.temperature", unit: "", decimals: 1, signed: false, disabled: true },
    voltage: { nameKey: "metric.input.voltage", unit: "V", decimals: 3, signed: false },
    current_ma: { nameKey: "metric.input.current", unit: "mA", decimals: 0, signed: false },
  },
};
const metricOrder = {
  battery: ["capacity", "power_w", "temperature", "voltage", "current_ma"],
  input: ["energy_wh", "power_w", "temperature", "voltage", "current_ma"],
};

const monitorState = { sourceKind: "battery", metric: "capacity", rangeMs: 300000, inputSourceId: "total" };
// 仅 5 分档从前端实时缓冲区绘制(2 秒一帧平滑滚动)；≥30 分一律查后端 RRD 归档
// (30s/5min 粒度足够，且能显示打开软件之前的历史)。
const LIVE_MAX_MS = 300000;   // ≤ 此范围用实时缓冲
const BUFFER_MAX_MS = 600000; // 缓冲保留 10 分钟，为 5 分窗口留 2× 余量
const HISTORY_FINE_STEP_MS = 30_000;
const HISTORY_COARSE_STEP_MS = 300_000;
const HISTORY_MAX_POINTS = 240;
const batteryRtBuffer = [];
const inputRtBuffers = new Map();
const knownInputSources = new Map();
let lastSamples = [];

const isMonitorActive = () => byId("monitor").classList.contains("active");

// 把一帧快照转成与后端 Sample 一致的样本（功率带符号：放电为负）。
function pushBatteryBuffer(battery, timestampMs) {
  const isCharging = battery.status === "Charging";
  const powerMagnitude = battery.power_now == null ? null : Math.abs(battery.power_now);
  batteryRtBuffer.push({
    timestamp_ms: timestampMs,
    capacity: battery.capacity ?? null,
    temperature: battery.temperature ?? null,
    power_w: powerMagnitude == null ? null : isCharging ? powerMagnitude : -powerMagnitude,
    voltage: battery.voltage_now ?? null,
    current_ma: battery.current_now ?? null,
    charging: isCharging,
  });
  const cutoff = timestampMs - BUFFER_MAX_MS;
  while (batteryRtBuffer.length && batteryRtBuffer[0].timestamp_ms < cutoff) batteryRtBuffer.shift();
}

function absOrNull(value) {
  return value == null ? null : Math.abs(Number(value));
}

function sumPresent(values) {
  const present = values.filter((value) => value != null && !isNaN(value));
  return present.length ? present.reduce((sum, value) => sum + value, 0) : null;
}

function averagePresent(values) {
  const present = values.filter((value) => value != null && !isNaN(value));
  return present.length ? present.reduce((sum, value) => sum + value, 0) / present.length : null;
}

function inputSample(source, timestampMs) {
  return {
    timestamp_ms: timestampMs,
    capacity: null,
    temperature: null,
    power_w: absOrNull(source.power_now),
    voltage: source.voltage_now ?? null,
    current_ma: absOrNull(source.current_now),
    charging: true,
  };
}

function totalInputSample(sources, timestampMs) {
  const onlineSources = (sources ?? []).filter((source) => source.online === true);
  if (!onlineSources.length) return null;
  return {
    timestamp_ms: timestampMs,
    capacity: null,
    temperature: null,
    power_w: sumPresent(onlineSources.map((source) => absOrNull(source.power_now))),
    voltage: averagePresent(onlineSources.map((source) => source.voltage_now ?? null)),
    current_ma: sumPresent(onlineSources.map((source) => absOrNull(source.current_now))),
    charging: true,
  };
}

function pushLimitedBuffer(buffer, sample, timestampMs) {
  buffer.push(sample);
  const cutoff = timestampMs - BUFFER_MAX_MS;
  while (buffer.length && buffer[0].timestamp_ms < cutoff) buffer.shift();
}

function inputBuffer(sourceId) {
  const key = sourceId || "total";
  if (!inputRtBuffers.has(key)) inputRtBuffers.set(key, []);
  return inputRtBuffers.get(key);
}

function rememberInputSources(sources) {
  (sources ?? []).forEach((source) => {
    if (source?.name) knownInputSources.set(source.name, source);
  });
}

function pushInputBuffers(sources, timestampMs) {
  rememberInputSources(sources);
  const totalSample = totalInputSample(sources, timestampMs);
  if (totalSample) pushLimitedBuffer(inputBuffer("total"), totalSample, timestampMs);

  (sources ?? [])
    .filter((source) => source.online === true)
    .forEach((source) => {
      pushLimitedBuffer(inputBuffer(source.name), inputSample(source, timestampMs), timestampMs);
    });
}

// 启动时用后端 RRD 归档预填实时缓冲区，使 5 分档一打开就能显示打开软件之前的数据。
// 后端为 30s 粒度的历史，随后由实时 tick 追加 2s 粒度的新点；二者按时间天然衔接。
async function seedBuffer(sourceKind = "battery", inputSourceId = "total") {
  try {
    const args = { rangeMs: BUFFER_MAX_MS, sourceKind };
    if (sourceKind === "input") args.inputSourceId = inputSourceId;
    const hist = await invoke("get_history", args);
    if (Array.isArray(hist) && hist.length) {
      // 仅插入早于当前缓冲最早点的历史，避免与已采集的实时点重叠/乱序。
      const buffer = sourceKind === "input" ? inputBuffer(inputSourceId) : batteryRtBuffer;
      const earliest = buffer.length ? buffer[0].timestamp_ms : Infinity;
      const older = hist.filter((sample) => sample.timestamp_ms < earliest);
      if (older.length) buffer.unshift(...older);
    }
  } catch (err) {
    console.error("预填历史失败:", err);
  }
}

function formatAxisTime(ms, rangeMs) {
  const date = new Date(ms);
  const pad2 = (value) => String(value).padStart(2, "0");
  if (rangeMs >= 604800000) return `${date.getMonth() + 1}/${date.getDate()}`;
  if (rangeMs >= 86400000) return `${date.getMonth() + 1}/${date.getDate()} ${pad2(date.getHours())}h`;
  return `${pad2(date.getHours())}:${pad2(date.getMinutes())}`;
}

function currentMetricConfig() {
  return metricConfigs[monitorState.sourceKind][monitorState.metric];
}

function normalizeMetricForSourceKind(sourceKind, metric) {
  if (sourceKind === "input") {
    if (metric === "capacity") return "energy_wh";
    if (metric === "temperature") return "power_w";
    const config = metricConfigs.input[metric];
    return config && !config.disabled ? metric : "power_w";
  }
  if (metric === "energy_wh") return "capacity";
  return metricConfigs.battery[metric] ? metric : "capacity";
}

function renderChartSourceChips() {
  document.querySelectorAll("#chartSourceChips [data-source-kind]").forEach((btn) => {
    const sourceKind = btn.dataset.sourceKind;
    btn.textContent = translate(`chart.source.${sourceKind}`);
    btn.classList.toggle("active", sourceKind === monitorState.sourceKind);
  });
}

function metricLabel(config) {
  return translate(config.nameKey);
}

function metricButtonText(config) {
  const suffix = config.disabled ? translate("metric.unsupported") : config.unit;
  return [metricLabel(config), suffix].filter(Boolean).join(" ");
}

function renderMetricChips() {
  const row = byId("metricChips");
  row.replaceChildren();
  for (const metric of metricOrder[monitorState.sourceKind]) {
    const config = metricConfigs[monitorState.sourceKind][metric];
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "chip";
    btn.dataset.metric = metric;
    btn.textContent = metricButtonText(config);
    btn.disabled = !!config.disabled;
    btn.classList.toggle("active", metric === monitorState.metric);
    row.appendChild(btn);
  }
}

function inputSourceLabel(source) {
  return `${sourceLabel(source.kind)} · ${source.name}`;
}

function renderInputSourceOptions() {
  const picker = byId("inputSourcePicker");
  const select = byId("inputSourceSelect");
  picker.hidden = monitorState.sourceKind !== "input";
  select.replaceChildren();

  const total = document.createElement("option");
  total.value = "total";
  total.textContent = translate("chart.input.total");
  select.appendChild(total);

  [...knownInputSources.values()]
    .sort((a, b) => a.name.localeCompare(b.name))
    .forEach((source) => {
      const option = document.createElement("option");
      option.value = source.name;
      option.textContent = inputSourceLabel(source);
      select.appendChild(option);
    });

  const hasSelected = [...select.options].some((option) => option.value === monitorState.inputSourceId);
  if (!hasSelected) monitorState.inputSourceId = "total";
  select.value = monitorState.inputSourceId;
}

function renderChartLegend() {
  byId("chartBandLabel").textContent = translate(
    monitorState.sourceKind === "input" ? "chart.onlinePeriod" : "chart.chargingPeriod"
  );
  byId("chartMetricName").textContent = metricLabel(currentMetricConfig());
}

function selectedLiveBuffer() {
  return monitorState.sourceKind === "input"
    ? inputBuffer(monitorState.inputSourceId)
    : batteryRtBuffer;
}

function historyArgs(rangeMs) {
  const args = { rangeMs, sourceKind: monitorState.sourceKind };
  if (monitorState.sourceKind === "input") args.inputSourceId = monitorState.inputSourceId;
  return args;
}

function continuityGapThreshold() {
  const sourceStep = monitorState.rangeMs > 86400000 ? HISTORY_COARSE_STEP_MS : HISTORY_FINE_STEP_MS;
  const effectiveStep = Math.max(sourceStep, monitorState.rangeMs / HISTORY_MAX_POINTS);
  return effectiveStep * 2.5;
}

function withInputEnergy(samples) {
  if (monitorState.sourceKind !== "input") return samples;
  const gapThreshold = continuityGapThreshold();
  let energyWh = 0;
  let hasEnergy = false;
  return samples.map((sample, index) => {
    const next = { ...sample };
    if (index === 0) {
      hasEnergy = sample.power_w != null;
      next.energy_wh = hasEnergy ? 0 : null;
      return next;
    }

    const prev = samples[index - 1];
    const elapsedMs = sample.timestamp_ms - prev.timestamp_ms;
    if (
      elapsedMs > 0 &&
      elapsedMs <= gapThreshold &&
      prev.power_w != null &&
      sample.power_w != null
    ) {
      energyWh += ((Math.abs(prev.power_w) + Math.abs(sample.power_w)) / 2) * (elapsedMs / 3_600_000);
      hasEnergy = true;
    } else if (sample.power_w != null) {
      hasEnergy = true;
    }
    next.energy_wh = hasEnergy ? energyWh : null;
    return next;
  });
}

function formatMetricValue(value, config) {
  return config.formatter ? config.formatter(value) : `${formatNumber(value, config.decimals)} ${config.unit}`;
}

async function refreshChart() {
  const rangeMs = monitorState.rangeMs;
  let samples;
  if (rangeMs <= LIVE_MAX_MS) {
    const cutoff = Date.now() - rangeMs;
    samples = selectedLiveBuffer().filter((sample) => sample.timestamp_ms >= cutoff);
  } else {
    try {
      samples = await invoke("get_history", historyArgs(rangeMs));
    } catch (err) {
      console.error(err);
      return;
    }
  }
  renderChart(withInputEnergy(samples));
}

function renderChart(samples) {
  const metricConfig = currentMetricConfig();
  renderChartLegend();
  const values = samples.map((sample) => sample[monitorState.metric]).filter((value) => value != null);

  if (samples.length < 2 || values.length === 0) {
    byId("chartSvg").innerHTML = "";
    byId("chartEmpty").textContent = translate(
      monitorState.sourceKind === "input" ? "chart.empty.input" : "chart.empty.battery"
    );
    byId("chartEmpty").style.display = "block";
    ["chartCurrent", "chartMinimum", "chartMaximum", "chartAverage"].forEach((id) => (byId(id).textContent = "—"));
    lastSamples = [];
    return;
  }
  byId("chartEmpty").style.display = "none";
  lastSamples = samples;
  drawChart(samples, monitorState.metric);

  byId("chartCurrent").textContent = formatMetricValue(values[values.length - 1], metricConfig);
  byId("chartMinimum").textContent = formatMetricValue(Math.min(...values), metricConfig);
  byId("chartMaximum").textContent = formatMetricValue(Math.max(...values), metricConfig);
  byId("chartAverage").textContent = formatMetricValue(
    values.reduce((sum, value) => sum + value, 0) / values.length,
    metricConfig
  );
}

function drawChart(samples, metric) {
  const metricConfig = currentMetricConfig();
  const svg = byId("chartSvg");
  const width = svg.clientWidth || 600;
  const height = svg.clientHeight || 240;
  svg.setAttribute("viewBox", `0 0 ${width} ${height}`);

  const paddingLeft = 46;
  const paddingRight = 14;
  const paddingTop = 14;
  const paddingBottom = 24;
  const plotWidth = width - paddingLeft - paddingRight;
  const plotHeight = height - paddingTop - paddingBottom;

  const timestamps = samples.map((sample) => sample.timestamp_ms);
  // 时间轴：右边缘钉在"现在"，窗口宽度固定为所选档位，曲线随时间往左滚动。
  const timeMax = Date.now();
  const timeMin = timeMax - monitorState.rangeMs;
  const timeSpan = Math.max(1, timeMax - timeMin);
  const values = samples.map((s) => s[metric]);
  const presentValues = values.filter((value) => value != null);

  let valueMin = Math.min(...presentValues);
  let valueMax = Math.max(...presentValues);
  if (metricConfig.signed) {
    valueMin = Math.min(valueMin, 0);
    valueMax = Math.max(valueMax, 0);
  }
  if (valueMin === valueMax) {
    valueMin -= 1;
    valueMax += 1;
  }
  const valuePadding = (valueMax - valueMin) * 0.1;
  valueMin -= valuePadding;
  valueMax += valuePadding;
  if (metricConfig.clamp) {
    valueMin = Math.max(metricConfig.clamp[0], valueMin);
    valueMax = Math.min(metricConfig.clamp[1], valueMax);
  }

  const xForTime = (time) => paddingLeft + ((time - timeMin) / timeSpan) * plotWidth;
  const yForValue = (value) => paddingTop + ((valueMax - value) / (valueMax - valueMin)) * plotHeight;
  const gapThreshold = continuityGapThreshold();

  // 充电时段背景带
  let chargingBands = "";
  for (let i = 0; i < samples.length - 1; i++) {
    if (samples[i].charging && timestamps[i + 1] - timestamps[i] <= gapThreshold) {
      const startX = xForTime(timestamps[i]);
      const endX = xForTime(timestamps[i + 1]);
      chargingBands += `<rect x="${startX.toFixed(1)}" y="${paddingTop}" width="${Math.max(0.5, endX - startX).toFixed(1)}" height="${plotHeight}" class="chg-band"/>`;
    }
  }

  // 网格线 + Y 轴标签
  let gridLines = "";
  for (const level of [valueMax, (valueMax + valueMin) / 2, valueMin]) {
    const y = yForValue(level);
    gridLines += `<line x1="${paddingLeft}" y1="${y.toFixed(1)}" x2="${width - paddingRight}" y2="${y.toFixed(1)}" class="grid-line"/>`;
    gridLines += `<text x="${paddingLeft - 6}" y="${(y + 3.5).toFixed(1)}" text-anchor="end" class="axis-label">${formatNumber(level, metricConfig.decimals)}</text>`;
  }

  // 0 基线（带符号指标）
  let zeroLine = "";
  if (metricConfig.signed && valueMin < 0 && valueMax > 0) {
    const zeroY = yForValue(0).toFixed(1);
    zeroLine = `<line x1="${paddingLeft}" y1="${zeroY}" x2="${width - paddingRight}" y2="${zeroY}" class="zero-line"/>`;
  }

  // 连续段（遇 null 断开）
  const segments = [];
  let segment = [];
  for (let i = 0; i < samples.length; i++) {
    const disconnected = i > 0 && timestamps[i] - timestamps[i - 1] > gapThreshold;
    if (values[i] == null || disconnected) {
      if (segment.length) {
        segments.push(segment);
        segment = [];
      }
      if (values[i] == null) continue;
    }
    segment.push([xForTime(timestamps[i]), yForValue(values[i])]);
  }
  if (segment.length) segments.push(segment);

  const baseY = metricConfig.signed && valueMin < 0 && valueMax > 0 ? yForValue(0) : paddingTop + plotHeight;
  const pointPath = (point) => `${point[0].toFixed(1)} ${point[1].toFixed(1)}`;
  const linePath = segments.map((points) => "M" + points.map(pointPath).join(" L")).join(" ");
  const areaPath = segments
    .map((points) => `M${points[0][0].toFixed(1)} ${baseY.toFixed(1)} L${points.map(pointPath).join(" L")} L${points[points.length - 1][0].toFixed(1)} ${baseY.toFixed(1)} Z`)
    .join(" ");

  // X 轴时间标签
  let xAxisLabels = "";
  for (let i = 0; i <= 4; i++) {
    const labelTime = timeMin + (timeSpan * i) / 4;
    const anchor = i === 0 ? "start" : i === 4 ? "end" : "middle";
    xAxisLabels += `<text x="${xForTime(labelTime).toFixed(1)}" y="${height - 7}" text-anchor="${anchor}" class="axis-label">${formatAxisTime(labelTime, monitorState.rangeMs)}</text>`;
  }

  const defs = `<defs><linearGradient id="area-gradient" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="var(--accent)" stop-opacity="0.28"/>
    <stop offset="100%" stop-color="var(--accent)" stop-opacity="0.02"/>
  </linearGradient></defs>`;

  svg.innerHTML =
    defs + chargingBands + gridLines + zeroLine +
    `<path d="${areaPath}" class="chart-area-fill"/>` +
    `<path d="${linePath}" class="chart-line"/>` +
    xAxisLabels;
}

byId("chartSourceChips").addEventListener("click", (event) => {
  const btn = event.target.closest("[data-source-kind]");
  if (!btn) return;
  monitorState.sourceKind = btn.dataset.sourceKind;
  monitorState.metric = normalizeMetricForSourceKind(monitorState.sourceKind, monitorState.metric);
  renderChartSourceChips();
  renderMetricChips();
  renderInputSourceOptions();
  renderChartLegend();
  refreshChart();
  if (monitorState.sourceKind === "input") {
    seedBuffer("input", monitorState.inputSourceId).then(refreshChart);
  }
});

byId("metricChips").addEventListener("click", (event) => {
  const btn = event.target.closest("[data-metric]");
  if (!btn || btn.disabled) return;
  monitorState.metric = btn.dataset.metric;
  renderMetricChips();
  renderChartLegend();
  refreshChart();
});

byId("rangeChips").addEventListener("click", (event) => {
  const btn = event.target.closest("[data-range]");
  if (!btn) return;
  [...byId("rangeChips").children].forEach((child) => child.classList.remove("active"));
  btn.classList.add("active");
  monitorState.rangeMs = parseInt(btn.dataset.range, 10);
  refreshChart();
});

byId("inputSourceSelect").addEventListener("change", (event) => {
  monitorState.inputSourceId = event.target.value || "total";
  seedBuffer("input", monitorState.inputSourceId).then(refreshChart);
});

renderChartSourceChips();
renderMetricChips();
renderInputSourceOptions();
renderChartLegend();

// 切到监测标签时立即刷新；窗口缩放时重绘
document.querySelectorAll(".tab").forEach((tab) => {
  if (tab.dataset.tab === "monitor") {
    tab.addEventListener("click", () => {
      refreshChart();
      tickAppPowerReport();
    });
  }
});
window.addEventListener("resize", () => {
  if (isMonitorActive() && lastSamples.length) drawChart(lastSamples, monitorState.metric);
});

// ---------- 轮询 ----------
async function tick() {
  try {
    const snap = await invoke("get_snapshot");
    lastSnapshot = snap;
    if (snap.battery) pushBatteryBuffer(snap.battery, snap.timestamp_ms);
    pushInputBuffers(snap.sources, snap.timestamp_ms);
    renderBattery(snap.battery);
    renderSources(snap.sources);
    renderInputSourceOptions();
    // 监测页可见时跟随主轮询实时刷新曲线（短档平滑滚动，长档查后端历史）。
    if (isMonitorActive()) refreshChart();
    const date = new Date(snap.timestamp_ms);
    byId("lastUpdate").textContent = translate("footer.updated", { time: date.toLocaleTimeString(currentLanguage) });
  } catch (err) {
    byId("statusText").textContent = translate("status.readFailed");
    console.error(err);
  }
}

tick();
setInterval(tick, 2000);

// 按应用耗电估算依赖后台线程按 SAMPLE_INTERVAL(30s) 采样的 CPU 时间差，
// 轮询过密没有意义；只在监测页可见时按较长间隔查询。
async function tickAppPowerReport() {
  if (!isMonitorActive()) return;
  try {
    const report = await invoke("get_app_power_report");
    renderAppPowerReport(report);
  } catch (err) {
    console.error(err);
  }
}
tickAppPowerReport();
setInterval(tickAppPowerReport, 8000);

// 预填历史缓冲，完成后若监测页可见则立即重绘短档曲线。
Promise.all([seedBuffer("battery"), seedBuffer("input", "total")]).then(() => {
  if (isMonitorActive()) refreshChart();
});

async function loadAppVersion() {
  try {
    byId("aboutVersion").textContent = await invoke("get_app_version");
  } catch (err) {
    console.error("读取版本号失败:", err);
  }
}
loadAppVersion();

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
    ask: translate("close.ask"),
    tray: translate("close.tray"),
    exit: translate("close.exit"),
  }[value]);

function setCloseActionDropdownOpen(open) {
  byId("closeActionDropdown").classList.toggle("open", open);
  byId("setCloseActionButton").setAttribute("aria-expanded", String(open));
}

function setCloseActionValue(value, shouldPersist = false) {
  const next = closeActionLabel(value) ? value : "ask";
  settings.close_action = next;
  byId("setCloseAction").value = next;
  byId("setCloseActionLabel").textContent = closeActionLabel(next);
  document.querySelectorAll("#setCloseActionMenu [data-value]").forEach((item) => {
    item.setAttribute("aria-selected", String(item.dataset.value === next));
    item.textContent = closeActionLabel(item.dataset.value);
  });
  if (shouldPersist) persistSettings();
}

async function loadSettings() {
  try {
    settings = await invoke("get_settings");
    settings.language = translations[settings.language] ? settings.language : "zh-CN";
    settings.theme = ["system", "light", "dark"].includes(settings.theme) ? settings.theme : "system";
    setSegmentedValue("setLanguage", settings.language);
    setSegmentedValue("setTheme", settings.theme);
    applyTheme(settings.theme);
    applyLanguage(settings.language);
    byId("setAutostart").checked = settings.autostart;
    byId("setSilentStart").checked = settings.silent_start;
    setCloseActionValue(settings.close_action);
    byId("setRemindCharge").checked = settings.remind_charge;
    byId("setRemindChargeAt").value = settings.remind_charge_at;
    byId("setRemindUnplug").checked = settings.remind_unplug;
    byId("setRemindUnplugAt").value = settings.remind_unplug_at;
    syncReminderInputs();
    refreshSuperPowerSaver();
  } catch (err) {
    console.error(err);
  }
}

const formatKilohertz = (kilohertz) => {
  if (kilohertz == null || isNaN(kilohertz)) return "—";
  return kilohertz >= 1000000
    ? `${(kilohertz / 1000000).toFixed(2)} GHz`
    : `${Math.round(kilohertz / 1000)} MHz`;
};

const formatHertz = (hertz) => {
  if (hertz == null || isNaN(hertz)) return "—";
  return hertz >= 1000000000
    ? `${(hertz / 1000000000).toFixed(2)} GHz`
    : `${Math.round(hertz / 1000000)} MHz`;
};

const frequencyLimitSummary = (state) => {
  const parts = [];
  if (state?.policies?.length) {
    parts.push(`CPU ${state.policies.map((policy) => formatKilohertz(policy.max_freq)).join(" / ")}`);
  }
  if (state?.gpus?.length) {
    parts.push(`GPU ${state.gpus.map((gpu) => formatHertz(gpu.max_freq)).join(" / ")}`);
  }
  return parts.length ? parts.join(" · ") : "—";
};

function setSuperPowerStatus(text, className = "power-mode-status", action = null) {
  const status = byId("superPowerStatus");
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
  const toggle = byId("setSuperPowerSaver");
  const desired = !!settings.super_power_saver;
  if (busy) {
    toggle.disabled = true;
    toggle.checked = desired;
    setSuperPowerStatus(translate("settings.super.applying"));
    return;
  }
  if (!state?.supported) {
    lastCpuPowerState = state;
    toggle.checked = false;
    toggle.disabled = true;
    setSuperPowerStatus(translate("settings.super.unsupported"), "power-mode-status warning");
    return;
  }
  toggle.disabled = false;
  toggle.checked = desired;
  lastCpuPowerState = state;
  const actual = !!state.active;
  const caps = frequencyLimitSummary(state);

  if (desired && actual) {
    setSuperPowerStatus(translate("settings.super.active", { caps }), "power-mode-status active");
  } else if (desired && !actual) {
    setSuperPowerStatus(
      translate("settings.super.wantOnMismatch", { caps }),
      "power-mode-status warning",
      {
        label: translate("settings.super.reapply"),
        onClick: () => applySuperPowerSaver(true),
      }
    );
  } else if (!desired && actual) {
    setSuperPowerStatus(
      translate("settings.super.wantOffMismatch", { caps }),
      "power-mode-status warning",
      {
        label: translate("settings.super.restore"),
        onClick: () => applySuperPowerSaver(false),
      }
    );
  } else {
    setSuperPowerStatus(translate("settings.super.inactive", { caps }));
  }
}

async function refreshSuperPowerSaver() {
  try {
    setSuperPowerStatus(translate("settings.super.checking"));
    const state = await invoke("get_cpu_power_state");
    renderSuperPowerSaver(state);
  } catch (err) {
    setSuperPowerStatus(translate("settings.super.error", { message: String(err) }), "power-mode-status warning");
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
    setSuperPowerStatus(translate("settings.super.error", { message: String(err) }), "power-mode-status warning");
    await refreshSuperPowerSaver();
  }
}

// 阈值输入框仅在对应提醒开启时可编辑。
function syncReminderInputs() {
  byId("setRemindChargeAt").disabled = !settings.remind_charge;
  byId("setRemindUnplugAt").disabled = !settings.remind_unplug;
}

async function persistSettings() {
  try {
    await invoke("save_settings", { newSettings: settings });
  } catch (err) {
    console.error("保存设置失败:", err);
  }
}

byId("setAutostart").addEventListener("change", (e) => {
  settings.autostart = e.target.checked;
  persistSettings();
});
byId("setSilentStart").addEventListener("change", (e) => {
  settings.silent_start = e.target.checked;
  persistSettings();
});
byId("setSuperPowerSaver").addEventListener("change", async (e) => {
  applySuperPowerSaver(e.target.checked);
});

function wireSegmentedSetting(groupId, key, apply) {
  byId(groupId).addEventListener("click", (e) => {
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
function commitThreshold(inputId, key, minValue, maxValue) {
  const input = byId(inputId);
  let nextValue = parseInt(input.value, 10);
  if (isNaN(nextValue)) nextValue = settings[key];
  nextValue = Math.min(maxValue, Math.max(minValue, nextValue));
  input.value = nextValue;
  settings[key] = nextValue;
  if (lastSnapshot?.battery) renderPowerPrediction(lastSnapshot.battery);
  persistSettings();
}

byId("setRemindCharge").addEventListener("change", (e) => {
  settings.remind_charge = e.target.checked;
  syncReminderInputs();
  persistSettings();
});
byId("setRemindUnplug").addEventListener("change", (e) => {
  settings.remind_unplug = e.target.checked;
  syncReminderInputs();
  persistSettings();
});
byId("setRemindChargeAt").addEventListener("change", () =>
  commitThreshold("setRemindChargeAt", "remind_charge_at", 1, 99)
);
byId("setRemindUnplugAt").addEventListener("change", () =>
  commitThreshold("setRemindUnplugAt", "remind_unplug_at", 1, 100)
);
byId("setCloseActionButton").addEventListener("click", () => {
  setCloseActionDropdownOpen(!byId("closeActionDropdown").classList.contains("open"));
});
byId("setCloseActionMenu").addEventListener("click", (e) => {
  const item = e.target.closest("[data-value]");
  if (!item) return;
  setCloseActionValue(item.dataset.value, true);
  setCloseActionDropdownOpen(false);
});
document.addEventListener("click", (e) => {
  if (!byId("closeActionDropdown").contains(e.target)) setCloseActionDropdownOpen(false);
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") setCloseActionDropdownOpen(false);
});

// ---------- 关闭确认弹框 ----------
const closeModal = byId("closeModal");
const showModal = () => closeModal.classList.add("show");
const hideModal = () => closeModal.classList.remove("show");

// 后端拦截窗口关闭后发来事件 → 弹框询问（仅 close_action=ask 时触发）。
listen("close-requested", () => showModal());

byId("closeCancel").addEventListener("click", hideModal);

byId("closeTray").addEventListener("click", () => {
  if (byId("closeRemember").checked) {
    settings.close_action = "tray";
    setCloseActionValue("tray");
    persistSettings();
  }
  hideModal();
  invoke("hide_window");
});

byId("closeQuit").addEventListener("click", () => {
  if (byId("closeRemember").checked) {
    settings.close_action = "exit";
    setCloseActionValue("exit");
    persistSettings();
  }
  invoke("quit_app");
});

loadSettings();
