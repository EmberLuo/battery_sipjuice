// Battery SipJuice — 跨页面复用的显示格式化函数

import { translate } from "./i18n.js";

export function formatNumber(value, decimals = 0) {
  return value == null || isNaN(value) ? "—" : Number(value).toFixed(decimals);
}

export function formatValueWithUnit(quantity, decimals = 0) {
  return quantity ? `${formatNumber(quantity.value, decimals)} ${quantity.unit}` : "—";
}

export function formatDuration(minutes, allowZero = false) {
  const totalMinutes = Number(minutes);
  if (!Number.isFinite(totalMinutes) || totalMinutes < 0 || (!allowZero && totalMinutes === 0)) return "—";
  const hours = Math.floor(totalMinutes / 60);
  const minutePart = totalMinutes % 60;
  return hours > 0
    ? `${hours} ${translate("time.hour")} ${minutePart} ${translate("time.minute")}`
    : `${minutePart} ${translate("time.minute")}`;
}
