import { locale, t } from './i18n';

const number1 = new Intl.NumberFormat(locale, { maximumFractionDigits: 1 });
const number0 = new Intl.NumberFormat(locale, { maximumFractionDigits: 0 });
const dateOnly = new Intl.DateTimeFormat(locale, { day: '2-digit', month: '2-digit', year: 'numeric' });
const timeOnly = new Intl.DateTimeFormat(locale, { hour: '2-digit', minute: '2-digit' });
const dateTime = new Intl.DateTimeFormat(locale, { dateStyle: 'full', timeStyle: 'short' });

const UNITS = ['KB', 'MB', 'GB', 'TB', 'PB'];
const DAY = 86_400_000;

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${number0.format(bytes)} B`;
  let value = bytes;
  let unit = -1;
  do {
    value /= 1024;
    unit++;
  } while (value >= 1024 && unit < UNITS.length - 1);
  return `${(value >= 100 ? number0 : number1).format(value)} ${UNITS[unit]}`;
}

function startOfDay(ms: number): number {
  const date = new Date(ms);
  date.setHours(0, 0, 0, 0);
  return date.getTime();
}

/** „Heute, 09:41“, „Gestern, 17:20“, sonst das Datum. */
export function formatDate(ms: number | null, now = Date.now()): string {
  if (ms === null) return '';
  const today = startOfDay(now);
  if (ms >= today && ms < today + DAY) return t('date.today', { time: timeOnly.format(ms) });
  if (ms >= startOfDay(today - 1) && ms < today) return t('date.yesterday', { time: timeOnly.format(ms) });
  return dateOnly.format(ms);
}

/** Ausgeschriebenes Datum mit Uhrzeit, für Tooltips. */
export function formatDateLong(ms: number | null): string {
  return ms === null ? '' : dateTime.format(ms);
}

export function formatCount(count: number): string {
  return number0.format(count);
}
