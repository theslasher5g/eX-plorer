<script lang="ts">
  import { onMount } from 'svelte';
  import { getApi, type SidebarData } from './lib/api';
  import { Pane } from './lib/stores/pane.svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Toolbar from './lib/components/Toolbar.svelte';
  import FileList from './lib/components/FileList.svelte';
  import StatusBar from './lib/components/StatusBar.svelte';

  const pane = new Pane();
  let sidebar = $state<SidebarData | null>(null);
  let toolbar = $state<ReturnType<typeof Toolbar>>();

  onMount(async () => {
    const api = await getApi();
    const info = await api.appInfo();
    document.documentElement.dataset.material = info.material;
    void pane.navigate(info.homeDir);
    sidebar = await api.sidebar();
  });

  function isTextInput(target: EventTarget | null): boolean {
    return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
  }

  function onKeydown(event: KeyboardEvent) {
    const { key, altKey, ctrlKey } = event;
    let handled = true;
    if (altKey && key === 'ArrowLeft') void pane.back();
    else if (altKey && key === 'ArrowRight') void pane.forward();
    else if (altKey && key === 'ArrowUp') void pane.up();
    else if (key === 'F5' || (ctrlKey && key.toLowerCase() === 'r')) void pane.refresh();
    else if ((ctrlKey && key.toLowerCase() === 'l') || key === 'F4') toolbar?.editPath();
    else if (ctrlKey && key.toLowerCase() === 'f') toolbar?.focusFilter();
    else if (key === 'Backspace' && !isTextInput(event.target)) void pane.up();
    else handled = false;
    if (handled) event.preventDefault();
  }

  // Maustasten „Zurück“/„Vor“.
  function onMouseup(event: MouseEvent) {
    if (event.button === 3) void pane.back();
    else if (event.button === 4) void pane.forward();
  }

  // Das Browser-Kontextmenü der WebView hat in der App nichts verloren.
  function onContextmenu(event: MouseEvent) {
    if (!isTextInput(event.target)) event.preventDefault();
  }
</script>

<svelte:window onkeydown={onKeydown} onmouseup={onMouseup} oncontextmenu={onContextmenu} />

<div class="app">
  <Sidebar data={sidebar} current={pane.path} onnavigate={(path) => pane.navigate(path)} />
  <main>
    <Toolbar bind:this={toolbar} {pane} />
    <FileList {pane} />
    <StatusBar {pane} />
  </main>
</div>

<style>
  .app {
    display: grid;
    grid-template-columns: 228px minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr);
    height: 100%;
  }

  main {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    min-width: 0;
    background: var(--surface);
    box-shadow: var(--shadow);
  }
</style>
