<script lang="ts">
  import { Folder, X } from '@lucide/svelte';
  import { getApi, toApiError, type Entry } from '../api';
  import { typeLabel } from '../filetypes';
  import { formatDate, formatSize } from '../format';
  import { shellImageUrl } from '../images';
  import { errorText, t } from '../i18n';
  import { joinPath } from '../paths';
  import { onedrive } from '../stores/onedrive.svelte';
  import { ops } from '../stores/ops.svelte';
  import { quicklook } from '../stores/quicklook.svelte';
  import FileIcon from './FileIcon.svelte';
  import PdfView from './PdfView.svelte';

  type View =
    | { kind: 'loading' }
    | { kind: 'image'; url: string }
    | { kind: 'text'; text: string; truncated: boolean }
    | { kind: 'pdf'; data: ArrayBuffer }
    | { kind: 'generic'; note?: string }
    | { kind: 'message'; text: string };

  const MAX_DOTS = 12;

  let view = $state.raw<View>({ kind: 'loading' });
  let pdf = $state<ReturnType<typeof PdfView>>();
  let closeButton = $state<HTMLButtonElement>();

  const pane = $derived(quicklook.pane);
  const entry = $derived(quicklook.entry);
  const dir = $derived(pane?.path ?? '');
  const path = $derived(entry ? joinPath(dir, entry.name) : '');
  const total = $derived(quicklook.entries.length);

  const meta = $derived.by(() => {
    if (!entry) return '';
    const parts = [typeLabel(entry)];
    if (!entry.isDir) parts.push(formatSize(entry.size));
    if (entry.modifiedMs !== null) parts.push(formatDate(entry.modifiedMs));
    const sync = onedrive.stateOf(entry, dir);
    if (sync) parts.push(t(`sync.${sync}`));
    return parts.join(' · ');
  });

  // Inhalt des aktuellen Eintrags laden. Veraltete Antworten werden verworfen.
  $effect(() => {
    const target = path;
    if (!target) return;
    let cancelled = false;
    let blobUrl: string | null = null;
    view = { kind: 'loading' };

    void (async () => {
      try {
        const api = await getApi();
        const info = await api.previewInfo(target);
        if (cancelled) return;
        if (info.isDir) {
          view = { kind: 'generic' };
        } else if (info.cloudOnly) {
          view = { kind: 'message', text: t('ql.cloud') };
        } else if (info.kind === 'image' && !info.tooLarge) {
          const bytes = await api.previewBytes(target);
          if (cancelled) return;
          blobUrl = URL.createObjectURL(new Blob([bytes], { type: info.mime ?? 'image/png' }));
          view = { kind: 'image', url: blobUrl };
        } else if (info.kind === 'pdf' && !info.tooLarge) {
          const data = await api.previewBytes(target);
          if (cancelled) return;
          view = { kind: 'pdf', data };
        } else if (info.kind === 'text') {
          const text = await api.previewText(target);
          if (cancelled) return;
          view = { kind: 'text', text: text.text, truncated: text.truncated };
        } else {
          view = { kind: 'generic', note: info.tooLarge ? t('ql.tooLarge') : undefined };
        }
      } catch (err) {
        if (cancelled) return;
        const { code } = toApiError(err);
        // Binärdaten oder unbekannter Typ: kein Fehler, nur keine Vorschau.
        view = code === 'unsupported' ? { kind: 'generic' } : { kind: 'message', text: errorText(code) };
      }
    })();

    return () => {
      cancelled = true;
      // Verzögert freigeben: das Bild im DOM kann noch einen Frame lang geladen werden.
      const stale = blobUrl;
      if (stale) setTimeout(() => URL.revokeObjectURL(stale), 1000);
    };
  });

  // Fokus in den Dialog, danach zurück an den Ursprung.
  $effect(() => {
    if (!pane) return;
    const previous = document.activeElement;
    closeButton?.focus();
    return () => {
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
      else document.querySelector<HTMLElement>('[role="grid"], [role="tree"], [role="listbox"]')?.focus();
    };
  });

  function openEntry(target: Entry) {
    const current = pane;
    quicklook.close();
    if (current) void ops.open(current, target);
  }

  function onKeydown(event: KeyboardEvent) {
    if (!quicklook.isOpen) return;
    let handled = true;
    switch (event.key) {
      case 'Escape':
      case ' ':
        quicklook.close();
        break;
      case 'ArrowLeft':
      case 'ArrowUp':
        quicklook.move(-1);
        break;
      case 'ArrowRight':
      case 'ArrowDown':
        quicklook.move(1);
        break;
      case 'Enter':
        // Auf einem Knopf löst Enter diesen aus.
        if (event.target instanceof HTMLButtonElement) return;
        if (entry) openEntry(entry);
        break;
      case 'Tab':
        // Fokus bleibt im Dialog.
        event.preventDefault();
        return event.stopPropagation();
      default:
        handled = pdf?.onKey(event) ?? false;
    }
    if (handled) {
      event.preventDefault();
      event.stopPropagation();
    }
  }
</script>

<svelte:window onkeydowncapture={onKeydown} />

