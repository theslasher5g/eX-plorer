<script lang="ts">
  import { Check, ChevronRight, Ellipsis } from '@lucide/svelte';
  import type { MenuItem } from '../api';
  import { contextmenu, MORE_ID, type Level } from '../stores/contextmenu.svelte';

  /** Pro Ebene der Index des markierten Eintrags (-1: keiner). */
  let marked = $state<number[]>([]);
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;

  const levels = $derived(contextmenu.levels);

  // Beim Öffnen und Schließen den Zustand zurücksetzen.
  $effect(() => {
    if (levels.length === 0) marked = [];
  });

  // Das Menü gehört der Oberfläche, nicht dem Fenster: bei Größenänderung oder Fokusverlust zu.
  function close() {
    contextmenu.dismiss();
  }

  /** Setzt das Menü an den Anker und hält es im sichtbaren Bereich. */
  function place(node: HTMLElement, anchor: Level['anchor']) {
    const margin = 8;
    const rect = node.getBoundingClientRect();
    let x = anchor.x;
    let y = anchor.y;
    if (anchor.right !== undefined) {
      // Untermenü: rechts neben den Eintrag, bei Platzmangel links davon.
      if (x + rect.width > innerWidth - margin) x = anchor.right - rect.width;
      y -= 6;
    } else if (x + rect.width > innerWidth - margin) {
      x -= rect.width;
    }
    x = Math.max(margin, Math.min(x, innerWidth - rect.width - margin));
    y = Math.max(margin, Math.min(y, innerHeight - rect.height - margin));
    node.style.left = `${x}px`;
    node.style.top = `${y}px`;
  }

  function usable(item: MenuItem): boolean {
    return !item.separator && !item.disabled;
  }

  function step(depth: number, from: number, delta: 1 | -1): number {
    const items = levels[depth]?.items ?? [];
    for (let n = 1; n <= items.length; n++) {
      const index = (from + delta * n + items.length * 2) % items.length;
      if (usable(items[index])) return index;
    }
    return from;
  }

  function mark(depth: number, index: number) {
    const next = marked.slice(0, depth + 1);
    next[depth] = index;
    marked = next;
  }

  function openSub(depth: number, index: number, element: Element | null, focusFirst = false) {
    const item = levels[depth]?.items[index];
    if (!item?.submenu || !element) return;
    const rect = element.getBoundingClientRect();
    void contextmenu.openSub(depth, item, { x: rect.right - 4, y: rect.top, right: rect.left + 4 }).then(() => {
      if (focusFirst && contextmenu.levels.length > depth + 1) mark(depth + 1, step(depth + 1, -1, 1));
    });
  }

  function hover(depth: number, index: number, item: MenuItem, event: PointerEvent) {
    if (item.separator) return;
    mark(depth, index);
    clearTimeout(hoverTimer);
    const element = event.currentTarget as Element;
    if (item.submenu && !item.disabled) {
      hoverTimer = setTimeout(() => openSub(depth, index, element), 140);
    } else {
      hoverTimer = setTimeout(() => contextmenu.closeDeeperThan(depth), 140);
    }
  }

  function activate(depth: number, index: number, element: Element | null) {
    const item = levels[depth]?.items[index];
    if (!item || item.disabled || item.separator) return;
    if (item.submenu) openSub(depth, index, element);
    else void contextmenu.choose(item);
  }

  function itemElement(depth: number, index: number): Element | null {
    return document.querySelector(`.cm[data-depth="${depth}"] [data-index="${index}"]`);
  }

  function onKeydown(event: KeyboardEvent) {
    if (!contextmenu.isOpen) return;
    // Solange das Menü offen ist, gehört ihm die Tastatur.
    event.stopPropagation();
    const depth = levels.length - 1;
    const index = marked[depth] ?? -1;
    switch (event.key) {
      case 'ArrowDown':
        mark(depth, step(depth, index, 1));
        break;
      case 'ArrowUp':
        mark(depth, step(depth, index < 0 ? 0 : index, -1));
        break;
      case 'Home':
        mark(depth, step(depth, -1, 1));
        break;
      case 'End':
        mark(depth, step(depth, 0, -1));
        break;
      case 'ArrowRight':
        if (index >= 0) openSub(depth, index, itemElement(depth, index), true);
        break;
      case 'ArrowLeft':
        if (depth > 0) contextmenu.closeDeeperThan(depth - 1);
        break;
      case 'Enter':
      case ' ':
        if (index >= 0) {
          const item = levels[depth].items[index];
          if (item.submenu) openSub(depth, index, itemElement(depth, index), true);
          else activate(depth, index, null);
        }
        break;
      case 'Escape':
        if (depth > 0) contextmenu.closeDeeperThan(depth - 1);
        else close();
        break;
      case 'Tab':
        break;
      default:
        return;
    }
    event.preventDefault();
  }
