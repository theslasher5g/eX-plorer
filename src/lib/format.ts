import { locale, t } from './i18n';
import { extension } from './paths';
import type { Entry } from './api';

const number1 = new Intl.NumberFormat(locale, { maximumFractionDigits: 1 });
const number0 = new Intl.NumberFormat(locale, { maximumFractionDigits: 0 });
const dateTime = new Intl.DateTimeFormat(locale, {
  day: '2-digit',
  month: '2-digit',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
});

const UNITS = ['KB', 'MB', 'GB', 'TB', 'PB'];

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

export function formatDate(ms: number | null): string {
  return ms === null ? '' : dateTime.format(ms);
}

export function formatCount(count: number): string {
  return number0.format(count);
}

export function typeLabel(entry: Entry): string {
  if (entry.isDir) return entry.isLink ? t('type.link') : t('type.folder');
  const ext = extension(entry.name);
  return ext ? t('type.fileExt', { ext: ext.toUpperCase() }) : t('type.file');
}
