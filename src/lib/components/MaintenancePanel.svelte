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

<section class="panel utilityPanel" class:embeddedPanel={embedded}>
  {#if !embedded}
    <div class="panelHeader logHeader">
      <div class="panelTitle">
        Maintenance
        {#if maintenance}
          <span class="panelSubtle">
            {maintenance.orphanGeneratedIconsCount} orphaned auto icon(s)
          </span>
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
      {#if embedded}
        <div class="introCard">
          <div class="introEyebrow">Maintenance</div>
          <div class="introRow">
            <strong class="introTitle">Review generated assets</strong>
            <span class="introMeta">
              {maintenance ? maintenance.orphanGeneratedIconsCount : 0} orphaned
            </span>
          </div>
          <div class="introText">
            Inspect stored assets and clean up orphaned generated icons when needed.
          </div>
        </div>
      {/if}

      {#if maintenance}
        <div class="summaryGrid">
          <div class="summaryCard">
            <div class="summaryTopRow">
              <div class="summaryLabel">Generated auto icons</div>
              <span class="summaryTone">Tracked</span>
            </div>
            <div class="summaryValue">{maintenance.generatedIconsCount}</div>
            <div class="summarySubtle">{formatBytes(maintenance.generatedIconsBytes)}</div>
          </div>

          <div class="summaryCard">
            <div class="summaryTopRow">
              <div class="summaryLabel">Manual icons</div>
              <span class="summaryTone">Manual</span>
            </div>
            <div class="summaryValue">{maintenance.manualIconsCount}</div>
            <div class="summarySubtle">{formatBytes(maintenance.manualIconsBytes)}</div>
          </div>

          <div class="summaryCard">
            <div class="summaryTopRow">
              <div class="summaryLabel">Backups</div>
              <span class="summaryTone">Safe</span>
            </div>
            <div class="summaryValue">{maintenance.backupsCount}</div>
            <div class="summarySubtle">{formatBytes(maintenance.backupsBytes)}</div>
          </div>

          <div class="summaryCard" class:alertCard={maintenance.orphanGeneratedIconsCount > 0}>
            <div class="summaryTopRow">
              <div class="summaryLabel">Orphaned auto icons</div>
              <span class="summaryTone" class:summaryToneAlert={maintenance.orphanGeneratedIconsCount > 0}>
                {maintenance.orphanGeneratedIconsCount > 0 ? 'Attention' : 'Clean'}
              </span>
            </div>
            <div class="summaryValue">{maintenance.orphanGeneratedIconsCount}</div>
            <div class="summarySubtle">{formatBytes(maintenance.orphanGeneratedIconsBytes)}</div>
          </div>
        </div>

        <div class="contentGrid">
          <div class="contentCard">
            <div class="sectionHeader">
              <strong class="sectionTitle">Storage overview</strong>
              <span class="sectionMeta">{formatBytes(maintenance.totalBytes)}</span>
            </div>

            <div class="sectionText">
              This space tracks generated icons, manually assigned icons and stored backups.
            </div>

            <div class="dataRow">
              <span class="dataKey">Generated</span>
              <span class="dataValue">
                {maintenance.generatedIconsCount} item(s) · {formatBytes(maintenance.generatedIconsBytes)}
              </span>
            </div>

            <div class="dataRow">
              <span class="dataKey">Manual</span>
              <span class="dataValue">
                {maintenance.manualIconsCount} item(s) · {formatBytes(maintenance.manualIconsBytes)}
              </span>
            </div>

            <div class="dataRow">
              <span class="dataKey">Backups</span>
              <span class="dataValue">
                {maintenance.backupsCount} item(s) · {formatBytes(maintenance.backupsBytes)}
              </span>
            </div>
          </div>

          <div class="contentCard">
            <div class="sectionHeader">
              <strong class="sectionTitle">Cleanup actions</strong>
              <span class="sectionMeta">{maintenanceBusy ? 'Running' : 'Ready'}</span>
            </div>

            <div class="sectionText">
              Start with a dry run to preview what would be removed. Apply cleanup only when the result looks right.
            </div>

            <div class="actionRow">
              <button type="button" class="ghost actionButton" on:click={onDryRun} disabled={maintenanceBusy}>
                Dry run cleanup
              </button>
              <button type="button" class="ghost actionButton" on:click={onCleanup} disabled={maintenanceBusy}>
                Cleanup orphaned auto icons
              </button>
            </div>
          </div>

          {#if lastCleanupResult}
            <div class="contentCard spanTwo">
              <div class="sectionHeader">
                <strong class="sectionTitle">Last cleanup result</strong>
                <span class="sectionMeta">{lastCleanupResult.dryRun ? 'Dry run' : 'Applied'}</span>
              </div>

              <div class="resultGrid">
                <div class="resultCell">
                  <span class="dataKey">Removed files</span>
                  <span class="resultValue">{lastCleanupResult.removedFilesCount}</span>
                </div>

                <div class="resultCell">
                  <span class="dataKey">Removed size</span>
                  <span class="resultValue">{formatBytes(lastCleanupResult.removedBytes)}</span>
                </div>
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
    gap: var(--utility-gap, 10px);
  }

  .introCard,
  .summaryCard,
  .contentCard {
    border: var(--utility-card-border, 1px solid rgba(255, 255, 255, 0.08));
    border-radius: var(--utility-card-radius, 12px);
    background: var(--utility-card-bg, rgba(255, 255, 255, 0.02));
    box-shadow: var(--utility-card-shadow, none);
    padding: var(--utility-card-padding, 10px 12px);
    min-width: 0;
  }

  .introEyebrow {
    font-size: 0.72rem;
    opacity: 0.68;
    margin-bottom: 4px;
  }

  .introRow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 4px;
  }

  .introTitle {
    font-size: 0.9rem;
    line-height: 1.2;
  }

  .introMeta,
  .sectionMeta,
  .summaryTone {
    font-size: 0.76rem;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.76));
    padding: 3px 8px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.04);
    white-space: nowrap;
  }

  .summaryToneAlert {
    color: var(--utility-strong-text, rgba(255, 255, 255, 0.95));
    background: rgba(255, 255, 255, 0.07);
  }

  .introText,
  .sectionText {
    font-size: 0.81rem;
    line-height: 1.4;
    color: var(--utility-soft-text, rgba(255, 255, 255, 0.74));
  }

  .summaryGrid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--utility-gap, 10px);
  }

  .summaryCard {
    min-height: 110px;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .summaryTopRow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
  }

  .alertCard {
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.05), rgba(255, 255, 255, 0.025)),
      rgba(255, 255, 255, 0.02);
  }

  .summaryLabel {
    font-size: 0.74rem;
    opacity: 0.72;
  }

  .summaryValue {
    font-size: 0.98rem;
    font-weight: 600;
    line-height: 1.35;
  }

  .summarySubtle {
    font-size: 0.78rem;
    opacity: 0.74;
    margin-top: 4px;
  }

  .contentGrid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--utility-gap, 10px);
  }

  .spanTwo {
    grid-column: 1 / -1;
  }

  .sectionHeader {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 10px;
  }

  .sectionTitle {
    font-size: 0.82rem;
    font-weight: 700;
    line-height: 1.2;
  }

  .dataRow {
    display: grid;
    grid-template-columns: 92px minmax(0, 1fr);
    gap: 10px;
    margin-top: 8px;
  }

  .dataKey {
    font-size: 0.76rem;
    opacity: 0.72;
  }

  .dataValue {
    font-size: 0.84rem;
    line-height: 1.4;
    word-break: break-word;
    min-width: 0;
  }

  .actionRow {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 14px;
  }

  .actionButton {
    min-width: 180px;
  }

  .resultGrid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
    margin-bottom: 2px;
  }

  .resultCell {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.025);
    border: 1px solid rgba(255, 255, 255, 0.04);
  }

  .resultValue {
    font-size: 0.9rem;
    font-weight: 600;
    line-height: 1.3;
  }

  .code {
    font-family: monospace;
    font-size: 0.8rem;
  }

  .emptyCard {
    padding: 14px;
  }

  @media (max-width: 1100px) {
    .summaryGrid,
    .contentGrid,
    .resultGrid {
      grid-template-columns: 1fr;
    }

    .summaryCard {
      min-height: auto;
    }

    .spanTwo {
      grid-column: auto;
    }
  }
</style>
