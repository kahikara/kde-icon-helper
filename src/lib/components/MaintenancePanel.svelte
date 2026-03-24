<script lang="ts">
  import type { CleanupResult, GeneratedAssetStats } from '$lib/types';

  export let maintenance: GeneratedAssetStats | null = null;
  export let maintenanceOpen = false;
  export let maintenanceBusy = false;
  export let embedded = false;
  export let lastCleanupResult: CleanupResult | null = null;
  export let onToggle: () => void;
  export let onRefresh: () => Promise<void> | void;
  export let onDryRun: () => Promise<void> | void;
  export let onCleanup: () => Promise<void> | void;

  function formatBytes(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }
</script>

<section class="panel diagnosticsPanel" class:embeddedPanel={embedded}>
  {#if !embedded}
    <div class="panelHeader logHeader">
      <div class="panelTitle">
        Maintenance
        {#if maintenance}
          <span class="diagSummary">
            {maintenance.orphanGeneratedIconsCount} orphaned auto icon(s)
          </span>
        {/if}
      </div>

      <div class="diagActions">
        <button type="button" class="ghost" on:click={onRefresh} disabled={maintenanceBusy}>
          {maintenanceBusy ? 'Refreshing…' : 'Refresh'}
        </button>
        <button type="button" class="ghost" on:click={onToggle}>
          {maintenanceOpen ? 'Hide' : 'Show'}
        </button>
      </div>
    </div>
  {/if}

  {#if embedded || maintenanceOpen}
    <div class="diagScroll">
      {#if maintenance}
        <div class="diagTable">
          <div class="diagCard">
            <div class="diagTopRow">
              <strong>Generated auto icons</strong>
              <span>{maintenance.generatedIconsCount}</span>
            </div>
            <div class="diagLine">
              <span class="diagKey">Size</span>
              <span class="diagValue">{formatBytes(maintenance.generatedIconsBytes)}</span>
            </div>
          </div>

          <div class="diagCard">
            <div class="diagTopRow">
              <strong>Manual icons</strong>
              <span>{maintenance.manualIconsCount}</span>
            </div>
            <div class="diagLine">
              <span class="diagKey">Size</span>
              <span class="diagValue">{formatBytes(maintenance.manualIconsBytes)}</span>
            </div>
          </div>

          <div class="diagCard">
            <div class="diagTopRow">
              <strong>Backups</strong>
              <span>{maintenance.backupsCount}</span>
            </div>
            <div class="diagLine">
              <span class="diagKey">Size</span>
              <span class="diagValue">{formatBytes(maintenance.backupsBytes)}</span>
            </div>
          </div>

          <div class="diagCard">
            <div class="diagTopRow">
              <strong>Orphaned auto icons</strong>
              <span class:bad={maintenance.orphanGeneratedIconsCount > 0}>
                {maintenance.orphanGeneratedIconsCount}
              </span>
            </div>
            <div class="diagLine">
              <span class="diagKey">Size</span>
              <span class="diagValue">{formatBytes(maintenance.orphanGeneratedIconsBytes)}</span>
            </div>
          </div>

          <div class="diagCard">
            <div class="diagTopRow">
              <strong>Total tracked size</strong>
              <span>{formatBytes(maintenance.totalBytes)}</span>
            </div>
            <div class="diagLine">
              <span class="diagKey">Scope</span>
              <span class="diagValue">Generated icons, manual icons, backups</span>
            </div>
          </div>
        </div>

        <div class="diagActions" style="margin-top: 10px;">
          <button type="button" class="ghost" on:click={onDryRun} disabled={maintenanceBusy}>
            Dry run cleanup
          </button>
          <button type="button" class="ghost" on:click={onCleanup} disabled={maintenanceBusy}>
            Cleanup orphaned auto icons
          </button>
        </div>

        {#if lastCleanupResult}
          <div class="diagCard" style="margin-top: 10px;">
            <div class="diagTopRow">
              <strong>Last cleanup result</strong>
              <span>{lastCleanupResult.dryRun ? 'Dry run' : 'Applied'}</span>
            </div>

            <div class="diagLine">
              <span class="diagKey">Removed files</span>
              <span class="diagValue">{lastCleanupResult.removedFilesCount}</span>
            </div>

            <div class="diagLine">
              <span class="diagKey">Removed size</span>
              <span class="diagValue">{formatBytes(lastCleanupResult.removedBytes)}</span>
            </div>

            {#if lastCleanupResult.removedPaths.length > 0}
              <div class="diagLine">
                <span class="diagKey">First path</span>
                <span class="diagValue code">{lastCleanupResult.removedPaths[0]}</span>
              </div>
            {/if}
          </div>
        {/if}
      {:else}
        <div class="empty compact">
          <strong>No maintenance data loaded yet</strong>
          <span>Use refresh to inspect generated assets and orphaned auto icons.</span>
        </div>
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
</style>
