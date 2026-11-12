export interface Entry {
  name: string;
  isDir: boolean;
  size: number;
  modifiedMs: number | null;
  createdMs: number | null;
  attributes: number;
  hidden: boolean;
  system: boolean;
  isLink: boolean;
}

export type PlaceKind = 'desktop' | 'documents' | 'downloads' | 'pictures' | 'onedrive';

export interface Place {
  kind: PlaceKind;
  label: string;
  path: string;
}

export type DriveKind = 'fixed' | 'removable' | 'remote' | 'cdrom' | 'ram' | 'unknown';

export interface Drive {
  path: string;
  label: string;
  kind: DriveKind;
  totalBytes: number | null;
  freeBytes: number | null;
}

export interface SidebarData {
  favorites: Place[];
  drives: Drive[];
  onedrive: Place[];
}

export interface AppInfo {
  material: 'mica' | 'solid';
  homeDir: string;
}

export interface ListSummary {
  path: string;
  count: number;
}

export interface ApiError {
  code: string;
  message: string;
}

export interface WindowControls {
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  close(): Promise<void>;
  isMaximized(): Promise<boolean>;
  onResized(handler: () => void): Promise<() => void>;
}

/** Einzige Schnittstelle des Frontends zum Backend. */
export interface Api {
  appInfo(): Promise<AppInfo>;
  sidebar(): Promise<SidebarData>;
  listDir(path: string, onBatch: (entries: Entry[]) => void): Promise<ListSummary>;
  window: WindowControls;
}

export function toApiError(err: unknown): ApiError {
  if (err && typeof err === 'object' && 'code' in err && typeof err.code === 'string') {
    const message = 'message' in err && typeof err.message === 'string' ? err.message : '';
    return { code: err.code, message };
  }
  return { code: 'internal', message: String(err) };
}
