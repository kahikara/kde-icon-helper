<script lang="ts">
  import BackupPanel from '$lib/components/BackupPanel.svelte';
  import DiagnosticsPanel from '$lib/components/DiagnosticsPanel.svelte';
  import MaintenancePanel from '$lib/components/MaintenancePanel.svelte';
  import type {
    BackupEntry,
    CleanupResult,
    GeneratedAssetStats,
    RuntimeDiagnostics
  } from '$lib/types';

  export let open = false;
  export let activeTab: 'backups' | 'maintenance' | 'diagnostics' = 'backups';

  export let diagnostics: RuntimeDiagnostics | null = null;
  export let diagnosticsBusy = false;
  export let diagnosticsMissingCount = 0;

  export let maintenance: GeneratedAssetStats | null = null;
  export let maintenanceBusy = false;
  export let lastCleanupResult: CleanupResult | null = null;

  export let backups: BackupEntry[] = [];
  export let backupsBusy = false;
  export let backupsRestoreBusy = false;
  export let selectedBackupPath: string | null = null;

  export let onOpenTab: (tab: 'backups' | 'maintenance' | 'diagnostics') => void;
  export let onClose: () => void;

  export let onRefreshDiagnostics: () => Promise<void> | void;
  export let onRefreshMaintenance: () => Promise<void> | void;
  export let onRefreshBackups: () => Promise<void> | void;

  export let onMaintenanceDryRun: () => Promise<void> | void;
  export let onMaintenanceCleanup: () => Promise<void> | void;

  export let onSelectBackup: (path: string) => void;
  export let onCopyBackupPath: () => Promise<void> | void;
  export let onCopyBackupOriginalPath: () => Promise<void> | void;
  export let onRestoreBackup: () => Promise<void> | void;

  $: utilitySummary =
    activeTab === 'backups'
      ? `${backups.length} backup item(s)`
      : activeTab === 'maintenance'
        ? maintenance
          ? `${maintenance.orphanGeneratedIconsCount} orphaned auto icon(s)`
          : 'Maintenance overview'
        : diagnostics
          ? diagnosticsMissingCount === 0
            ? 'All key tools found'
            : `${diagnosticsMissingCount} tool(s) missing`
          : 'Runtime diagnostics';

  $: activeRefresh =
    activeTab === 'backups'
      ? onRefreshBackups
      : activeTab === 'maintenance'
        ? onRefreshMaintenance
        : onRefreshDiagnostics;
</script>

