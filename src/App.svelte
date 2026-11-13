<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { getApi, type SidebarData } from './lib/api';
  import { samePath } from './lib/paths';
  import { contextmenu } from './lib/stores/contextmenu.svelte';
import { dialogs } from './lib/stores/dialog.svelte';
  import { dnd } from './lib/stores/dnd.svelte';
  import { onedrive } from './lib/stores/onedrive.svelte';
  import { ops } from './lib/stores/ops.svelte';
  import { quicklook } from './lib/stores/quicklook.svelte';
  import { settings } from './lib/stores/settings.svelte';
  import { Tabs } from './lib/stores/tabs.svelte';
  import DragGhost from './lib/components/DragGhost.svelte';
  import ContextMenu from './lib/components/ContextMenu.svelte';
  import ColumnView from './lib/components/ColumnView.svelte';
  import ConfirmDialog from './lib/components/ConfirmDialog.svelte';
  import FileList from './lib/components/FileList.svelte';
  import NetworkPage from './lib/components/NetworkPage.svelte';
  import QuickLook from './lib/components/QuickLook.svelte';
  import SettingsDialog from './lib/components/SettingsDialog.svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import StatusBar from './lib/components/StatusBar.svelte';
  import TitleBar from './lib/components/TitleBar.svelte';
  import TileGrid from './lib/components/TileGrid.svelte';
  import Toast from './lib/components/Toast.svelte';
  import TreeView from './lib/components/TreeView.svelte';
  import Toolbar from './lib/components/Toolbar.svelte';

  const tabs = new Tabs();
  const pane = $derived(tabs.current);

  let sidebar = $state<SidebarData | null>(null);
  let home = $state('');
  let toolbar = $state<ReturnType<typeof Toolbar>>();

  ops.bind(tabs, () => sidebar?.drives ?? []);

  onMount(() => {
    const cleanups: (() => void)[] = [];
    let disposed = false;
    const keep = (stop: () => void) => (disposed ? stop() : cleanups.push(stop));

    void (async () => {
      const api = await getApi();
      await settings.load();
      // Der erste Tab beginnt mit der gewählten Startansicht.
      pane.view = settings.value.startView;
      const info = await api.appInfo();
      document.documentElement.dataset.material = info.material;
      home = info.homeDir;
      void pane.navigate(home);
      sidebar = await api.sidebar();
      onedrive.setRoots(sidebar.onedrive.map((place) => place.path));
      onedrive.accounts = sidebar.onedrive;

      // Änderungen im Dateisystem (auch von anderen Programmen) nachladen.
      keep(
        await api.onDirChanged((dir) => {
          for (const p of tabs.livePanes) if (samePath(p.path, dir)) void p.refresh();
        }),
      );
      keep(await dnd.init());
      void ops.refreshClipboard();
    })();

    // Die Zwischenablage kann sich in anderen Programmen ändern.
    const onFocus = () => void ops.refreshClipboard();
    window.addEventListener('focus', onFocus);
    return () => {
      disposed = true;
      window.removeEventListener('focus', onFocus);
      cleanups.forEach((stop) => stop());
    };
  });

  // Die Ordner aller Tabs beobachten lassen.
  $effect(() => {
    const paths = [...new Set(tabs.livePanes.map((p) => p.path).filter(Boolean))];
    void getApi().then((api) => api.watchDirs(paths)).catch(() => {});
  });

  // Suche in Unterordnern: ab dem ersten Zeichen, nach kurzer Pause, im aktiven Tab.
  $effect(() => {
    const target = pane;
    const query = target.filter.trim();
    const path = target.path;
    const hidden = target.showHidden;
    const searchable = target.view !== 'tree' && !target.isNetwork && !target.error && !!path;
    if (!query || !searchable) {
      untrack(() => target.search.clear());
      return;
    }
    const timer = setTimeout(() => untrack(() => void target.search.run(path, query, hidden)), 300);
    return () => clearTimeout(timer);
  });

  function newTab() {
    if (home) tabs.open(home);
  }

  function isTextInput(target: EventTarget | null): boolean {
    return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
  }

  function onKeydown(event: KeyboardEvent) {
    // Solange ein Dialog offen ist, gehört ihm die Tastatur.
    if (dialogs.confirmState || quicklook.isOpen || settings.dialog || contextmenu.isOpen) return;
    const { altKey, ctrlKey, shiftKey } = event;
    const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
    let handled = true;
    if (altKey && key === 'ArrowLeft') void pane.back();
    else if (altKey && key === 'ArrowRight') void pane.forward();
    else if (altKey && key === 'ArrowUp') void pane.up();
    else if (key === 'F5' || (ctrlKey && key === 'r')) void pane.refresh();
    else if ((ctrlKey && key === 'l') || key === 'F4') toolbar?.editPath();
    else if (ctrlKey && key === 'f') toolbar?.focusFilter();
    else if (ctrlKey && key === 't') newTab();
    else if (ctrlKey && key === ',') settings.open();
    else if (ctrlKey && key === '1') pane.view = 'list';
    else if (ctrlKey && key === '2') pane.view = 'grid';
    else if (ctrlKey && key === '3') pane.view = 'columns';
    else if (ctrlKey && key === '4') pane.view = 'tree';
    else if (ctrlKey && key === 'w') tabs.close(tabs.active);
    else if (ctrlKey && key === 'Tab') tabs.cycle(shiftKey ? -1 : 1);
    else if (isTextInput(event.target)) handled = false;
    // Die Baumansicht hat eine eigene Auswahl; Dateioperationen gibt es dort nicht.
    else if (pane.view === 'tree') handled = key === 'Backspace' ? (void pane.up(), true) : false;
    else if (key === 'Backspace') void pane.up();
    else if (ctrlKey && shiftKey && key === 'n') void ops.newFolder();
    else if (ctrlKey && key === 'c') void ops.copy(false);
    else if (ctrlKey && key === 'x') void ops.copy(true);
    else if (ctrlKey && key === 'v') void ops.paste();
    else if (key === ' ') quicklook.open(tabs.target);
    else if (key === 'F2') ops.startRename();
    else if (key === 'Delete') void ops.remove(shiftKey);
    else handled = false;
    if (handled) event.preventDefault();
  }

  // Maustasten „Zurück“/„Vor“.
  function onMouseup(event: MouseEvent) {
    if (event.button === 3) void pane.back();
    else if (event.button === 4) void pane.forward();
  }

  // Merkt, wo das Menü erscheinen soll: beim Rechtsklick am Zeiger, über die
  // Tastatur (Menütaste, Umschalt+F10) an der markierten Zeile.
  function onContextPoint(event: MouseEvent) {
    let point = { x: event.clientX, y: event.clientY };
    if (event.button !== 2) {
      const row = document.querySelector('.row.cursor, .tile.cursor, .crow.cur, .row.sel, .tile.sel, .crow.sel');
      if (row) {
        const rect = row.getBoundingClientRect();
        point = { x: rect.left + Math.min(48, rect.width / 2), y: rect.bottom - 4 };
      }
    }
    contextmenu.point = point;
  }

  // Das Browser-Kontextmenü der WebView hat in der App nichts verloren.
  function onContextmenu(event: MouseEvent) {
    if (!isTextInput(event.target)) event.preventDefault();
  }
