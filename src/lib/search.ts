/**
 * Namensvergleich der Suche: Teilzeichenfolge ohne Beachtung der Groß-/Kleinschreibung;
 * mit `*` oder `?` gilt das Muster für den ganzen Namen. Dasselbe tut das Backend.
 */
export function matchesName(name: string, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  const text = name.toLowerCase();
  if (!/[*?]/.test(needle)) return text.includes(needle);
  return glob(needle, text);
}

function glob(pattern: string, text: string): boolean {
  let p = 0;
  let t = 0;
  let star = -1;
  let mark = 0;
  while (t < text.length) {
    if (p < pattern.length && (pattern[p] === '?' || pattern[p] === text[t])) {
      p++;
      t++;
    } else if (p < pattern.length && pattern[p] === '*') {
      star = p++;
      mark = t;
    } else if (star >= 0) {
      p = star + 1;
      t = ++mark;
    } else {
      return false;
    }
  }
  while (p < pattern.length && pattern[p] === '*') p++;
  return p === pattern.length;
}