{#if open}
  <button
    type="button"
    class="utilityOverlay"
    aria-label="Close utility"
    on:click={onClose}
  ></button>

  <div
    class="panel utilityWindow"
    role="dialog"
    aria-modal="true"
    aria-label="Utility drawer"
    tabindex="-1"
  >
    <div class="utilityHeader">
      <div class="utilityTitleWrap">
        <div class="utilityEyebrow">On demand tools</div>
        <div class="utilityTitleRow">
          <div class="panelTitle">Utility</div>
          <span class="utilitySummary">{utilitySummary}</span>
        </div>
      </div>

      <div class="utilityHeaderActions">
        <button type="button" class="ghost" on:click={activeRefresh}>
          Refresh
        </button>
        <button type="button" class="ghost" on:click={onClose}>
          Close
        </button>
      </div>
    </div>

    <div class="utilityTabRow">
      <button
        type="button"
        class="ghost utilityTabButton"
        class:activeTab={activeTab === 'backups'}
        on:click={() => onOpenTab('backups')}
      >
        <span>Backups</span>
        <span class="utilityTabMeta">{backups.length}</span>
      </button>

      <button
        type="button"
        class="ghost utilityTabButton"
        class:activeTab={activeTab === 'maintenance'}
        on:click={() => onOpenTab('maintenance')}
      >
        <span>Maintenance</span>
        <span class="utilityTabMeta">
          {maintenance ? maintenance.orphanGeneratedIconsCount : 0}
        </span>
      </button>

      <button
        type="button"
        class="ghost utilityTabButton"
        class:activeTab={activeTab === 'diagnostics'}
        on:click={() => onOpenTab('diagnostics')}
      >
        <span>Diagnostics</span>
        <span class="utilityTabMeta">{diagnosticsMissingCount}</span>
      </button>
    </div>

    <div class="utilityInfoRow">
      <span class="utilityInfoChip">Ctrl+B Backups</span>
      <span class="utilityInfoChip">Ctrl+M Maintenance</span>
      <span class="utilityInfoChip">Ctrl+D Diagnostics</span>
      <span class="utilityInfoChip">Esc Close</span>
    </div>

    <div class="utilityBody">
      {#if activeTab === 'backups'}
        <BackupPanel
          embedded={true}
          backups={backups}
          backupsOpen={true}
          backupsBusy={backupsBusy}
          backupsRestoreBusy={backupsRestoreBusy}
          selectedBackupPath={selectedBackupPath}
          onToggle={onClose}
          onRefresh={onRefreshBackups}
          onSelect={onSelectBackup}
          onCopyPath={onCopyBackupPath}
          onCopyOriginalPath={onCopyBackupOriginalPath}
          onRestore={onRestoreBackup}
        />
      {:else if activeTab === 'maintenance'}
        <MaintenancePanel
          embedded={true}
          maintenance={maintenance}
          maintenanceOpen={true}
          maintenanceBusy={maintenanceBusy}
          lastCleanupResult={lastCleanupResult}
          onToggle={onClose}
          onRefresh={onRefreshMaintenance}
          onDryRun={onMaintenanceDryRun}
          onCleanup={onMaintenanceCleanup}
        />
      {:else}
        <DiagnosticsPanel
          embedded={true}
          diagnostics={diagnostics}
          diagnosticsOpen={true}
          diagnosticsBusy={diagnosticsBusy}
          missingCount={diagnosticsMissingCount}
          onToggle={onClose}
          onRefresh={onRefreshDiagnostics}
        />
      {/if}
    </div>
  </div>
{/if}

<style>
  .utilityOverlay {
    position: fixed;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
    padding: 0;
    border: 0;
    background: rgba(3, 8, 16, 0.42);
    backdrop-filter: blur(2px);
    appearance: none;
    z-index: 70;
  }

  .utilityWindow {
    position: fixed;
    top: 78px;
    left: 50%;
    transform: translateX(-50%);
    width: min(1320px, calc(100vw - 32px));
    max-height: calc(100vh - 110px);
    padding: 14px;
    border-radius: 16px;
    overflow: hidden;
    z-index: 80;
  }

  .utilityHeader {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }

  .utilityEyebrow {
    font-size: 0.72rem;
    opacity: 0.72;
    margin-bottom: 2px;
  }

  .utilityTitleRow {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .utilitySummary {
    font-size: 0.78rem;
    opacity: 0.82;
  }

  .utilityHeaderActions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .utilityTabRow {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    margin-bottom: 10px;
  }

  .utilityTabButton {
    min-height: 42px;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    text-align: left;
  }

  .utilityTabButton.activeTab {
    background: rgba(255, 255, 255, 0.08);
  }

  .utilityTabMeta {
    font-size: 0.76rem;
    opacity: 0.78;
  }

  .utilityInfoRow {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }

  .utilityInfoChip {
    font-size: 0.72rem;
    opacity: 0.76;
    padding: 4px 8px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.04);
  }

  .utilityBody {
    max-height: calc(100vh - 250px);
    overflow: auto;
    padding-right: 2px;
  }

  @media (max-width: 900px) {
    .utilityWindow {
      top: 70px;
      width: min(100vw - 20px, 1320px);
      max-height: calc(100vh - 84px);
    }

    .utilityHeader {
      flex-direction: column;
    }

    .utilityTabRow {
      grid-template-columns: 1fr;
    }

    .utilityBody {
      max-height: calc(100vh - 310px);
    }
  }
</style>
