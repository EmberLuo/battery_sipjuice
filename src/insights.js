// Battery SipJuice — 充电会话与长期健康趋势

import { formatDuration } from "./formatters.js";
import { currentLanguage, translate } from "./i18n.js";

const { invoke } = window.__TAURI__.core;
const byId = (id) => document.getElementById(id);

let insightsData = { sessions: [], health: [] };
let insightsBatteryId = "";
let insightsRequestId = 0;

function batteryInsightId(battery) {
  const serial = battery?.serial_number?.trim();
  return serial ? `serial:${serial}` : battery?.device ? `device:${battery.device}` : "";
}

function formatDurationMs(ms) {
  if (ms == null || !Number.isFinite(Number(ms))) return "—";
  return formatDuration(Math.max(0, Math.round(Number(ms) / 60_000)), true);
}

function formatSessionTime(session) {
  const start = new Date(session.start_ms);
  const end = new Date(session.end_ms);
  const date = start.toLocaleDateString(currentLanguage, { month: "short", day: "numeric" });
  const startTime = start.toLocaleTimeString(currentLanguage, { hour: "2-digit", minute: "2-digit" });
  const endTime = end.toLocaleTimeString(currentLanguage, { hour: "2-digit", minute: "2-digit" });
  return session.active ? `${date} ${startTime}` : `${date} ${startTime}–${endTime}`;
}

function finiteOrNull(value) {
  return value == null || !Number.isFinite(Number(value)) ? null : Number(value);
}

function formatMetric(value, unit, decimals = 1) {
  const number = finiteOrNull(value);
  return number == null ? "—" : `${number.toFixed(decimals)}${unit}`;
}

function sessionSource(session) {
  const values = session.usb_types?.length
    ? session.usb_types
    : session.source_kinds?.length
      ? session.source_kinds
      : session.source_names;
  return values?.length ? values.join(" + ") : translate("sessions.unknownSource");
}

function sessionStatus(session) {
  if (session.active) return { text: translate("sessions.active"), className: "active" };
  if (!session.complete) return { text: translate("sessions.incomplete"), className: "incomplete" };
  return { text: translate("sessions.complete"), className: "complete" };
}

function comparisonDelta(current, previous, unit, decimals = 1, scale = 1) {
  const currentValue = finiteOrNull(current);
  const previousValue = finiteOrNull(previous);
  if (currentValue == null || previousValue == null) return "—";
  const delta = (currentValue - previousValue) / scale;
  const sign = delta > 0 ? "+" : "";
  return `${translate("sessions.previous")} ${sign}${delta.toFixed(decimals)}${unit}`;
}

function comparisonItem(label, value, delta) {
  const item = document.createElement("div");
  item.className = "comparison-item";
  const labelElement = document.createElement("span");
  labelElement.textContent = label;
  const valueElement = document.createElement("b");
  valueElement.textContent = value;
  const deltaElement = document.createElement("small");
  deltaElement.textContent = delta;
  item.append(labelElement, valueElement, deltaElement);
  return item;
}

function renderSessionComparison() {
  const target = byId("sessionComparison");
  target.replaceChildren();
  const completed = insightsData.sessions.filter((session) => !session.active && session.complete);
  if (completed.length < 2) {
    const empty = document.createElement("p");
    empty.className = "empty-hint";
    empty.textContent = translate("sessions.comparison.empty");
    target.appendChild(empty);
    return;
  }
  const [current, previous] = completed;
  target.append(
    comparisonItem(
      translate("sessions.duration"),
      formatDurationMs(current.charging_ms || current.duration_ms),
      comparisonDelta(
        current.charging_ms || current.duration_ms,
        previous.charging_ms || previous.duration_ms,
        ` ${translate("time.minute")}`,
        0,
        60_000
      )
    ),
    comparisonItem(
      translate("sessions.gained"),
      formatMetric(current.charged_percent, "%", 0),
      comparisonDelta(current.charged_percent, previous.charged_percent, "%", 0)
    ),
    comparisonItem(
      translate("sessions.avgInput"),
      formatMetric(current.average_input_power_w, " W"),
      comparisonDelta(current.average_input_power_w, previous.average_input_power_w, " W")
    ),
    comparisonItem(
      translate("sessions.peakTemp"),
      formatMetric(current.peak_temperature_c, " °C"),
      comparisonDelta(current.peak_temperature_c, previous.peak_temperature_c, " °C")
    )
  );
}

function sessionMetric(label, value) {
  const item = document.createElement("div");
  item.className = "session-metric";
  const name = document.createElement("span");
  name.textContent = label;
  const content = document.createElement("b");
  content.textContent = value;
  item.append(name, content);
  return item;
}

