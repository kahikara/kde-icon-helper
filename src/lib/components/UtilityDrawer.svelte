<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import BackupPanel from '$lib/components/BackupPanel.svelte';
  import DiagnosticsPanel from '$lib/components/DiagnosticsPanel.svelte';
  import MaintenancePanel from '$lib/components/MaintenancePanel.svelte';
  import type {
    BackupEntry,
    CleanupResult,
    GeneratedAssetStats,
    LauncherEntry,
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
  export let bulkFixCandidateCount = 0;
  export let bulkFixBusy = false;
  export let bulkFixPreviewEntries: LauncherEntry[] = [];

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
  export let onBulkFixVisible: () => Promise<void> | void;

  export let onSelectBackup: (path: string) => void;
  export let onCopyBackupPath: () => Promise<void> | void;
  export let onCopyBackupOriginalPath: () => Promise<void> | void;
  export let onRestoreBackup: () => Promise<void> | void;
  export let onResetUi: () => Promise<void> | void;

  let utilityWindowEl: HTMLDivElement | null = null;
  let lastOpen = false;
  let previousBodyOverflow = '';

  async function syncOpenState(isOpen: boolean) {
    if (typeof document === 'undefined') return;

    if (isOpen) {
      previousBodyOverflow = document.body.style.overflow;
      document.body.style.overflow = 'hidden';
      await tick();
      utilityWindowEl?.focus();
      return;
    }

    document.body.style.overflow = previousBodyOverflow;
  }

  function handleOverlayKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onClose();
    }
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      onClose();
    }
  }

  onDestroy(() => {
    if (typeof document !== 'undefined') {
      document.body.style.overflow = previousBodyOverflow;
    }
  });

  $: if (open !== lastOpen) {
    lastOpen = open;
    void syncOpenState(open);
  }

  $: utilitySummary =
    activeTab === 'backups'
      ? `${backups.length} total`
      : activeTab === 'maintenance'
        ? maintenance
          ? `${maintenance.orphanGeneratedIconsCount} orphaned`
          : 'No data'
        : diagnostics
          ? diagnosticsMissingCount === 0
            ? 'All key tools found'
            : `${diagnosticsMissingCount} missing`
          : 'No data';

  $: activeRefresh =
    activeTab === 'backups'
      ? onRefreshBackups
      : activeTab === 'maintenance'
        ? onRefreshMaintenance
        : onRefreshDiagnostics;

  $: activeWorkspaceTitle =
    activeTab === 'backups'
      ? 'Backups'
      : activeTab === 'maintenance'
        ? 'Maintenance'
        : 'Diagnostics';

  $: activeBusy =
    activeTab === 'backups'
      ? backupsBusy || backupsRestoreBusy
      : activeTab === 'maintenance'
        ? maintenanceBusy || bulkFixBusy
        : diagnosticsBusy;

  $: activeHasIssues =
    activeTab === 'backups'
      ? false
      : activeTab === 'maintenance'
        ? (maintenance?.orphanGeneratedIconsCount ?? 0) > 0
        : diagnostics
          ? diagnosticsMissingCount > 0 || !diagnostics.desktopDirExists
          : diagnosticsMissingCount > 0;

  $: activeStateText = activeBusy ? 'Busy' : activeHasIssues ? 'Review' : 'Ready';
</script>

