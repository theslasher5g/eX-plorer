import { Channel, invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { Api, AppInfo, Entry, ListSummary, SidebarData } from './types';

export const tauriApi: Api = {
  appInfo: () => invoke<AppInfo>('app_info'),
  sidebar: () => invoke<SidebarData>('sidebar'),
  listDir(path, onBatch) {
    const channel = new Channel<Entry[]>();
    channel.onmessage = onBatch;
    return invoke<ListSummary>('list_dir', { path, onBatch: channel });
  },
  window: {
    minimize: () => getCurrentWindow().minimize(),
    toggleMaximize: () => getCurrentWindow().toggleMaximize(),
    close: () => getCurrentWindow().close(),
    isMaximized: () => getCurrentWindow().isMaximized(),
    onResized: (handler) => getCurrentWindow().onResized(handler),
  },
};
