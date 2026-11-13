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
  /** OneDrive-Sync-Zustand; `null` außerhalb von Cloud-Ordnern. */
  sync: SyncState | null;
}

export type SyncState = 'cloud' | 'local' | 'pinned' | 'syncing';

export type PlaceKind = 'desktop' | 'documents' | 'downloads' | 'pictures' | 'onedrive';

export interface Place {
  kind: PlaceKind;
  label: string;
  path: string;
  /** OneDrive: E-Mail-Adresse des Kontos. */
  email?: string;
  /** OneDrive: privates oder geschäftliches Konto. */
  account?: 'personal' | 'business';
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

export interface PreviewInfo {
  kind: 'image' | 'pdf' | 'text' | 'other';
  mime: string | null;
  size: number;
  isDir: boolean;
  /** Nur in der Cloud: eine Vorschau würde den Inhalt herunterladen. */
  cloudOnly: boolean;
  tooLarge: boolean;
}

export interface TextContent {
  text: string;
  truncated: boolean;
}

export type MenuOutcome =
  | { kind: 'dismissed' }
  | { kind: 'done' }
  | { kind: 'verb'; verb: string }
  | { kind: 'newTab' }
  | { kind: 'extra'; id: number };

/** Zusätzlicher Eintrag der App im nativen Kontextmenü. */
export interface MenuExtra {
  /** Eigene Kennung im Bereich 0x7001–0x70FF. */
  id: number;
  label: string;
  /** Entfällt, wenn das Windows-Menü schon einen Eintrag mit einem dieser Texte hat. */
  skipIf: string[];
}

export interface OpenResult {
  /** Die Datei kann Code ausführen; erst nach Bestätigung erneut öffnen. */
  needsConfirm: boolean;
}

export interface Settings {
  /** Versteckte und Systemdateien anzeigen. */
  showHidden: boolean;
  treeDepth: number;
  treeTotal: number;
  treePerDir: number;
  /** Kachelbreite in Pixeln. */
  tileSize: number;
  /** Vom Benutzer hinzugefügte Server (`\\server`). */
  servers: string[];
}

export interface TreeEntry {
  name: string;
  isDir: boolean;
  /** Symlink oder Junction: wird nie automatisch betreten. */
  isLink: boolean;
  size: number;
  /** Ordner wurde wegen des Tiefenlimits nicht gelesen. */
  depthLimited: boolean;
}

export interface TreeChunk {
  /** Pfad relativ zur Scan-Wurzel. */
  rel: string[];
  entries: TreeEntry[];
  /** Anzahl weiterer, nicht gelieferter Einträge dieses Ordners. */
  more: number;
  error: string | null;
}

export interface TreeScanArgs {
  path: string;
  scanId: number;
  depth: number;
  total: number;
  perDir: number;
  offset?: number;
  showHidden: boolean;
}

export interface TreeSummary {
  count: number;
  cancelled: boolean;
  /** Das Gesamtlimit hat den Scan beendet. */
  truncated: boolean;
  /** Netzwerkpfad: Es wurde bewusst nur eine Ebene gelesen. */
  remoteLimited: boolean;
}

export type Reachability = 'online' | 'offline' | 'authrequired';

export interface NetDrive {
  /** Laufwerksbuchstabe, z. B. `Z`. */
  letter: string;
  /** Wurzelpfad, z. B. `Z:\\`. */
  path: string;
  /** `\\server\\freigabe`, wenn bekannt. */
  remote: string | null;
}

export interface ShareInfo {
  name: string;
  comment: string;
  /** `\\server\\freigabe` */
  path: string;
}

export interface ServerRef {
  /** `\\server` */
  server: string;
  /** `\\server\\freigabe`, wenn angegeben. */
  share: string | null;
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
  previewInfo(path: string): Promise<PreviewInfo>;
  previewBytes(path: string): Promise<ArrayBuffer>;
  previewText(path: string): Promise<TextContent>;
  /** Zeigt das native Windows-Kontextmenü an der Mausposition. */
  contextMenu(
    paths: string[],
    folder: string | null,
    newTabLabel: string | null,
    extras: MenuExtra[],
  ): Promise<MenuOutcome>;
  /** „Immer auf diesem Gerät behalten“ (true) bzw. „Speicherplatz freigeben“ (false). */
  onedriveSetPinned(paths: string[], pinned: boolean): Promise<number>;
  openItem(path: string, confirmed: boolean): Promise<OpenResult>;
  treeScan(args: TreeScanArgs, onChunk: (chunk: TreeChunk) => void): Promise<TreeSummary>;
  treeCancel(scanId: number): Promise<void>;
  settingsGet(): Promise<Settings>;
  settingsSet(settings: Settings): Promise<Settings>;
  clipboardSetText(text: string): Promise<void>;
  networkDrives(): Promise<NetDrive[]>;
  /** Erreichbarkeit einer Wurzel; Zeitüberschreitung zählt als `offline`. */
  networkProbe(path: string): Promise<Reachability>;
  /** Verbindet über den Windows-Anmeldedialog; `false` = abgebrochen. */
  networkConnect(target: string): Promise<{ connected: boolean }>;
  networkParse(input: string): Promise<ServerRef>;
  networkShares(server: string): Promise<ShareInfo[]>;
  window: WindowControls;
}

export function toApiError(err: unknown): ApiError {
  if (err && typeof err === 'object' && 'code' in err && typeof err.code === 'string') {
    const message = 'message' in err && typeof err.message === 'string' ? err.message : '';
    return { code: err.code, message };
  }
  return { code: 'internal', message: String(err) };
}
