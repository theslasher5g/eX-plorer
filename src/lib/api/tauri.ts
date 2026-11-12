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
  OpResult,
  PasteResult,
  RenameResult,
  SidebarData,
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
  window: {
    minimize: () => getCurrentWindow().minimize(),
    toggleMaximize: () => getCurrentWindow().toggleMaximize(),
    close: () => getCurrentWindow().close(),
    isMaximized: () => getCurrentWindow().isMaximized(),
    onResized: (handler) => getCurrentWindow().onResized(handler),
  },
};
