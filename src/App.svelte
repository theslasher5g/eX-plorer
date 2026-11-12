<script lang="ts">
  import { onMount } from 'svelte';
  import { getApi, type SidebarData } from './lib/api';
  import { Tabs } from './lib/stores/tabs.svelte';
  import FileList from './lib/components/FileList.svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import StatusBar from './lib/components/StatusBar.svelte';
  import TitleBar from './lib/components/TitleBar.svelte';
  import Toolbar from './lib/components/Toolbar.svelte';

  const tabs = new Tabs();
  const pane = $derived(tabs.current);

  let sidebar = $state<SidebarData | null>(null);
  let home = $state('');
  let toolbar = $state<ReturnType<typeof Toolbar>>();

  onMount(async () => {
    const api = await getApi();
    const info = await api.appInfo();
    document.documentElement.dataset.material = info.material;
    home = info.homeDir;
    void pane.navigate(home);
    sidebar = await api.sidebar();
  });

  function newTab() {
    if (home) tabs.open(home);
  }

  function isTextInput(target: EventTarget | null): boolean {
    return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
  }

  function onKeydown(event: KeyboardEvent) {
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
    else if (ctrlKey && key === 'w') tabs.close(tabs.active);
    else if (ctrlKey && key === 'Tab') tabs.cycle(shiftKey ? -1 : 1);
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

<div class="win">
  <TitleBar {tabs} onnewtab={newTab} />
  <div class="body">
    <Sidebar data={sidebar} current={pane.path} onnavigate={(path) => pane.navigate(path)} />
    <section class="right">
      <Toolbar bind:this={toolbar} {pane} />
      {#key pane.id}
        <FileList {pane} />
      {/key}
      <StatusBar {pane} />
    </section>
  </div>
</div>

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