function renderSessionList() {
  const target = byId("sessionList");
  target.replaceChildren();
  if (!insightsData.sessions.length) {
    const empty = document.createElement("p");
    empty.className = "empty-hint";
    empty.textContent = translate("sessions.empty");
    target.appendChild(empty);
    return;
  }
  insightsData.sessions.forEach((session) => {
    const card = document.createElement("article");
    card.className = "session-card";
    const head = document.createElement("div");
    head.className = "session-head";
    const heading = document.createElement("div");
    const title = document.createElement("h4");
    title.textContent = formatSessionTime(session);
    const source = document.createElement("p");
    source.textContent = sessionSource(session);
    heading.append(title, source);
    const status = sessionStatus(session);
    const badge = document.createElement("span");
    badge.className = `session-badge ${status.className}`;
    badge.textContent = status.text;
    head.append(heading, badge);

    const metrics = document.createElement("div");
    metrics.className = "session-metrics";
    const range = session.start_capacity == null || session.end_capacity == null
      ? "—"
      : `${session.start_capacity}% → ${session.end_capacity}% (${session.charged_percent >= 0 ? "+" : ""}${session.charged_percent}%)`;
    metrics.append(
      sessionMetric(translate("sessions.duration"), formatDurationMs(session.charging_ms || session.duration_ms)),
      sessionMetric(translate("sessions.gained"), range),
      sessionMetric(translate("sessions.avgInput"), formatMetric(session.average_input_power_w, " W")),
      sessionMetric(translate("sessions.peakPower"), formatMetric(session.peak_input_power_w, " W")),
      sessionMetric(translate("sessions.peakTemp"), formatMetric(session.peak_temperature_c, " °C")),
      sessionMetric(translate("sessions.batteryEnergy"), formatMetric(session.battery_energy_wh, " Wh", 2)),
      sessionMetric(translate("sessions.inputEnergy"), formatMetric(session.input_energy_wh, " Wh", 2)),
      sessionMetric(translate("sessions.capacity"), formatMetric(session.charged_mah, " mAh", 0))
    );
    const foot = document.createElement("div");
    foot.className = "session-foot";
    foot.textContent = `${translate("sessions.source")}: ${sessionSource(session)} · ${translate("sessions.samples")}: ${session.powered_sample_count}/${session.sample_count}`;
    card.append(head, metrics);
    card.appendChild(foot);
    target.appendChild(card);
  });
}

function snapshotHealth(snapshot) {
  const direct = finiteOrNull(snapshot.health_percent);
  if (direct != null) return direct;
  const full = snapshot.full_capacity;
  const design = snapshot.design_capacity;
  const fullValue = finiteOrNull(full?.value);
  const designValue = finiteOrNull(design?.value);
  if (fullValue != null && designValue > 0 && full.unit === design.unit) {
    return (fullValue / designValue) * 100;
  }
  const driverSoh = finiteOrNull(snapshot.state_of_health);
  return driverSoh != null && driverSoh > 0 && driverSoh <= 100 ? driverSoh : null;
}

function smoothedHealthPoints(points) {
  const windowMs = 7 * 24 * 60 * 60_000;
  return points.map((point, index) => {
    const values = [];
    for (let cursor = index; cursor >= 0; cursor -= 1) {
      if (point.recorded_at_ms - points[cursor].recorded_at_ms >= windowMs) break;
      values.push(points[cursor].value);
    }
    values.sort((a, b) => a - b);
    const middle = Math.floor(values.length / 2);
    const value = values.length % 2
      ? values[middle]
      : (values[middle - 1] + values[middle]) / 2;
    return { ...point, value };
  });
}

function svgElement(name, attributes = {}) {
  const element = document.createElementNS("http://www.w3.org/2000/svg", name);
  Object.entries(attributes).forEach(([key, value]) => element.setAttribute(key, String(value)));
  return element;
}

