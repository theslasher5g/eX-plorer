<script lang="ts">
  import { Cloud, CircleCheck, RefreshCw } from '@lucide/svelte';
  import type { SyncState } from '../api';
  import { t } from '../i18n';

  // Zustand durch Form UND Farbe unterscheidbar: Wolke, Haken, gefüllter Haken, Pfeile.
  let { state, compact = false }: { state: SyncState; compact?: boolean } = $props();

  const label = $derived(t(`sync.${state}`));
</script>

<span class="sbadge {state}" class:compact title={compact ? label : undefined}>
  {#if state === 'cloud'}
    <Cloud size={16} strokeWidth={1.7} />
  {:else if state === 'local'}
    <CircleCheck size={16} strokeWidth={1.7} />
  {:else if state === 'pinned'}
    <svg class="pin" viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
      <circle cx="12" cy="12" r="9" fill="currentColor" />
      <path d="m8 12 3 3 5-6" fill="none" stroke="var(--surface)" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  {:else}
    <span class="spin"><RefreshCw size={16} strokeWidth={1.7} /></span>
  {/if}
  {#if compact}
    <span class="sr">{label}</span>
  {:else}
    {label}
  {/if}
</span>

<style>
  .sbadge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    white-space: nowrap;
  }

  .sbadge :global(svg) {
    flex: none;
  }

  .cloud,
  .syncing {
    color: var(--accent);
  }

  .local {
    color: var(--ok);
  }

  .pinned {
    color: var(--ok);
    font-weight: 600;
  }

  .spin {
    display: flex;
    animation: spin 1s linear infinite;
    transform-origin: center;
  }

  /* Als Eckmarke auf einer Kachel */
  .compact {
    padding: 3px;
    border-radius: 10px;
    background: var(--surface);
    box-shadow: var(--shadow-sm);
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
