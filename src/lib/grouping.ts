import type { Entry } from './api';
import { typeLabel } from './filetypes';
import { t, type MessageKey } from './i18n';

export type GroupKey = 'none' | 'name' | 'type' | 'modified' | 'size';

export const GROUP_KEYS: GroupKey[] = ['none', 'name', 'type', 'modified', 'size'];

/** Zu welcher Gruppe ein Eintrag gehört; `order` bestimmt die Reihenfolge der Gruppen. */
export interface GroupInfo {
  id: string;
  label: string;
  order: number;
}

/** Eine zusammenhängende Gruppe in der sortierten Liste. */
export interface Group {
  id: string;
  label: string;
  /** Index des ersten Eintrags in `visible`. */
  start: number;
  count: number;
}

const DAY = 86_400_000;

const SIZE_BUCKETS: { max: number; key: MessageKey }[] = [
  { max: 0, key: 'group.size.empty' },
  { max: 10 * 1024, key: 'group.size.tiny' },
  { max: 100 * 1024, key: 'group.size.small' },
  { max: 1024 * 1024, key: 'group.size.medium' },
  { max: 16 * 1024 * 1024, key: 'group.size.large' },
  { max: 128 * 1024 * 1024, key: 'group.size.huge' },
  { max: Infinity, key: 'group.size.giant' },
];

/** Beginn des Tages (lokale Zeit) in Millisekunden. */
function startOfDay(ms: number): number {
  const d = new Date(ms);
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

/** Grenzen der Zeit-Gruppen, einmal je Berechnung bestimmt. */
export function dateBoundaries(now: number) {
  const today = startOfDay(now);
  const d = new Date(today);
  const monday = today - ((d.getDay() + 6) % 7) * DAY;
  return {
    today,
    yesterday: today - DAY,
    thisWeek: monday,
    lastWeek: monday - 7 * DAY,
    thisMonth: new Date(d.getFullYear(), d.getMonth(), 1).getTime(),
    lastMonth: new Date(d.getFullYear(), d.getMonth() - 1, 1).getTime(),
    thisYear: new Date(d.getFullYear(), 0, 1).getTime(),
  };
}

export type DateBounds = ReturnType<typeof dateBoundaries>;

const DATE_BUCKETS: MessageKey[] = [
  'group.date.today',
  'group.date.yesterday',
  'group.date.thisWeek',
  'group.date.lastWeek',
  'group.date.thisMonth',
  'group.date.lastMonth',
  'group.date.thisYear',
  'group.date.older',
  'group.date.unknown',
];

export function dateBucket(ms: number | null | undefined, b: DateBounds): number {
  if (ms == null) return 8;
  if (ms >= b.today) return 0;
  if (ms >= b.yesterday) return 1;
  if (ms >= b.thisWeek) return 2;
  if (ms >= b.lastWeek) return 3;
  if (ms >= b.thisMonth) return 4;
  if (ms >= b.lastMonth) return 5;
  if (ms >= b.thisYear) return 6;
  return 7;
}

export function sizeBucket(size: number): number {
  const index = SIZE_BUCKETS.findIndex((b) => size <= b.max);
  return index < 0 ? SIZE_BUCKETS.length - 1 : index;
}

/** Ordnet einen Eintrag einer Gruppe zu. Ordner haben bei Größe eine eigene Gruppe. */
export function groupOf(entry: Entry, key: GroupKey, bounds: DateBounds): GroupInfo {
  switch (key) {
    case 'name': {
      const first = entry.name.replace(/^[^\p{L}\p{N}]+/u, '').charAt(0).toUpperCase();
      if (!first) return { id: 'n:#', label: '#', order: 0 };
      if (/\p{N}/u.test(first)) return { id: 'n:0-9', label: '0–9', order: 1 };
      return { id: `n:${first}`, label: first, order: 2 };
    }
    case 'type': {
      const label = typeLabel(entry);
      return { id: `t:${label}`, label, order: entry.isDir ? 0 : 1 };
    }
    case 'modified': {
      const order = dateBucket(entry.modifiedMs, bounds);
      return { id: `m:${order}`, label: t(DATE_BUCKETS[order]), order };
    }
    case 'size': {
      if (entry.isDir) return { id: 's:dir', label: t('group.size.folder'), order: 0 };
      const index = sizeBucket(entry.size);
      return { id: `s:${index}`, label: t(SIZE_BUCKETS[index].key), order: index + 1 };
    }
    default:
      return { id: '', label: '', order: 0 };
  }
}
