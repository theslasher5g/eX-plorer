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

export type TransferMode = 'auto' | 'copy' | 'move';

export interface OpResult {
  /** Der Benutzer hat im Windows-Dialog abgebrochen. */
  aborted: boolean;
  count: number;
  /** Es wurde verschoben (nicht kopiert oder gelöscht). */
  moved: boolean;
  /** Namen der bearbeiteten Elemente. */
  names: string[];
}

export interface RenameResult {
  name: string;
  path: string;
  aborted: boolean;
}

export interface ClipboardFiles {
  paths: string[];
  /** Ausgeschnitten statt kopiert. */
  cut: boolean;
}

export type PasteResult = OpResult & { cut: boolean };

/** Drag-&-Drop-Ereignis des Fensters. Position in physischen Pixeln. */
export interface DragDropEvent {
  type: 'enter' | 'over' | 'drop' | 'leave';
  paths: string[];
  position: { x: number; y: number };
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
  newFolder(parent: string, base: string): Promise<string>;
  renameItem(path: string, newName: string): Promise<RenameResult>;
  transfer(sources: string[], dest: string, mode: TransferMode): Promise<OpResult>;
  deleteItems(paths: string[], permanent: boolean): Promise<OpResult>;
  clipboardSet(paths: string[], cut: boolean): Promise<void>;
  clipboardGet(): Promise<ClipboardFiles | null>;
  paste(dest: string): Promise<PasteResult>;
  /** Startet einen Drag in andere Programme; kehrt nach dem Loslassen zurück. */
  startDrag(paths: string[]): Promise<number>;
  /** Setzt die Ordner, deren Änderungen gemeldet werden. */
  watchDirs(paths: string[]): Promise<void>;
  onDirChanged(handler: (path: string) => void): Promise<() => void>;
  onDragDrop(handler: (event: DragDropEvent) => void): Promise<() => void>;
  window: WindowControls;
}

export function toApiError(err: unknown): ApiError {
  if (err && typeof err === 'object' && 'code' in err && typeof err.code === 'string') {
    const message = 'message' in err && typeof err.message === 'string' ? err.message : '';
    return { code: err.code, message };
  }
  return { code: 'internal', message: String(err) };
}
