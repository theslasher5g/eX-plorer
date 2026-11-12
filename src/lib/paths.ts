// Reine Anzeige-Hilfen für Windows-Pfade. Die verbindliche Prüfung und
// Normalisierung findet immer im Backend statt.

export interface Crumb {
  label: string;
  path: string;
}

/** Zerlegt `C:\a\b` bzw. `\\server\share\a` in klickbare Teilpfade. */
export function splitPath(path: string): Crumb[] {
  const unified = path.replace(/\//g, '\\');
  let rootLabel: string;
  let rootPath: string;
  let rest: string;

  const unc = /^\\\\([^\\]+)\\([^\\]+)(.*)$/.exec(unified);
  const drive = /^([A-Za-z]:)(.*)$/.exec(unified);
  if (unc) {
    rootPath = `\\\\${unc[1]}\\${unc[2]}`;
    rootLabel = rootPath;
    rest = unc[3];
  } else if (drive) {
    rootLabel = drive[1].toUpperCase();
    rootPath = `${rootLabel}\\`;
    rest = drive[2];
  } else {
    return [];
  }

  const crumbs: Crumb[] = [{ label: rootLabel, path: rootPath }];
  let current = rootPath;
  for (const part of rest.split('\\')) {
    if (!part) continue;
    current = joinPath(current, part);
    crumbs.push({ label: part, path: current });
  }
  return crumbs;
}

export function parentPath(path: string): string | null {
  const crumbs = splitPath(path);
  return crumbs.length > 1 ? crumbs[crumbs.length - 2].path : null;
}

export function joinPath(dir: string, name: string): string {
  return dir.endsWith('\\') ? dir + name : `${dir}\\${name}`;
}

export function samePath(a: string, b: string): boolean {
  return a.toLowerCase() === b.toLowerCase();
}

/** Dateiendung in Kleinbuchstaben, ohne Punkt; leer, wenn keine vorhanden. */
export function extension(name: string): string {
  const dot = name.lastIndexOf('.');
  return dot > 0 && dot < name.length - 1 ? name.slice(dot + 1).toLowerCase() : '';
}