</script>

<svelte:window onkeydowncapture={onKeydown} onblur={close} onresize={close} />

{#if levels.length > 0}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="scrim"
    onpointerdown={close}
    oncontextmenu={(event) => {
      event.preventDefault();
      close();
    }}
  ></div>
  {#each levels as level, depth (depth + ':' + level.path.join('.'))}
    <div class="cm" role="menu" data-depth={depth} use:place={level.anchor}>
      {#each level.items as item, index (index)}
        {#if item.separator}
          <hr />
        {:else}
          <button
            class="mi"
            class:on={marked[depth] === index}
            class:def={item.default}
            class:more={item.id === MORE_ID}
            role="menuitem"
            data-index={index}
            aria-disabled={item.disabled}
            aria-haspopup={item.submenu ? 'menu' : undefined}
            tabindex="-1"
            onpointermove={(event) => marked[depth] !== index && hover(depth, index, item, event)}
            onclick={(event) => activate(depth, index, event.currentTarget)}
          >
            <span class="gut">
              {#if item.icon}
                <img src={item.icon} alt="" draggable="false" />
              {:else if item.checked}
                <Check size={14} strokeWidth={2.2} />
              {:else if item.id === MORE_ID}
                <Ellipsis size={15} strokeWidth={1.9} />
              {/if}
            </span>
            <span class="lbl">{item.label}</span>
            {#if item.shortcut}<span class="kbd">{item.shortcut}</span>{/if}
            {#if item.submenu}<ChevronRight class="sub" size={14} strokeWidth={1.9} />{/if}
          </button>
        {/if}
      {/each}
    </div>
  {/each}
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 900;
  }

  .cm {
    position: fixed;
    z-index: 901;
    min-width: 220px;
    max-width: 340px;
    max-height: calc(100vh - 16px);
    overflow-y: auto;
    padding: 6px;
    border-radius: 12px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow-lg);
    font-size: 13px;
    user-select: none;
  }

  hr {
    height: 1px;
    margin: 5px 8px;
    border: 0;
    background: var(--border);
  }

  .mi {
    width: 100%;
    height: 32px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px 0 6px;
    border-radius: 7px;
    text-align: left;
    color: var(--text);
  }

  .mi.on {
    background: var(--accent);
    color: var(--on-accent);
  }

  .mi[aria-disabled='true'] {
    color: var(--muted);
    opacity: 0.6;
  }

  .mi[aria-disabled='true'].on {
    background: var(--hover);
    color: var(--muted);
  }

  .def .lbl {
    font-weight: 600;
  }

  .gut {
    flex: none;
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
  }

  .gut img {
    width: 16px;
    height: 16px;
    object-fit: contain;
  }

  .lbl {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .kbd {
    flex: none;
    margin-left: 16px;
    font-size: 12px;
    opacity: 0.65;
  }

  .mi :global(.sub) {
    flex: none;
    margin-right: -2px;
    opacity: 0.7;
  }
</style>
