<script lang="ts">
  import type { CleanupResult, GeneratedAssetStats, LauncherEntry } from '$lib/types';

  export let maintenance: GeneratedAssetStats | null = null;
  export let maintenanceOpen = false;
  export let maintenanceBusy = false;
  export let embedded = false;
  export let lastCleanupResult: CleanupResult | null = null;
  export let onToggle: () => void;
  export let onRefresh: () => Promise<void> | void;
  export let onDryRun: () => Promise<void> | void;
  export let onCleanup: () => Promise<void> | void;
  export let bulkFixCandidateCount = 0;
  export let bulkFixBusy = false;
  export let bulkFixPreviewEntries: LauncherEntry[] = [];
  export let onBulkFixVisible: () => Promise<void> | void;

  function formatBytes(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }
</script>

<section class="panel utilityPanel" class:embeddedPanel={embedded}>
  {#if !embedded}
    <div class="panelHeader logHeader">
      <div class="panelTitle">
        Maintenance
        {#if maintenance}
          <span class="panelSubtle">{maintenance.orphanGeneratedIconsCount} orphaned</span>
        {/if}
      </div>

      <div class="panelActions">
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
    <div class="panelBody">
      {#if maintenance}
        <div class="quickStats">
          <div class="statCard" class:alertCard={maintenance.orphanGeneratedIconsCount > 0}>
            <div class="statLabel">Orphaned auto icons</div>
            <div class="statValue">{maintenance.orphanGeneratedIconsCount}</div>
            <div class="statSubtle">{formatBytes(maintenance.orphanGeneratedIconsBytes)}</div>
          </div>

          <div class="statCard">
            <div class="statLabel">Generated icons</div>
            <div class="statValue">{maintenance.generatedIconsCount}</div>
            <div class="statSubtle">{formatBytes(maintenance.generatedIconsBytes)}</div>
          </div>

          <div class="statCard">
            <div class="statLabel">Backups</div>
            <div class="statValue">{maintenance.backupsCount}</div>
            <div class="statSubtle">{formatBytes(maintenance.backupsBytes)}</div>
          </div>
        </div>

        <div class="actionGrid">
          <div class="contentCard actionCard">
            <div class="sectionHeader">
              <strong class="sectionTitle">Fix visible issues</strong>
              <span class="sectionMeta">
                {bulkFixBusy ? 'Running' : bulkFixCandidateCount > 0 ? `${bulkFixCandidateCount} ready` : 'Nothing to do'}
              </span>
            </div>

            <div class="sectionText">
              Works on the currently visible launcher list and respects the active search and filters.
            </div>

            {#if bulkFixCandidateCount > 0}
              <div class="compactPreviewList">
                {#each bulkFixPreviewEntries.slice(0, 5) as entry}
                  <div class="compactPreviewRow">
                    <div class="compactPreviewName">{entry.name}</div>
                    <div class="compactPreviewMeta">{entry.launcherSource}</div>
                  </div>
                {/each}

                {#if bulkFixPreviewEntries.length > 5}
                  <div class="compactPreviewMore">
                    + {bulkFixPreviewEntries.length - 5} more visible candidate(s)
                  </div>
                {/if}
              </div>
            {/if}

            <div class="actionRow">
              <button
                type="button"
                class="ghost utilityActionButton utilityActionButtonPrimary"
                on:click={onBulkFixVisible}
                disabled={maintenanceBusy || bulkFixBusy || bulkFixCandidateCount === 0}
              >
                {bulkFixBusy ? 'Fixing visible issues…' : 'Fix visible issues'}
              </button>
            </div>
          </div>

          <div class="contentCard actionCard">
            <div class="sectionHeader">
              <strong class="sectionTitle">Cleanup generated icons</strong>
              <span class="sectionMeta">{maintenanceBusy ? 'Running' : 'Ready'}</span>
            </div>

            <div class="sectionText">
              Review orphaned generated assets first, then remove them once the preview looks right.
            </div>

            <div class="resultGrid">
              <div class="resultCell">
                <span class="dataKey">Orphaned</span>
                <span class="resultValue">{maintenance.orphanGeneratedIconsCount}</span>
              </div>

              <div class="resultCell">
                <span class="dataKey">Size</span>
                <span class="resultValue">{formatBytes(maintenance.orphanGeneratedIconsBytes)}</span>
              </div>
            </div>

            <div class="actionRow">
              <button
                type="button"
                class="ghost utilityActionButton"
                on:click={onDryRun}
                disabled={maintenanceBusy || bulkFixBusy}
              >
                Dry run
              </button>

              <button
                type="button"
                class="ghost utilityActionButton utilityActionButtonPrimary"
                on:click={onCleanup}
                disabled={maintenanceBusy || bulkFixBusy}
              >
                Cleanup now
              </button>
            </div>
          </div>
        </div>

        <div class="contentGrid">
          <div class="contentCard">
            <div class="sectionHeader">
              <strong class="sectionTitle">Storage overview</strong>
              <span class="sectionMeta">{formatBytes(maintenance.totalBytes)}</span>
            </div>

            <div class="dataRow">
              <span class="dataKey">Generated</span>
              <span class="dataValue">{maintenance.generatedIconsCount} · {formatBytes(maintenance.generatedIconsBytes)}</span>
            </div>

            <div class="dataRow">
              <span class="dataKey">Manual</span>
              <span class="dataValue">{maintenance.manualIconsCount} · {formatBytes(maintenance.manualIconsBytes)}</span>
            </div>

            <div class="dataRow">
              <span class="dataKey">Backups</span>
              <span class="dataValue">{maintenance.backupsCount} · {formatBytes(maintenance.backupsBytes)}</span>
            </div>
          </div>

          {#if lastCleanupResult}
            <div class="contentCard">
              <div class="sectionHeader">
                <strong class="sectionTitle">Last cleanup</strong>
                <span class="sectionMeta">{lastCleanupResult.dryRun ? 'Dry run' : 'Applied'}</span>
              </div>

              <div class="dataRow">
                <span class="dataKey">Removed files</span>
                <span class="dataValue">{lastCleanupResult.removedFilesCount}</span>
              </div>

              <div class="dataRow">
                <span class="dataKey">Removed size</span>
                <span class="dataValue">{formatBytes(lastCleanupResult.removedBytes)}</span>
              </div>

              {#if lastCleanupResult.removedPaths.length > 0}
                <div class="dataRow">
                  <span class="dataKey">First path</span>
                  <span class="dataValue code">{lastCleanupResult.removedPaths[0]}</span>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {:else}
        <div class="contentCard emptyCard">
          <div class="empty compact">
            <strong>No maintenance data loaded yet</strong>
            <span>Use refresh to inspect generated assets and orphaned auto icons.</span>
          </div>
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
    gap: var(--utility-gap, 8px);
  }

  .statCard,
  .contentCard {
    border: var(--utility-card-border, 1px solid rgba(255, 255, 255, 0.08));
    border-radius: var(--utility-card-radius, 11px);
    background: var(--utility-card-bg, rgba(255, 255, 255, 0.02));
    box-shadow: var(--utility-card-shadow, none);
    padding: var(--utility-card-padding, 9px 11px);
    min-width: 0;
  }

  .quickStats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--utility-gap, 8px);
  }

  .statCard {
    display: grid;
    gap: 4px;
  }

  .statLabel {
    font-size: 0.73rem;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
  }

  .statValue {
    font-size: 0.96rem;
    font-weight: 700;
    line-height: 1.25;
  }

  .statSubtle {
    font-size: 0.76rem;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
  }

  .alertCard {
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.045), rgba(255, 255, 255, 0.022)),
      rgba(255, 255, 255, 0.02);
  }

  .actionGrid,
  .contentGrid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--utility-gap, 8px);
  }

  .actionCard {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .sectionHeader {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
  }

  .sectionTitle {
    font-size: 0.81rem;
    font-weight: 700;
    line-height: 1.2;
  }

  .sectionMeta {
    font-size: 0.74rem;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.76));
    padding: 2px 7px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.04);
    white-space: nowrap;
  }

  .sectionText {
    font-size: 0.79rem;
    line-height: 1.36;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
  }

  .compactPreviewList {
    margin-top: 12px;
    display: grid;
    gap: 6px;
  }

  .compactPreviewRow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
    align-items: center;
    padding: 7px 8px;
    border-radius: 9px;
    border: 1px solid rgba(255, 255, 255, 0.035);
    background: rgba(255, 255, 255, 0.018);
  }

  .compactPreviewName {
    min-width: 0;
    font-size: 0.8rem;
    font-weight: 600;
    line-height: 1.25;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .compactPreviewMeta {
    font-size: 0.72rem;
    line-height: 1.2;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
    white-space: nowrap;
  }

  .compactPreviewMore {
    font-size: 0.75rem;
    line-height: 1.3;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
    padding: 1px 2px 0;
  }

  .actionRow {
    display: flex;
    gap: 7px;
    flex-wrap: wrap;
    margin-top: 12px;
  }

  .utilityActionButton {
    min-width: 152px;
    min-height: 34px;
    background: rgba(255, 255, 255, 0.025);
    border-color: rgba(255, 255, 255, 0.07);
    font-size: 0.8rem;
  }

  .utilityActionButton:hover:enabled {
    background: rgba(255, 255, 255, 0.05);
  }

  .utilityActionButtonPrimary {
    background: rgba(255, 255, 255, 0.07);
    color: var(--utility-strong-text, rgba(255, 255, 255, 0.95));
  }

  .utilityActionButtonPrimary:hover:enabled {
    background: rgba(255, 255, 255, 0.1);
  }

  .resultGrid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    margin-top: 12px;
  }

  .resultCell {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 9px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.022);
    border: 1px solid rgba(255, 255, 255, 0.04);
  }

  .resultValue {
    font-size: 0.88rem;
    font-weight: 600;
    line-height: 1.3;
  }

  .dataRow {
    display: grid;
    grid-template-columns: 92px minmax(0, 1fr);
    gap: 10px;
    margin-top: 7px;
  }

  .dataKey {
    font-size: 0.75rem;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
  }

  .dataValue {
    font-size: 0.82rem;
    line-height: 1.36;
    word-break: break-word;
    min-width: 0;
  }

  .code {
    font-family: monospace;
    font-size: 0.79rem;
  }

  .emptyCard {
    padding: 13px;
  }

  @media (max-width: 1100px) {
    .quickStats,
    .actionGrid,
    .contentGrid,
    .resultGrid {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 900px) {
    .utilityActionButton {
      width: 100%;
      min-width: 0;
    }

    .compactPreviewRow {
      grid-template-columns: 1fr;
    }
  }
</style>
