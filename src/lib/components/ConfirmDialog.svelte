<script lang="ts">
  import { tick } from 'svelte';
  import { dialogs } from '../stores/dialog.svelte';

  const pending = $derived(dialogs.confirmState);

  let confirmButton = $state<HTMLButtonElement>();
  let cancelButton = $state<HTMLButtonElement>();

  // Fokus beim Öffnen auf die Hauptaktion, beim Schließen zurück an den Ursprung.
  $effect(() => {
    if (!pending) return;
    const previous = document.activeElement;
    void tick().then(() => confirmButton?.focus());
    return () => {
      // War der Ursprung eine inzwischen entfernte Zeile, geht der Fokus an die Liste.
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
      else document.querySelector<HTMLElement>('[role="grid"], [role="tree"], [role="listbox"]')?.focus();
    };
  });

  function onKeydown(event: KeyboardEvent) {
    // Tastatur gehört dem Dialog, solange er offen ist.
    event.stopPropagation();
    if (event.key === 'Escape') {
      event.preventDefault();
      dialogs.answer(false);
    } else if (event.key === 'Tab') {
      event.preventDefault();
      (document.activeElement === confirmButton ? cancelButton : confirmButton)?.focus();
    }
    // Enter löst den fokussierten Knopf selbst aus.
  }
</script>

{#if pending}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="scrim" onclick={(event) => event.target === event.currentTarget && dialogs.answer(false)}>
    <div
      class="card"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="dlg-title"
      aria-describedby="dlg-body"
      tabindex="-1"
      onkeydown={onKeydown}
    >
      <h2 id="dlg-title">{pending.title}</h2>
      <p id="dlg-body">{pending.body}</p>
      <div class="actions">
        <button class="btn" bind:this={cancelButton} onclick={() => dialogs.answer(false)}>{pending.cancel}</button>
        <button
          class="btn pri"
          class:danger={pending.danger}
          bind:this={confirmButton}
          onclick={() => dialogs.answer(true)}
        >
          {pending.confirm}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--scrim);
    backdrop-filter: blur(6px);
    animation: fade var(--t-overlay) ease-out;
  }

  .card {
    width: min(380px, calc(100vw - 32px));
    padding: 22px;
    border-radius: var(--r-dialog);
    background: var(--surface);
    box-shadow:
      var(--shadow-lg),
      0 0 0 1px var(--border-strong);
    animation: rise var(--t-overlay) ease-out;
    outline: none;
  }

  h2 {
    margin: 0 0 6px;
    font-size: 16px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  p {
    margin: 0 0 18px;
    font-size: 13px;
    color: var(--muted);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: scale(0.95) translateY(8px);
    }
  }
</style>
