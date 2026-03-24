<script lang="ts">
  import type { BackupEntry } from '$lib/types';

  export let backups: BackupEntry[] = [];
  export let backupsOpen = false;
  export let backupsBusy = false;
  export let backupsRestoreBusy = false;
  export let embedded = false;
  export let selectedBackupPath: string | null = null;
  export let onToggle: () => void;
  export let onRefresh: () => Promise<void> | void;
  export let onSelect: (path: string) => void;
  export let onCopyPath: () => Promise<void> | void;
  export let onCopyOriginalPath: () => Promise<void> | void;
  export let onRestore: () => Promise<void> | void;

  let backupQuery = '';
  let backupFilter: 'all' | 'restorable' = 'all';

  function formatBytes(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }

  function relativeTime(unixMs: number) {
    const diffMs = Date.now() - unixMs;
    const diffSec = Math.max(0, Math.floor(diffMs / 1000));

    if (diffSec < 10) return 'just now';
    if (diffSec < 60) return `${diffSec}s ago`;

    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m ago`;

    const diffHour = Math.floor(diffMin / 60);
    if (diffHour < 24) return `${diffHour}h ago`;

    const diffDay = Math.floor(diffHour / 24);
    if (diffDay < 30) return `${diffDay}d ago`;

    const diffMonth = Math.floor(diffDay / 30);
    if (diffMonth < 12) return `${diffMonth}mo ago`;

    const diffYear = Math.floor(diffMonth / 12);
    return `${diffYear}y ago`;
  }

  $: filteredBackups = backups.filter((backup) => {
    const matchesQuery =
      backupQuery.trim() === '' ||
      `${backup.name} ${backup.path} ${backup.originalPath ?? ''} ${backup.fileKind}`
        .toLowerCase()
        .includes(backupQuery.toLowerCase());

    const matchesFilter = backupFilter === 'all' || backup.restoreAvailable;

    return matchesQuery && matchesFilter;
  });

  $: restorableCount = backups.filter((entry) => entry.restoreAvailable).length;

  $: selectedBackup =
    filteredBackups.find((entry) => entry.path === selectedBackupPath) ??
    backups.find((entry) => entry.path === selectedBackupPath) ??
    filteredBackups[0] ??
    backups[0] ??
    null;
</script>

<section class="panel utilityPanel" class:embeddedPanel={embedded}>
  {#if !embedded}
    <div class="panelHeader logHeader">
      <div class="panelTitle">
        Backups
        {#if backups.length > 0}
          <span class="panelSubtle">{backups.length} total · {restorableCount} restorable</span>
        {/if}
      </div>

      <div class="panelActions">
        <button type="button" class="ghost" on:click={onRefresh} disabled={backupsBusy || backupsRestoreBusy}>
          {backupsBusy ? 'Refreshing…' : 'Refresh'}
        </button>
        <button type="button" class="ghost" on:click={onToggle}>
          {backupsOpen ? 'Hide' : 'Show'}
        </button>
      </div>
    </div>
  {/if}

  {#if embedded || backupsOpen}
    <div class="panelBody">
      {#if backups.length === 0}
        <div class="empty compact">
          <strong>No backups yet</strong>
          <span>Fixes and restore actions that create backups will show up here.</span>
        </div>
      {:else}
        <div class="toolbarCard">
          <div class="toolbarRow">
            <input
              type="text"
              placeholder="Search backups"
              bind:value={backupQuery}
              style="flex:1; min-width:220px;"
            />

            <select bind:value={backupFilter} style="min-width:160px;">
              <option value="all">All backups</option>
              <option value="restorable">Restorable only</option>
            </select>
          </div>
        </div>

        {#if filteredBackups.length === 0}
          <div class="empty compact">
            <strong>No matching backups</strong>
            <span>Try a different search or filter.</span>
          </div>
        {:else}
          <div class="backupSplit">
            <div class="contentCard listPane">
              <div class="cardTopRow">
                <strong>Backup list</strong>
                <span>{filteredBackups.length}</span>
              </div>

              <div class="listScroll">
                {#each filteredBackups as backup}
                  <button
                    type="button"
                    class="ghost listButton"
                    on:click={() => onSelect(backup.path)}
                  >
                    <span class="listInner">
                      <strong class="listTitle">
                        {selectedBackup?.path === backup.path ? '● ' : ''}{backup.name}
                      </strong>
                      <span class="listMeta">
                        {relativeTime(backup.modifiedUnixMs)} · {backup.modifiedDisplay}
                      </span>
                      <span class="listMeta">
                        {backup.fileKind} · {formatBytes(backup.sizeBytes)} · {backup.restoreAvailable ? 'restorable' : 'read only'}
                      </span>
                    </span>
                  </button>
                {/each}
              </div>
            </div>

            <div class="contentCard detailsPane">
              <div class="cardTopRow">
                <strong>Selected backup</strong>
                <span>{selectedBackup?.fileKind ?? 'None'}</span>
              </div>

              {#if selectedBackup}
                <div class="detailsScroll">
                  <div class="dataRow">
                    <span class="dataKey">Name</span>
                    <span class="dataValue">{selectedBackup.name}</span>
                  </div>

                  <div class="dataRow">
                    <span class="dataKey">Modified</span>
                    <span class="dataValue">
                      {selectedBackup.modifiedDisplay} · {relativeTime(selectedBackup.modifiedUnixMs)}
                    </span>
                  </div>

                  <div class="dataRow">
                    <span class="dataKey">Size</span>
                    <span class="dataValue">{formatBytes(selectedBackup.sizeBytes)}</span>
                  </div>

                  <div class="dataRow">
                    <span class="dataKey">Original path</span>
                    <span class="dataValue code">{selectedBackup.originalPath ?? 'Unknown'}</span>
                  </div>

                  <div class="dataRow">
                    <span class="dataKey">Backup path</span>
                    <span class="dataValue code">{selectedBackup.path}</span>
                  </div>

                  <div class="dataRow">
                    <span class="dataKey">Restore</span>
                    <span class="dataValue">
                      {#if selectedBackup.restoreAvailable}
                        Available
                      {:else}
                        {selectedBackup.restoreReason ?? 'Not available'}
                      {/if}
                    </span>
                  </div>
                </div>

                <div class="actionRow detailsActions">
                  <button type="button" class="ghost" on:click={onCopyPath}>
                    Copy backup path
                  </button>

                  <button
                    type="button"
                    class="ghost"
                    on:click={onCopyOriginalPath}
                    disabled={!selectedBackup.originalPath}
                  >
                    Copy original path
                  </button>

                  <button
                    type="button"
                    class="ghost"
                    on:click={onRestore}
                    disabled={!selectedBackup.restoreAvailable || backupsRestoreBusy}
                  >
                    {backupsRestoreBusy ? 'Restoring…' : 'Restore backup'}
                  </button>
                </div>
              {:else}
                <div class="empty compact">
                  <strong>No backup selected</strong>
                  <span>Select an item from the list.</span>
                </div>
              {/if}
            </div>
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</section>

<style>
  .embeddedPanel {
    padding: 0;
    background: transparent;
    box-shadow: none;
    border: 0;
  }

  .panelSubtle {
    font-size: 0.78rem;
    opacity: 0.78;
    margin-left: 8px;
  }

  .panelActions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .panelBody {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .toolbarCard,
  .contentCard {
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.02);
    padding: 10px 12px;
  }

  .toolbarRow {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .backupSplit {
    display: grid;
    grid-template-columns: minmax(0, 1.05fr) minmax(0, 0.95fr);
    gap: 10px;
    align-items: stretch;
  }

  .listPane,
  .detailsPane {
    min-height: 430px;
    height: 430px;
    display: flex;
    flex-direction: column;
  }

  .cardTopRow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 10px;
  }

  .listScroll,
  .detailsScroll {
    overflow: auto;
    padding-right: 2px;
  }

  .listScroll {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .listButton {
    text-align: left;
    justify-content: flex-start;
  }

  .listInner {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
  }

  .listTitle {
    width: 100%;
    text-align: left;
  }

  .listMeta {
    font-size: 0.78rem;
    opacity: 0.82;
  }

  .dataRow {
    display: grid;
    grid-template-columns: 92px minmax(0, 1fr);
    gap: 10px;
    margin-top: 6px;
  }

  .dataKey {
    font-size: 0.76rem;
    opacity: 0.72;
  }

  .dataValue {
    font-size: 0.84rem;
    line-height: 1.4;
    word-break: break-word;
  }

  .code {
    font-family: monospace;
    font-size: 0.8rem;
  }

  .detailsActions {
    margin-top: auto;
    padding-top: 12px;
  }

  .actionRow {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  @media (max-width: 980px) {
    .backupSplit {
      grid-template-columns: 1fr;
    }

    .listPane,
    .detailsPane {
      min-height: 320px;
      height: auto;
    }
  }
</style>
