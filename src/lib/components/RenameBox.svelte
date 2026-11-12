<script lang="ts">
  import { onMount, untrack } from 'svelte';

  let {
    name,
    isDir,
    oncommit,
    oncancel,
  }: {
    name: string;
    isDir: boolean;
    /** Liefert `true`, wenn der Name angenommen wurde. */
    oncommit: (value: string) => Promise<boolean>;
    oncancel: () => void;
  } = $props();

  let value = $state(untrack(() => name));
  let input = $state<HTMLInputElement>();
  let busy = false;
  let finished = false;

  onMount(() => {
    if (!input) return;
    input.focus();
    // Wie im Explorer: bei Dateien nur den Namen ohne Endung markieren.
    const dot = name.lastIndexOf('.');
    input.setSelectionRange(0, !isDir && dot > 0 ? dot : name.length);
  });

  async function commit(): Promise<boolean> {
    if (busy || finished) return true;
    busy = true;
    const accepted = await oncommit(value);
    busy = false;
    if (accepted) finished = true;
    return accepted;
  }

  function onKeydown(event: KeyboardEvent) {
    // Alle Tasten gehören dem Eingabefeld, nicht den Listen-Kürzeln.
    event.stopPropagation();
    if (event.key === 'Enter') {
      event.preventDefault();
      void commit().then((accepted) => {
        if (!accepted) input?.focus();
      });
    } else if (event.key === 'Escape') {
      event.preventDefault();
      finished = true;
      oncancel();
    }
  }

  async function onBlur() {
    if (finished) return;
    // Wie im Explorer: Wegklicken übernimmt den Namen; ein ungültiger wird verworfen.
    if (!(await commit())) {
      finished = true;
      oncancel();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<input
  bind:this={input}
  bind:value
  class="rename"
  aria-label={name}
  spellcheck="false"
  autocomplete="off"
  onkeydown={onKeydown}
  onblur={onBlur}
  onclick={(event) => event.stopPropagation()}
  ondblclick={(event) => event.stopPropagation()}
  onpointerdown={(event) => event.stopPropagation()}
/>

<style>
  .rename {
    flex: 1;
    min-width: 0;
    height: 26px;
    margin: 0 -2px;
    padding: 0 6px;
    border: 0;
    border-radius: var(--r-sm);
    outline: 0;
    background: var(--surface);
    box-shadow: 0 0 0 2px var(--accent);
    user-select: text;
    -webkit-user-select: text;
  }
</style>