{#if pane && entry}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="scrim" onclick={(event) => event.target === event.currentTarget && quicklook.close()}>
    <div class="card" role="dialog" aria-modal="true" aria-label={t('ql.title')}>
      <div class="qhead">
        <FileIcon {entry} {dir} />
        <span class="title">
          <b title={entry.name}>{entry.name}</b>
          <small>{meta}</small>
        </span>
        {#if !entry.isDir}
          <button class="btn" onclick={() => openEntry(entry)}>{t('ql.open')}</button>
        {/if}
        <button
          bind:this={closeButton}
          class="icon-btn"
          title={t('ql.close')}
          aria-label={t('ql.close')}
          onclick={() => quicklook.close()}
        >
          <X size={18} strokeWidth={1.7} />
        </button>
      </div>

      <div class="qbody">
        {#if view.kind === 'loading'}
          <p class="note">{t('ql.loading')}</p>
        {:else if view.kind === 'image'}
          <img class="pic" src={view.url} alt={entry.name} />
        {:else if view.kind === 'text'}
          <pre class="txt" tabindex="-1">{view.text}</pre>
          {#if view.truncated}<span class="cut">{t('ql.truncated')}</span>{/if}
        {:else if view.kind === 'pdf'}
          <PdfView bind:this={pdf} data={view.data} onerror={() => (view = { kind: 'message', text: t('ql.pdfFailed') })} />
        {:else if view.kind === 'message'}
          <p class="note">{view.text}</p>
        {:else}
          <div class="generic">
            <div class="art">
              {#if entry.isDir}
                <Folder size={56} strokeWidth={1.4} />
              {:else}
                <FileIcon {entry} {dir} />
                <img class="thumb" src={shellImageUrl(path, 256, entry.modifiedMs)} alt="" onerror={(e) => e.currentTarget.remove()} />
              {/if}
            </div>
            <b>{entry.name}</b>
            <dl>
              <dt>{t('ql.kind')}</dt>
              <dd>{typeLabel(entry)}</dd>
              {#if !entry.isDir}
                <dt>{t('ql.size')}</dt>
                <dd>{formatSize(entry.size)}</dd>
              {/if}
              <dt>{t('ql.modified')}</dt>
              <dd>{formatDate(entry.modifiedMs)}</dd>
            </dl>
            {#if view.note}<p class="note">{view.note}</p>{/if}
          </div>
        {/if}
      </div>

      <div class="qfoot">
        <button class="icon-btn" title={t('ql.prev')} aria-label={t('ql.prev')} disabled={quicklook.index <= 0} onclick={() => quicklook.move(-1)}>
          <svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d="m15 18-6-6 6-6" /></svg>
        </button>
        {#if total <= MAX_DOTS}
          <div class="dots">
            {#each { length: total } as _, n (n)}
              <button class:on={n === quicklook.index} aria-label={t('ql.position', { n: n + 1, total })} onclick={() => quicklook.goto(n)}></button>
            {/each}
          </div>
        {:else}
          <span class="pos">{t('ql.position', { n: quicklook.index + 1, total })}</span>
        {/if}
        <button class="icon-btn" title={t('ql.next')} aria-label={t('ql.next')} disabled={quicklook.index >= total - 1} onclick={() => quicklook.move(1)}>
          <svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d="m9 18 6-6-6-6" /></svg>
        </button>
        <span class="hint">{t('ql.hint')}</span>
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
    padding: 24px;
    background: var(--scrim);
    backdrop-filter: blur(6px);
    animation: fade var(--t-overlay) ease-out;
  }

  .card {
    width: min(720px, 100%);
    max-height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: var(--r-dialog);
    background: var(--surface);
    box-shadow:
      var(--shadow-lg),
      0 0 0 1px var(--border-strong);
    animation: rise var(--t-overlay) ease-out;
  }

  .qhead {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px 12px 18px;
    border-bottom: 1px solid var(--border);
  }

  .title {
    flex: 1;
    min-width: 0;
  }

  .title b,
  .title small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title b {
    font-weight: 600;
  }

  .title small {
    font-size: 12px;
    color: var(--muted);
  }

  .qbody {
    position: relative;
    flex: none;
    height: min(68vh, 560px);
    padding: 20px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    background: var(--surface-2);
    overflow: hidden;
  }

  .pic {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 8px;
    box-shadow: var(--shadow-sm);
    user-select: none;
  }

  .txt {
    flex: 1;
    width: 100%;
    min-height: 0;
    margin: 0;
    padding: 18px 20px;
    overflow: auto;
    border-radius: 10px;
    background: var(--surface);
    font-family: var(--mono);
    font-size: 13px;
    line-height: 1.6;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
    -webkit-user-select: text;
    outline: none;
  }

  .cut,
  .note {
    margin: 0;
    font-size: 12.5px;
    color: var(--muted);
    text-align: center;
  }

  .generic {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    max-width: 100%;
  }

  .art {
    position: relative;
    display: grid;
    place-items: center;
    width: 160px;
    height: 120px;
    border-radius: 12px;
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    color: var(--folder-back);
    transform-origin: center;
  }

  .art :global(.fi) {
    transform: scale(2.2);
  }

  .thumb {
    position: absolute;
    inset: 8px;
    width: calc(100% - 16px);
    height: calc(100% - 16px);
    object-fit: contain;
    background: var(--surface);
  }

  .generic b {
    max-width: 100%;
    font-weight: 600;
    text-align: center;
    overflow-wrap: anywhere;
  }

  dl {
    display: grid;
    grid-template-columns: auto auto;
    gap: 6px 16px;
    margin: 0;
    font-size: 13px;
  }

  dt {
    color: var(--muted);
  }

  dd {
    margin: 0;
    text-align: right;
  }

  .qfoot {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 14px;
    padding: 10px;
    font-size: 12.5px;
    color: var(--muted);
  }

  .qfoot .ic {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.7;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .dots {
    display: flex;
    gap: 6px;
  }

  .dots button {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--track-soft);
  }

  .dots button.on {
    background: var(--accent);
  }

  .pos {
    font-variant-numeric: tabular-nums;
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
