import { Channel, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type {
  Api,
  AppInfo,
  ClipboardFiles,
  Entry,
  ListSummary,
  MenuItem,
  MenuOpened,
  MenuOutcome,
  NetDrive,
  OpenResult,
  OpResult,
  PasteResult,
  PreviewInfo,
  Reachability,
  RenameResult,
  ServerRef,
  Settings,
  ShareInfo,
  SidebarData,
  TextContent,
  TreeChunk,
  TreeSummary,
} from './types';

export const tauriApi: Api = {
  appInfo: () => invoke<AppInfo>('app_info'),
  sidebar: () => invoke<SidebarData>('sidebar'),
  listDir(path, onBatch) {
    const channel = new Channel<Entry[]>();
    channel.onmessage = onBatch;
    return invoke<ListSummary>('list_dir', { path, onBatch: channel });
  },
  newFolder: (parent, base) => invoke<string>('new_folder', { parent, base }),
  renameItem: (path, newName) => invoke<RenameResult>('rename_item', { path, newName }),
  transfer: (sources, dest, mode) => invoke<OpResult>('transfer', { sources, dest, mode }),
  deleteItems: (paths, permanent) => invoke<OpResult>('delete_items', { paths, permanent }),
  clipboardSet: (paths, cut) => invoke<void>('clipboard_set', { paths, cut }),
  clipboardGet: () => invoke<ClipboardFiles | null>('clipboard_get'),
  paste: (dest) => invoke<PasteResult>('paste', { dest }),
  startDrag: (paths) => invoke<number>('start_drag', { paths }),
  watchDirs: (paths) => invoke<void>('watch_dirs', { paths }),
  onDirChanged: (handler) => listen<string>('dir-changed', (event) => handler(event.payload)),
  onDragDrop: (handler) =>
    getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;
      handler({
        type: payload.type,
        paths: 'paths' in payload ? payload.paths : [],
        position: 'position' in payload ? payload.position : { x: 0, y: 0 },
      });
    }),
  previewInfo: (path) => invoke<PreviewInfo>('preview_info', { path }),
  previewBytes: (path) => invoke<ArrayBuffer>('preview_bytes', { path }),
  previewText: (path) => invoke<TextContent>('preview_text', { path }),
  menuOpen: (paths, folder, newTabLabel, extras) =>
    invoke<MenuOpened>('menu_open', { paths, folder, newTabLabel, extras }),
  menuSubmenu: (session, path) => invoke<MenuItem[]>('menu_submenu', { session, path }),
  menuInvoke: (session, id) => invoke<MenuOutcome>('menu_invoke', { session, id }),
  menuNative: (session) => invoke<MenuOutcome>('menu_native', { session }),
  menuClose: (session) => invoke<void>('menu_close', { session }),
  onedriveSetPinned: (paths, pinned) => invoke<number>('onedrive_set_pinned', { paths, pinned }),
  openItem: (path, confirmed) => invoke<OpenResult>('open_item', { path, confirmed }),
  treeScan(args, onChunk) {
    const channel = new Channel<TreeChunk>();
    channel.onmessage = onChunk;
    return invoke<TreeSummary>('tree_scan', { args, onChunk: channel });
  },
  treeCancel: (scanId) => invoke<void>('tree_cancel', { scanId }),
  settingsGet: () => invoke<Settings>('settings_get'),
  settingsSet: (settings) => invoke<Settings>('settings_set', { settings }),
  clipboardSetText: (text) => invoke<void>('clipboard_set_text', { text }),
  networkDrives: () => invoke<NetDrive[]>('network_drives'),
  networkProbe: (path) => invoke<Reachability>('network_probe', { path }),
  networkConnect: (target) => invoke<{ connected: boolean }>('network_connect', { target }),
  networkParse: (input) => invoke<ServerRef>('network_parse', { input }),
  networkShares: (server) => invoke<ShareInfo[]>('network_shares', { server }),
  window: {
    minimize: () => getCurrentWindow().minimize(),
    setTheme: (theme) => getCurrentWindow().setTheme(theme),
    toggleMaximize: () => getCurrentWindow().toggleMaximize(),
    close: () => getCurrentWindow().close(),
    isMaximized: () => getCurrentWindow().isMaximized(),
    onResized: (handler) => getCurrentWindow().onResized(handler),
  },
};
