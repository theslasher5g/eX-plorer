<script lang="ts">
  import { Check, CircleAlert } from '@lucide/svelte';
  import { toasts } from '../stores/toast.svelte';
</script>

<div class="live" role="status" aria-live="polite">
  {#if toasts.current}
    {#key toasts.current.id}
      <div class="toast">
        {#if toasts.current.kind === 'err'}
          <CircleAlert size={16} strokeWidth={1.7} />
        {:else}
          <Check size={16} strokeWidth={1.7} />
        {/if}
        <span>{toasts.current.text}</span>
      </div>
    {/key}
  {/if}
</div>

<style>
  .live {
    position: fixed;
    left: 50%;
    bottom: 48px;
    transform: translateX(-50%);
    z-index: 80;
    pointer-events: none;
    max-width: calc(100vw - 32px);
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 40px;
    padding: 9px 16px;
    border-radius: 20px;
    background: var(--text);
    color: var(--surface);
    box-shadow: var(--shadow-md);
    font-size: 13.5px;
    animation: in 200ms ease-out;
  }

  .toast :global(svg) {
    flex: none;
  }

  @keyframes in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
</style>