{#if open}
  <div
    class="utilityOverlay"
    role="button"
    tabindex="0"
    aria-label="Close utility"
    on:click={onClose}
    on:keydown={handleOverlayKeydown}
  ></div>

  <div
    class="panel utilityWindow"
    bind:this={utilityWindowEl}
    role="dialog"
    aria-modal="true"
    aria-labelledby="utility-title"
    aria-describedby="utility-summary"
    tabindex="-1"
    on:keydown={handleWindowKeydown}
  >
    <div class="utilityTopShell">
      <div class="utilityHeader">
        <div class="utilityTitleWrap">
          <div id="utility-title" class="panelTitle">Utility</div>
          <div id="utility-summary" class="utilitySummary">
            {activeWorkspaceTitle} · {utilitySummary}
          </div>
        </div>

        <div class="utilityHeaderActions">
          <button type="button" class="ghost shellButton" on:click={activeRefresh}>
            {activeBusy ? 'Working…' : 'Refresh'}
          </button>
          <button type="button" class="ghost shellButton" on:click={onResetUi}>
            Reset UI
          </button>
          <button type="button" class="ghost shellButton" on:click={onClose}>
            Close
          </button>
        </div>
      </div>

      <div class="utilityToolbarRow">
        <div class="utilityTabRow">
          <button
            type="button"
            class="ghost utilityTabButton"
            class:activeTab={activeTab === 'backups'}
            aria-pressed={activeTab === 'backups'}
            on:click={() => onOpenTab('backups')}
          >
            <span class="utilityTabLabel">Backups</span>
            <span class="utilityTabMeta">{backups.length}</span>
          </button>

          <button
            type="button"
            class="ghost utilityTabButton"
            class:activeTab={activeTab === 'maintenance'}
            aria-pressed={activeTab === 'maintenance'}
            on:click={() => onOpenTab('maintenance')}
          >
            <span class="utilityTabLabel">Maintenance</span>
            <span
              class="utilityTabMeta"
              class:utilityTabMetaAlert={(maintenance?.orphanGeneratedIconsCount ?? 0) > 0}
            >
              {maintenance ? maintenance.orphanGeneratedIconsCount : 0}
            </span>
          </button>

          <button
            type="button"
            class="ghost utilityTabButton"
            class:activeTab={activeTab === 'diagnostics'}
            aria-pressed={activeTab === 'diagnostics'}
            on:click={() => onOpenTab('diagnostics')}
          >
            <span class="utilityTabLabel">Diagnostics</span>
            <span
              class="utilityTabMeta"
              class:utilityTabMetaAlert={diagnosticsMissingCount > 0}
            >
              {diagnosticsMissingCount}
            </span>
          </button>
        </div>

        <div class="utilityStatusPill" class:isBusy={activeBusy} class:stateAlert={activeHasIssues && !activeBusy}>
          {activeStateText}
        </div>
      </div>
    </div>

    <div class="utilityBody">
      <div class="utilityCanvas">
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
            bulkFixCandidateCount={bulkFixCandidateCount}
            bulkFixBusy={bulkFixBusy}
            bulkFixPreviewEntries={bulkFixPreviewEntries}
            onToggle={onClose}
            onRefresh={onRefreshMaintenance}
            onDryRun={onMaintenanceDryRun}
            onCleanup={onMaintenanceCleanup}
            onBulkFixVisible={onBulkFixVisible}
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
  </div>
{/if}

<style>
  .utilityOverlay {
    position: fixed;
    inset: 0;
    background: rgba(3, 8, 16, 0.44);
    backdrop-filter: blur(4px);
    z-index: 70;
    border: 0;
    outline: none;
  }

  .utilityWindow {
    --utility-gap: 8px;
    --utility-card-radius: 11px;
    --utility-card-padding: 9px 10px;
    --utility-card-border: 1px solid rgba(255, 255, 255, 0.07);
    --utility-card-bg: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.026),
      rgba(255, 255, 255, 0.016)
    );
    --utility-card-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.016);
    --utility-soft-text: rgba(255, 255, 255, 0.72);
    --utility-strong-text: rgba(255, 255, 255, 0.96);

    position: fixed;
    top: 76px;
    left: 50%;
    transform: translateX(-50%);
    width: min(1180px, calc(100vw - 34px));
    max-height: calc(100vh - 104px);
    padding: 0;
    border-radius: 16px;
    overflow: hidden;
    z-index: 80;
    display: flex;
    flex-direction: column;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background:
      linear-gradient(180deg, rgba(18, 22, 29, 0.98), rgba(13, 16, 22, 0.98)),
      rgba(12, 15, 21, 0.98);
    box-shadow:
      0 22px 72px rgba(0, 0, 0, 0.38),
      inset 0 1px 0 rgba(255, 255, 255, 0.035);
    backdrop-filter: blur(12px);
    outline: none;
  }

  .utilityTopShell {
    position: sticky;
    top: 0;
    z-index: 4;
    padding: 10px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.02), rgba(255, 255, 255, 0)),
      rgba(15, 19, 25, 0.92);
    backdrop-filter: blur(10px);
  }

  .utilityHeader {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 8px;
  }

  .utilityTitleWrap {
    min-width: 0;
  }

  .utilitySummary {
    margin-top: 3px;
    font-size: 0.76rem;
    color: var(--utility-soft-text);
  }

  .utilityHeaderActions {
    display: flex;
    gap: 7px;
    flex-wrap: wrap;
    flex-shrink: 0;
  }

  .shellButton {
    min-width: 86px;
    min-height: 32px;
    background: rgba(255, 255, 255, 0.02);
    border-color: rgba(255, 255, 255, 0.06);
  }

  .shellButton:hover:enabled {
    background: rgba(255, 255, 255, 0.045);
  }

  .utilityToolbarRow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
    align-items: center;
  }

  .utilityTabRow {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 5px;
    padding: 4px;
    border-radius: 11px;
    background: rgba(255, 255, 255, 0.022);
    border: 1px solid rgba(255, 255, 255, 0.045);
  }

  .utilityTabButton {
    min-height: 40px;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    text-align: left;
    border-radius: 9px;
    border: 1px solid transparent;
  }

  .utilityTabButton:hover {
    background: rgba(255, 255, 255, 0.028);
  }

  .utilityTabButton.activeTab {
    background: rgba(255, 255, 255, 0.06);
    border-color: rgba(255, 255, 255, 0.07);
  }

  .utilityTabLabel {
    font-weight: 600;
    line-height: 1.2;
  }

  .utilityTabMeta {
    min-width: 26px;
    padding: 2px 7px;
    border-radius: 999px;
    text-align: center;
    font-size: 0.72rem;
    line-height: 1.2;
    background: rgba(255, 255, 255, 0.045);
    color: var(--utility-soft-text);
  }

  .utilityTabMetaAlert {
    background: rgba(255, 255, 255, 0.08);
    color: var(--utility-strong-text);
  }

  .utilityStatusPill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 88px;
    min-height: 36px;
    padding: 0 11px;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    background: rgba(255, 255, 255, 0.025);
    color: var(--utility-soft-text);
    font-size: 0.75rem;
    white-space: nowrap;
  }

  .utilityStatusPill.isBusy {
    background: rgba(255, 255, 255, 0.055);
    color: var(--utility-strong-text);
  }

  .utilityStatusPill.stateAlert {
    background: rgba(255, 255, 255, 0.07);
    color: var(--utility-strong-text);
  }

  .utilityBody {
    flex: 1;
    min-height: 0;
    overflow: auto;
    overscroll-behavior: contain;
    padding: 10px;
    background: rgba(255, 255, 255, 0.008);
  }

  .utilityCanvas {
    min-height: min(560px, calc(100vh - 252px));
    border-radius: 13px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.013), rgba(255, 255, 255, 0.007)),
      rgba(0, 0, 0, 0.1);
    padding: 9px;
  }

  @media (max-width: 980px) {
    .utilityWindow {
      top: 70px;
      width: min(100vw - 20px, 1180px);
      max-height: calc(100vh - 84px);
      border-radius: 14px;
    }

    .utilityHeader {
      flex-direction: column;
      align-items: stretch;
    }

    .utilityHeaderActions {
      width: 100%;
    }

    .utilityToolbarRow {
      grid-template-columns: 1fr;
    }

    .utilityTabRow {
      grid-template-columns: 1fr;
    }

    .utilityStatusPill {
      justify-content: flex-start;
    }
  }
</style>
