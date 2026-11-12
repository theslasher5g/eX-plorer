<script lang="ts">
  import { onMount } from 'svelte';
  import { ChevronLeft, ChevronRight } from '@lucide/svelte';
  import { t } from '../i18n';

  let { data, onerror }: { data: ArrayBuffer; onerror: () => void } = $props();

  // Schwere Bibliothek erst laden, wenn wirklich ein PDF angezeigt wird.
  type Pdfjs = typeof import('pdfjs-dist');
  type Doc = Awaited<ReturnType<Pdfjs['getDocument']>['promise']>;
  type RenderTask = { cancel(): void; promise: Promise<unknown> };

  let host = $state<HTMLDivElement>();
  let canvas = $state<HTMLCanvasElement>();
  let doc = $state.raw<Doc | null>(null);
  let page = $state(1);
  let total = $state(0);
  let size = $state({ width: 0, height: 0 });
  let task: RenderTask | null = null;

  onMount(() => {
    let disposed = false;
    let loadingTask: { destroy(): Promise<void> } | null = null;

    void (async () => {
      try {
        const pdfjs = await import('pdfjs-dist');
        const worker = (await import('pdfjs-dist/build/pdf.worker.min.mjs?url')).default;
        pdfjs.GlobalWorkerOptions.workerSrc = worker;
        const opened = pdfjs.getDocument({
          // Kopie, denn pdf.js übernimmt den Puffer.
          data: new Uint8Array(data.slice(0)),
          standardFontDataUrl: '/pdfjs/standard_fonts/',
          wasmUrl: '/pdfjs/wasm/',
        });
        loadingTask = opened;
        const loaded = await opened.promise;
        if (disposed) return;
        total = loaded.numPages;
        doc = loaded;
      } catch {
        if (!disposed) onerror();
      }
    })();

    return () => {
      disposed = true;
      task?.cancel();
      void loadingTask?.destroy();
    };
  });

  // Fenstergröße beobachten, damit die Seite immer hineinpasst.
  $effect(() => {
    if (!host) return;
    const observer = new ResizeObserver(([entry]) => {
      size = { width: entry.contentRect.width, height: entry.contentRect.height };
    });
    observer.observe(host);
    return () => observer.disconnect();
  });

  $effect(() => {
    const current = doc;
    const target = canvas;
    const { width, height } = size;
    const number = page;
    if (!current || !target || width < 50 || height < 50) return;

    let cancelled = false;
    void (async () => {
      try {
        const pdfPage = await current.getPage(number);
        if (cancelled) return;
        const base = pdfPage.getViewport({ scale: 1 });
        const fit = Math.min(width / base.width, height / base.height);
        const ratio = window.devicePixelRatio || 1;
        const viewport = pdfPage.getViewport({ scale: fit * ratio });
        target.width = Math.floor(viewport.width);
        target.height = Math.floor(viewport.height);
        target.style.width = `${Math.floor(viewport.width / ratio)}px`;
        target.style.height = `${Math.floor(viewport.height / ratio)}px`;
        task?.cancel();
        task = pdfPage.render({ canvas: target, viewport });
        await task.promise;
      } catch (err) {
        // Abbrechen beim Blättern ist normal; alles andere ist ein Fehler.
        if (!cancelled && (err as { name?: string })?.name !== 'RenderingCancelledException') onerror();
      }
    })();
    return () => {
      cancelled = true;
      task?.cancel();
    };
  });

  function turn(delta: number) {
    page = Math.max(1, Math.min(total, page + delta));
  }

  export function onKey(event: KeyboardEvent): boolean {
    if (event.key === 'PageDown') turn(1);
    else if (event.key === 'PageUp') turn(-1);
    else return false;
    return true;
  }
</script>

<div class="pdf">
  <div class="stage" bind:this={host}>
    <canvas bind:this={canvas} aria-label={t('ql.page', { page, total })}></canvas>
  </div>
  {#if total > 1}
    <div class="pages">
      <button class="icon-btn" title={t('ql.prevPage')} aria-label={t('ql.prevPage')} disabled={page <= 1} onclick={() => turn(-1)}>
        <ChevronLeft size={18} strokeWidth={1.7} />
      </button>
      <span>{t('ql.page', { page, total })}</span>
      <button class="icon-btn" title={t('ql.nextPage')} aria-label={t('ql.nextPage')} disabled={page >= total} onclick={() => turn(1)}>
        <ChevronRight size={18} strokeWidth={1.7} />
      </button>
    </div>
  {/if}
</div>

<style>
  .pdf {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-height: 0;
  }

  .stage {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  canvas {
    background: #fff;
    border-radius: 4px;
    box-shadow: var(--shadow-md);
  }

  .pages {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding-top: 8px;
    font-size: 12.5px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
</style>