</script>

<svelte:window
  onkeydown={onKeydown}
  onmouseup={onMouseup}
  oncontextmenu={onContextmenu}
  oncontextmenucapture={onContextPoint}
/>

<div class="win">
  <TitleBar {tabs} onnewtab={newTab} />
  <div class="body">
    <Sidebar
      data={sidebar}
      current={pane.path}
      onnavigate={(path) => pane.navigate(path)}
      onnewtab={(path) => tabs.open(path)}
    />
    <section class="right">
      <Toolbar bind:this={toolbar} {pane} />
      {#key pane.id}
        {#if pane.isNetwork}
          <NetworkPage {pane} />
        {:else if pane.view === 'tree'}
          <TreeView {pane} />
        {:else if pane.view === 'grid'}
          <TileGrid {pane} />
        {:else if pane.view === 'columns'}
          <ColumnView {pane} />
        {:else}
          <FileList {pane} />
        {/if}
      {/key}
      <StatusBar {pane} />
    </section>
  </div>
</div>

<QuickLook />
<ContextMenu />
<DragGhost />
<SettingsDialog />
<ConfirmDialog />
<Toast />

<style>
  .win {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .right {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    position: relative;
    background: var(--surface);
    border-top-left-radius: 10px;
    border-top: 1px solid var(--border);
    border-left: 1px solid var(--border);
    overflow: hidden;
  }
</style>