function renderHealthTrend() {
  const chart = byId("healthTrendChart");
  const empty = byId("healthTrendEmpty");
  const summary = byId("healthTrendSummary");
  chart.replaceChildren();
  summary.replaceChildren();
  const rawPoints = insightsData.health
    .map((snapshot) => ({ ...snapshot, value: snapshotHealth(snapshot) }))
    .filter((snapshot) => snapshot.value != null);
  const points = smoothedHealthPoints(rawPoints);

  if (points.length) {
    const first = points[0];
    const last = points[points.length - 1];
    const firstCycles = finiteOrNull(first.cycle_count);
    const lastCycles = finiteOrNull(last.cycle_count);
    const cycleDelta = firstCycles == null || lastCycles == null ? null : lastCycles - firstCycles;
    const wearPerCycle = cycleDelta > 0 ? (first.value - last.value) / cycleDelta : null;
    summary.append(
      comparisonItem(translate("health.trend.latest"), formatMetric(last.value, "%"), ""),
      comparisonItem(
        translate("health.trend.change"),
        `${last.value - first.value >= 0 ? "+" : ""}${(last.value - first.value).toFixed(1)}%`,
        `${new Date(first.recorded_at_ms).toLocaleDateString(currentLanguage)} → ${new Date(last.recorded_at_ms).toLocaleDateString(currentLanguage)}`
      ),
      comparisonItem(
        translate("health.trend.cycles"),
        cycleDelta == null ? "—" : `${cycleDelta >= 0 ? "+" : ""}${cycleDelta}`,
        ""
      ),
      comparisonItem(
        translate("health.trend.wearPerCycle"),
        wearPerCycle == null ? "—" : `${wearPerCycle >= 0 ? "" : "−"}${Math.abs(wearPerCycle).toFixed(3)}%`,
        ""
      )
    );
  }

  const canDraw = points.length >= 2 && points[points.length - 1].recorded_at_ms > points[0].recorded_at_ms;
  chart.hidden = !canDraw;
  empty.hidden = canDraw;
  if (!canDraw) return;

  const width = 720;
  const height = 220;
  const padX = 44;
  const padY = 24;
  const minTime = points[0].recorded_at_ms;
  const maxTime = points[points.length - 1].recorded_at_ms;
  const rawMin = Math.min(...points.map((point) => point.value));
  const rawMax = Math.max(...points.map((point) => point.value));
  const minValue = Math.max(0, Math.floor(rawMin - 1));
  const maxValue = Math.min(110, Math.ceil(rawMax + 1));
  const valueSpan = Math.max(1, maxValue - minValue);
  const x = (time) => padX + ((time - minTime) / (maxTime - minTime)) * (width - padX * 2);
  const y = (value) => height - padY - ((value - minValue) / valueSpan) * (height - padY * 2);

  for (let i = 0; i <= 4; i += 1) {
    const value = minValue + (valueSpan * i) / 4;
    const lineY = y(value);
    chart.appendChild(svgElement("line", { x1: padX, y1: lineY, x2: width - padX, y2: lineY, class: "health-grid-line" }));
    const label = svgElement("text", { x: padX - 8, y: lineY + 4, "text-anchor": "end", class: "health-axis-label" });
    label.textContent = `${value.toFixed(0)}%`;
    chart.appendChild(label);
  }
  const polyline = svgElement("polyline", {
    points: points.map((point) => `${x(point.recorded_at_ms).toFixed(1)},${y(point.value).toFixed(1)}`).join(" "),
    class: "health-trend-line",
  });
  chart.appendChild(polyline);
  points.forEach((point) => {
    chart.appendChild(svgElement("circle", { cx: x(point.recorded_at_ms), cy: y(point.value), r: 3.5, class: "health-trend-point" }));
  });
  const firstLabel = svgElement("text", { x: padX, y: height - 4, class: "health-axis-label" });
  firstLabel.textContent = new Date(minTime).toLocaleDateString(currentLanguage, { month: "short", day: "numeric" });
  const lastLabel = svgElement("text", { x: width - padX, y: height - 4, "text-anchor": "end", class: "health-axis-label" });
  lastLabel.textContent = new Date(maxTime).toLocaleDateString(currentLanguage, { month: "short", day: "numeric" });
  chart.append(firstLabel, lastLabel);
}

export function renderBatteryInsights() {
  renderSessionComparison();
  renderSessionList();
  renderHealthTrend();
}

export async function refreshBatteryInsights(battery) {
  const requestId = ++insightsRequestId;
  const batteryId = batteryInsightId(battery);
  if (!batteryId) {
    insightsBatteryId = "";
    insightsData = { sessions: [], health: [] };
    renderBatteryInsights();
    return;
  }
  if (insightsBatteryId !== batteryId) {
    insightsBatteryId = batteryId;
    insightsData = { sessions: [], health: [] };
    renderBatteryInsights();
  }
  try {
    const result = await invoke("get_battery_insights", { batteryId });
    if (requestId !== insightsRequestId || insightsBatteryId !== batteryId) return;
    insightsData = result;
    renderBatteryInsights();
  } catch (err) {
    console.error("读取充电洞察失败:", err);
  }
}
