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
  export let onRestore: () => Promise<void> | void;

  $: selectedBackup =
    backups.find((entry) => entry.path === selectedBackupPath) ?? backups[0] ?? null;
</script>

<section class="panel diagnosticsPanel">
  <div class="panelHeader logHeader">
    <div class="panelTitle">
      Backups
      {#if backups.length > 0}
        <span class="diagSummary">{backups.length} item(s)</span>
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
        <div class="diagTable">
          <div class="diagCard">
            <div class="diagTopRow">
              <strong>Backup list</strong>
              <span>{backups.length}</span>
            </div>

            <div style="display:flex; flex-direction:column; gap:6px; max-height:280px; overflow:auto;">
              {#each backups as backup}
                <button
                  type="button"
                  class="ghost"
                  style="text-align:left; justify-content:flex-start;"
                  on:click={() => onSelect(backup.path)}
                >
                  <span style="display:flex; flex-direction:column; align-items:flex-start; gap:2px;">
                    <strong>{backup.name}</strong>
                    <span style="font-size:0.78rem; opacity:0.82;">
                      {backup.modifiedDisplay} · {backup.fileKind} · {backup.sizeBytes} bytes
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
                <span class="diagValue">{selectedBackup.modifiedDisplay}</span>
              </div>

              <div class="diagLine">
                <span class="diagKey">Size</span>
                <span class="diagValue">{selectedBackup.sizeBytes} bytes</span>
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
                  Copy path
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
    </div>
  {/if}
</section>
