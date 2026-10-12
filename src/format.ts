import type { Lang } from "./i18n";

const UNITS = ["B", "KB", "MB", "GB", "TB"];

let lang: Lang = "es";
let oneDecimal = new Intl.NumberFormat("es-ES", { maximumFractionDigits: 1 });
let integer = new Intl.NumberFormat("es-ES");

/** Cambia el formato de números al del idioma elegido. */
export function setFormatLang(next: Lang) {
  lang = next;
  const locale = next === "en" ? "en-US" : "es-ES";
  oneDecimal = new Intl.NumberFormat(locale, { maximumFractionDigits: 1 });
  integer = new Intl.NumberFormat(locale);
}

export function formatSize(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${unit === 0 ? integer.format(value) : oneDecimal.format(value)} ${UNITS[unit]}`;
}

export function formatCount(n: number): string {
  return integer.format(n);
}

export function formatPercent(part: number, total: number): string {
  if (total <= 0) return "—";
  const pct = (part / total) * 100;
  const sign = lang === "en" ? "%" : " %";
  return pct > 0 && pct < 0.1 ? `<${oneDecimal.format(0.1)}${sign}` : `${oneDecimal.format(pct)}${sign}`;
}
