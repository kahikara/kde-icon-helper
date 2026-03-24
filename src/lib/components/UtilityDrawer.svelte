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
  <section class="panel utilityShell">
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
        class:activeTab={activeTab === 'backups'}
        class="ghost utilityTabButton"
        on:click={() => onOpenTab('backups')}
      >
        <span>Backups</span>
        <span class="utilityTabMeta">{backups.length}</span>
      </button>

      <button
        type="button"
        class:activeTab={activeTab === 'maintenance'}
        class="ghost utilityTabButton"
        on:click={() => onOpenTab('maintenance')}
      >
        <span>Maintenance</span>
        <span class="utilityTabMeta">
          {maintenance ? maintenance.orphanGeneratedIconsCount : 0}
        </span>
      </button>

      <button
        type="button"
        class:activeTab={activeTab === 'diagnostics'}
        class="ghost utilityTabButton"
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
  </section>
{/if}

<style>
  .utilityShell {
    margin-top: 10px;
    padding: 14px;
    border-radius: 16px;
  }

  .utilityHeader {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
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
    min-height: 140px;
  }

  @media (max-width: 780px) {
    .utilityTabRow {
      grid-template-columns: 1fr;
    }
  }
</style>
