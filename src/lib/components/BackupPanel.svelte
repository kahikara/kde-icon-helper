<script lang="ts">
  import type { BackupEntry } from '$lib/types';

  export let backups: BackupEntry[] = [];
  export let backupsOpen = false;
  export let backupsBusy = false;
  export let backupsRestoreBusy = false;
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

<section class="panel diagnosticsPanel">
  <div class="panelHeader logHeader">
    <div class="panelTitle">
      Backups
      {#if backups.length > 0}
        <span class="diagSummary">{backups.length} total · {restorableCount} restorable</span>
      {/if}
    </div>

    <div class="diagActions">
      <button type="button" class="ghost" on:click={onRefresh} disabled={backupsBusy || backupsRestoreBusy}>
        {backupsBusy ? 'Refreshing…' : 'Refresh'}
      </button>
      <button type="button" class="ghost" on:click={onToggle}>
        {backupsOpen ? 'Hide' : 'Show'}
      </button>
    </div>
  </div>

  {#if backupsOpen}
    <div class="diagScroll">
      {#if backups.length === 0}
        <div class="empty compact">
          <strong>No backups yet</strong>
          <span>Fixes and restore actions that create backups will show up here.</span>
        </div>
      {:else}
        <div class="diagCard" style="margin-bottom: 8px;">
          <div style="display:flex; gap:8px; flex-wrap:wrap;">
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
          <div class="diagTable">
            <div class="diagCard">
              <div class="diagTopRow">
                <strong>Backup list</strong>
                <span>{filteredBackups.length}</span>
              </div>

              <div style="display:flex; flex-direction:column; gap:6px; max-height:320px; overflow:auto;">
                {#each filteredBackups as backup}
                  <button
                    type="button"
                    class="ghost"
                    style="text-align:left; justify-content:flex-start;"
                    on:click={() => onSelect(backup.path)}
                  >
                    <span style="display:flex; flex-direction:column; align-items:flex-start; gap:2px;">
                      <strong>
                        {selectedBackup?.path === backup.path ? '● ' : ''}{backup.name}
                      </strong>
                      <span style="font-size:0.78rem; opacity:0.82;">
                        {relativeTime(backup.modifiedUnixMs)} · {backup.modifiedDisplay}
                      </span>
                      <span style="font-size:0.78rem; opacity:0.82;">
                        {backup.fileKind} · {formatBytes(backup.sizeBytes)} · {backup.restoreAvailable ? 'restorable' : 'read only'}
                      </span>
                    </span>
                  </button>
                {/each}
              </div>
            </div>

            {#if selectedBackup}
              <div class="diagCard">
                <div class="diagTopRow">
                  <strong>Selected backup</strong>
                  <span>{selectedBackup.fileKind}</span>
                </div>

                <div class="diagLine">
                  <span class="diagKey">Name</span>
                  <span class="diagValue">{selectedBackup.name}</span>
                </div>

                <div class="diagLine">
                  <span class="diagKey">Modified</span>
                  <span class="diagValue">
                    {selectedBackup.modifiedDisplay} · {relativeTime(selectedBackup.modifiedUnixMs)}
                  </span>
                </div>

                <div class="diagLine">
                  <span class="diagKey">Size</span>
                  <span class="diagValue">{formatBytes(selectedBackup.sizeBytes)}</span>
                </div>

                <div class="diagLine">
                  <span class="diagKey">Original path</span>
                  <span class="diagValue code">{selectedBackup.originalPath ?? 'Unknown'}</span>
                </div>

                <div class="diagLine">
                  <span class="diagKey">Backup path</span>
                  <span class="diagValue code">{selectedBackup.path}</span>
                </div>

                <div class="diagLine">
                  <span class="diagKey">Restore</span>
                  <span class="diagValue">
                    {#if selectedBackup.restoreAvailable}
                      Available
                    {:else}
                      {selectedBackup.restoreReason ?? 'Not available'}
                    {/if}
                  </span>
                </div>

                <div class="diagActions" style="margin-top: 10px;">
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
              </div>
            {/if}
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</section>
