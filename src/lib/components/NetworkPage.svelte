<script lang="ts">
  import { untrack } from 'svelte';
  import { Globe, HardDrive, Lock, Monitor, Plus, RefreshCw, TriangleAlert, X } from '@lucide/svelte';
  import { getApi, toApiError, type NetDrive, type Reachability, type ShareInfo } from '../api';
  import { t } from '../i18n';
  import type { Pane } from '../stores/pane.svelte';
  import { ops } from '../stores/ops.svelte';
  import { settings } from '../stores/settings.svelte';

  let { pane }: { pane: Pane } = $props();

  type Status = 'checking' | 'online' | 'offline' | 'auth';

  interface DriveRow {
    drive: NetDrive;
    status: Status;
  }

  interface ServerRow {
    server: string;
    status: Status;
    shares: ShareInfo[];
  }

  let drives = $state<DriveRow[]>([]);
  let servers = $state<ServerRow[]>([]);
  let loadingDrives = $state(true);
  let draft = $state('');
  let draftError = $state(false);
  // Veraltete Antworten (nach erneutem Prüfen) werden verworfen.
  let generation = 0;

  const toStatus = (value: Reachability): Status =>
    value === 'online' ? 'online' : value === 'authrequired' ? 'auth' : 'offline';

  async function probeDrive(row: DriveRow, run: number) {
    row.status = 'checking';
    try {
      const status = toStatus(await (await getApi()).networkProbe(row.drive.path));
      if (run === generation) row.status = status;
    } catch {
      if (run === generation) row.status = 'offline';
    }
  }

  async function loadShares(row: ServerRow, run: number) {
    row.status = 'checking';
    try {
      const shares = await (await getApi()).networkShares(row.server);
      if (run !== generation) return;
      row.shares = shares;
      row.status = 'online';
    } catch (err) {
      if (run !== generation) return;
      row.shares = [];
      row.status = toApiError(err).code === 'auth_required' ? 'auth' : 'offline';
    }
  }

  // Alles neu prüfen: beim Öffnen der Seite und bei „Aktualisieren“ (F5).
  $effect(() => {
    void pane.revision;
    void settings.value.servers.join('|');
    const run = ++generation;
    untrack(() => {
      void (async () => {
        try {
          const list = await (await getApi()).networkDrives();
          if (run !== generation) return;
          drives = list.map((drive) => ({ drive, status: 'checking' as Status }));
        } catch {
          drives = [];
        }
        loadingDrives = false;
        // Jedes Laufwerk und jeder Server wird für sich und gleichzeitig geprüft:
        // Ein toter Eintrag hält die anderen nicht auf.
        drives.forEach((row) => void probeDrive(row, run));
      })();
      servers = settings.value.servers.map((server) => ({ server, status: 'checking' as Status, shares: [] }));
      servers.forEach((row) => void loadShares(row, run));
    });
  });

  async function login(target: string, after: () => Promise<void>) {
    if (await ops.connect(target)) await after();
  }

  async function addServer(event: SubmitEvent) {
    event.preventDefault();
    const input = draft.trim();
    if (!input) return;
    try {
      const parsed = await (await getApi()).networkParse(input);
      draftError = false;
      draft = '';
      const known = settings.value.servers.some((s) => s.toLowerCase() === parsed.server.toLowerCase());
      if (!known) settings.update({ servers: [...settings.value.servers, parsed.server] });
      // Mit Freigabe: gleich dorthin.
      if (parsed.share) void pane.navigate(parsed.share);
    } catch {
      draftError = true;
    }
  }

  function removeServer(server: string) {
    settings.update({ servers: settings.value.servers.filter((s) => s !== server) });
  }

  const driveTitle = (drive: NetDrive) => drive.remote ?? `${drive.letter}:`;
  const serverName = (server: string) => server.replace(/^\\+/, '');
</script>

