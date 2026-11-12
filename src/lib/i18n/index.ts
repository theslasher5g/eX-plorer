import { de, type MessageKey } from './de';

export type { MessageKey };

/** Aktive Sprache. Weitere Sprachen brauchen nur ein Wörterbuch mit denselben Schlüsseln. */
export const locale = 'de-DE';
const messages: Record<MessageKey, string> = de;

export function t(key: MessageKey, params?: Record<string, string | number>): string {
  const template = messages[key];
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}

export function errorText(code: string): string {
  const key = `error.${code}`;
  return key in messages ? messages[key as MessageKey] : messages['error.internal'];
}
