<script lang="ts">
  import { onDestroy, tick } from 'svelte';
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

  $: activeWorkspaceTitle =
    activeTab === 'backups'
      ? 'Backup browser'
      : activeTab === 'maintenance'
        ? 'Maintenance'
        : 'Diagnostics';

  $: activeWorkspaceText =
    activeTab === 'backups'
      ? 'Inspect and restore created snapshots.'
      : activeTab === 'maintenance'
        ? 'Review generated assets and cleanup candidates.'
        : 'Check the runtime environment and required tools.';

  $: activeBusy =
    activeTab === 'backups'
      ? backupsBusy || backupsRestoreBusy
      : activeTab === 'maintenance'
        ? maintenanceBusy
        : diagnosticsBusy;
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
          <div class="utilityEyebrow">On demand tools</div>

          <div class="utilityTitleRow">
            <div id="utility-title" class="panelTitle">Utility</div>
            <span id="utility-summary" class="utilitySummary">{utilitySummary}</span>
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
            <span class="utilityTabMeta">
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
            <span class="utilityTabMeta">{diagnosticsMissingCount}</span>
          </button>
        </div>

        <div class="utilityWorkspaceMeta">
          <div class="utilityWorkspaceCard">
            <div class="utilityWorkspaceLabel">{activeWorkspaceTitle}</div>
            <div class="utilityWorkspaceText">{activeWorkspaceText}</div>
          </div>

          <div class="utilityStatusPill" class:isBusy={activeBusy}>
            {activeBusy ? 'Busy' : 'Ready'}
          </div>
        </div>
      </div>

      <div class="utilityInfoRow">
        <span class="utilityInfoChip">Ctrl+B</span>
        <span class="utilityInfoChip">Ctrl+M</span>
        <span class="utilityInfoChip">Ctrl+D</span>
        <span class="utilityInfoChip">Esc close</span>
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
  </div>
{/if}

<style>
  .utilityOverlay {
    position: fixed;
    inset: 0;
    background: rgba(3, 8, 16, 0.52);
    backdrop-filter: blur(6px);
    z-index: 70;
    border: 0;
    outline: none;
  }

  .utilityWindow {
    --utility-gap: 10px;
    --utility-card-radius: 12px;
    --utility-card-padding: 10px 12px;
    --utility-card-border: 1px solid rgba(255, 255, 255, 0.08);
    --utility-card-bg: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.035),
      rgba(255, 255, 255, 0.02)
    );
    --utility-card-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.02);
    --utility-soft-text: rgba(255, 255, 255, 0.74);
    --utility-strong-text: rgba(255, 255, 255, 0.96);

    position: fixed;
    top: 78px;
    left: 50%;
    transform: translateX(-50%);
    width: min(1320px, calc(100vw - 32px));
    max-height: calc(100vh - 110px);
    padding: 0;
    border-radius: 18px;
    overflow: hidden;
    z-index: 80;
    display: flex;
    flex-direction: column;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background:
      linear-gradient(180deg, rgba(18, 22, 29, 0.98), rgba(13, 16, 22, 0.98)),
      rgba(12, 15, 21, 0.98);
    box-shadow:
      0 24px 80px rgba(0, 0, 0, 0.42),
      inset 0 1px 0 rgba(255, 255, 255, 0.04);
    backdrop-filter: blur(16px);
    outline: none;
  }

  .utilityTopShell {
    position: sticky;
    top: 0;
    z-index: 4;
    padding: 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.03), rgba(255, 255, 255, 0)),
      rgba(15, 19, 25, 0.92);
    backdrop-filter: blur(12px);
  }

  .utilityHeader {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }

  .utilityTitleWrap {
    min-width: 0;
  }

  .utilityEyebrow {
    font-size: 0.72rem;
    opacity: 0.68;
    margin-bottom: 3px;
    letter-spacing: 0.02em;
  }

  .utilityTitleRow {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .utilitySummary {
    font-size: 0.78rem;
    color: var(--utility-soft-text);
  }

  .utilityHeaderActions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    flex-shrink: 0;
  }

  .utilityToolbarRow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 10px;
    align-items: start;
  }

  .utilityTabRow {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 6px;
    padding: 4px;
    border-radius: 13px;
    background: rgba(255, 255, 255, 0.025);
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .utilityTabButton {
    min-height: 46px;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    text-align: left;
    border-radius: 10px;
    border: 1px solid transparent;
    transition:
      background 0.14s ease,
      border-color 0.14s ease;
  }

  .utilityTabButton:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .utilityTabButton.activeTab {
    background: rgba(255, 255, 255, 0.07);
    border-color: rgba(255, 255, 255, 0.08);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.025);
  }

  .utilityTabLabel {
    font-weight: 500;
  }

  .utilityTabMeta {
    min-width: 30px;
    padding: 3px 8px;
    border-radius: 999px;
    text-align: center;
    font-size: 0.75rem;
    line-height: 1.2;
    background: rgba(255, 255, 255, 0.05);
    color: var(--utility-soft-text);
  }

  .utilityWorkspaceMeta {
    display: flex;
    align-items: stretch;
    gap: 8px;
    min-width: 290px;
  }

  .utilityWorkspaceCard {
    min-width: 0;
    padding: 9px 11px;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    background: rgba(255, 255, 255, 0.025);
  }

  .utilityWorkspaceLabel {
    font-size: 0.75rem;
    color: var(--utility-soft-text);
    margin-bottom: 3px;
  }

  .utilityWorkspaceText {
    font-size: 0.81rem;
    line-height: 1.35;
    color: var(--utility-strong-text);
  }

  .utilityStatusPill {
    align-self: stretch;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 78px;
    padding: 0 12px;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    background: rgba(255, 255, 255, 0.03);
    color: var(--utility-soft-text);
    font-size: 0.76rem;
    white-space: nowrap;
  }

  .utilityStatusPill.isBusy {
    background: rgba(255, 255, 255, 0.06);
    color: var(--utility-strong-text);
  }

  .utilityInfoRow {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 10px;
  }

  .utilityInfoChip {
    font-size: 0.72rem;
    color: var(--utility-soft-text);
    padding: 4px 9px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.04);
  }

  .utilityBody {
    flex: 1;
    min-height: 0;
    overflow: auto;
    overscroll-behavior: contain;
    padding: 14px;
    background: rgba(255, 255, 255, 0.01);
  }

  .utilityCanvas {
    min-height: min(640px, calc(100vh - 320px));
    border-radius: 15px;
    border: 1px solid rgba(255, 255, 255, 0.055);
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.018), rgba(255, 255, 255, 0.008)),
      rgba(0, 0, 0, 0.12);
    padding: 12px;
  }

  @media (max-width: 1100px) {
    .utilityToolbarRow {
      grid-template-columns: 1fr;
    }

    .utilityWorkspaceMeta {
      min-width: 0;
    }
  }

  @media (max-width: 900px) {
    .utilityWindow {
      top: 70px;
      width: min(100vw - 20px, 1320px);
      max-height: calc(100vh - 84px);
      border-radius: 16px;
    }

    .utilityTopShell,
    .utilityBody {
      padding: 12px;
    }

    .utilityHeader {
      flex-direction: column;
    }

    .utilityHeaderActions {
      width: 100%;
    }

    .utilityTabRow {
      grid-template-columns: 1fr;
    }

    .utilityWorkspaceMeta {
      flex-direction: column;
    }

    .utilityStatusPill {
      min-height: 38px;
      justify-content: flex-start;
    }

    .utilityCanvas {
      min-height: auto;
      padding: 10px;
    }
  }
</style>