{#snippet pill(status: Status)}
  {#if status === 'online'}
    <span class="pill ok"><span class="dot"></span>{t('network.connected')}</span>
  {:else if status === 'checking'}
    <span class="pill busy"><span class="spin"><RefreshCw size={14} strokeWidth={1.7} /></span>{t('network.checking')}</span>
  {:else if status === 'offline'}
    <span class="pill err"><TriangleAlert size={14} strokeWidth={1.7} />{t('network.offline')}</span>
  {:else}
    <span class="pill warn"><Lock size={14} strokeWidth={1.7} />{t('network.auth')}</span>
  {/if}
{/snippet}

<div class="netwrap" role="region" aria-label={t('sidebar.network')}>
  <h3 class="sechead">{t('network.drives')}</h3>
  {#if loadingDrives}
    <p class="hint">{t('list.loading')}</p>
  {:else if drives.length === 0}
    <p class="hint">{t('network.noDrives')}</p>
  {/if}
  {#each drives as row (row.drive.path)}
    <div class="netrow" data-status={row.status}>
      <span class="driveic"><HardDrive size={22} strokeWidth={1.7} /></span>
      <span class="nettxt">
        <b>{driveTitle(row.drive)}</b>
        <span>{row.drive.path}</span>
      </span>
      {@render pill(row.status)}
      {#if row.status === 'online'}
        <button class="btn" onclick={() => pane.navigate(row.drive.path)}>{t('network.open')}</button>
      {:else if row.status === 'offline'}
        <button class="btn" onclick={() => probeDrive(row, generation)}>
          <RefreshCw size={16} strokeWidth={1.7} />{t('network.retry')}
        </button>
      {:else if row.status === 'auth'}
        <button class="btn pri" onclick={() => login(row.drive.path, () => probeDrive(row, generation))}>
          {t('network.login')}
        </button>
      {/if}
    </div>
  {/each}

  <h3 class="sechead second">{t('network.shares')}</h3>
  {#if servers.length === 0}
    <p class="hint">{t('network.noServers')}</p>
  {/if}
  {#each servers as row (row.server)}
    <section class="server" data-status={row.status}>
      <header>
        <Monitor size={16} strokeWidth={1.7} />
        <b>{serverName(row.server)}</b>
        {@render pill(row.status)}
        {#if row.status === 'offline'}
          <button class="btn" onclick={() => loadShares(row, generation)}>
            <RefreshCw size={16} strokeWidth={1.7} />{t('network.retry')}
          </button>
        {:else if row.status === 'auth'}
          <button
            class="btn pri"
            onclick={() => login(`${row.server}\\IPC$`, () => loadShares(row, generation))}
          >
            {t('network.login')}
          </button>
        {/if}
        <button
          class="icon-btn remove"
          title={t('network.remove')}
          aria-label="{t('network.remove')}: {serverName(row.server)}"
          onclick={() => removeServer(row.server)}
        >
          <X size={16} strokeWidth={1.7} />
        </button>
      </header>
      {#if row.status === 'online'}
        {#if row.shares.length === 0}
          <p class="hint">{t('network.noShares')}</p>
        {:else}
          <div class="sharegrid">
            {#each row.shares as share (share.path)}
              <button class="share" onclick={() => pane.navigate(share.path)}>
                <span class="driveic"><Globe size={22} strokeWidth={1.7} /></span>
                <span class="sname">
                  <b>{share.name}</b>
                  {#if share.comment}<small>{share.comment}</small>{/if}
                </span>
              </button>
            {/each}
          </div>
        {/if}
      {/if}
    </section>
  {/each}

  <form class="add" onsubmit={addServer}>
    <label class="field" class:bad={draftError}>
      <span class="sr">{t('network.addLabel')}</span>
      <input
        type="text"
        bind:value={draft}
        placeholder={t('network.addPlaceholder')}
        spellcheck="false"
        autocomplete="off"
        aria-invalid={draftError}
        oninput={() => (draftError = false)}
      />
    </label>
    <button class="btn" type="submit" disabled={!draft.trim()}>
      <Plus size={16} strokeWidth={1.7} />{t('network.add')}
    </button>
  </form>
  {#if draftError}
    <p class="invalid" role="alert">{t('network.invalid')}</p>
  {/if}

  <p class="note">{t('network.hint')}</p>
</div>

<style>
  .netwrap {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 20px 24px 28px;
  }

  .sechead {
    margin: 0 0 10px 4px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }

  .sechead.second {
    margin-top: 28px;
  }

  .hint {
    margin: 0 0 8px 4px;
    font-size: 13px;
    color: var(--muted);
  }

  .netrow {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px 14px;
    padding: 12px 14px;
    margin-bottom: 8px;
    border-radius: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-sm);
  }

  .driveic {
    width: 40px;
    height: 40px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--r-menu);
    background: var(--accent-soft);
    color: var(--accent);
  }

  .nettxt {
    flex: 1;
    min-width: 160px;
  }

  .nettxt b {
    display: block;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .nettxt span {
    font-size: 12.5px;
    color: var(--muted);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 10px;
    border-radius: 12px;
    font-size: 12.5px;
    white-space: nowrap;
  }

  .pill.ok {
    background: var(--ok-bg);
    color: var(--ok);
  }

  .pill.busy {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .pill.err {
    background: var(--err-bg);
    color: var(--err);
  }

  .pill.warn {
    background: var(--warn-bg);
    color: var(--warn);
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
  }

  .spin {
    display: flex;
    animation: spin 1s linear infinite;
  }

  .server {
    margin-bottom: 14px;
  }

  .server header {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 12px;
    margin: 0 0 8px 4px;
  }

  .server header b {
    font-weight: 600;
  }

  .remove {
    margin-left: auto;
    width: 28px;
    height: 28px;
  }

  .sharegrid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 10px;
  }

  .share {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    border-radius: 12px;
    text-align: left;
    transition: background var(--t-hover);
  }

  .share:hover {
    background: var(--hover);
  }

  .sname {
    min-width: 0;
  }

  .sname b {
    display: block;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sname small {
    display: block;
    font-size: 12px;
    color: var(--muted);
  }

  .add {
    display: flex;
    gap: 8px;
    margin: 18px 0 0;
    max-width: 460px;
  }

  .field {
    flex: 1;
    min-width: 0;
    height: 34px;
    display: flex;
    align-items: center;
    padding: 0 10px;
    border-radius: var(--r-btn);
    background: var(--surface-2);
    transition:
      box-shadow var(--t-hover),
      background var(--t-hover);
  }

  .field:focus-within {
    background: var(--surface);
    box-shadow: 0 0 0 2px var(--accent);
  }

  .field.bad {
    box-shadow: 0 0 0 2px var(--err);
  }

  .field input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    user-select: text;
    -webkit-user-select: text;
  }

  .invalid {
    margin: 6px 0 0 4px;
    font-size: 12.5px;
    color: var(--err);
  }

  .note {
    margin: 28px 4px 0;
    max-width: 560px;
    font-size: 12.5px;
    color: var(--muted);
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
