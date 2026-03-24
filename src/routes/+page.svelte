<script lang="ts">
  import appIcon from '$lib/assets/kde-icon-helper.svg';
  import LauncherList from '$lib/components/LauncherList.svelte';
  import InspectorPanel from '$lib/components/InspectorPanel.svelte';
  import ContextMenu from '$lib/components/ContextMenu.svelte';
  import UtilityDrawer from '$lib/components/UtilityDrawer.svelte';
  import {
    entryActionItems,
    inputActionItems,
    kindFilterOptions,
    previewFallbackGlyph,
    rowGlyph,
    statusClass,
    statusFilterOptions,
    statusText
  } from '$lib/launcher-ui';
  import { createLauncherController } from '$lib/launcher-controller';
  import { onMount } from 'svelte';

  const controller = createLauncherController();
  let searchInputEl: HTMLInputElement | null = null;

  $: controller.bindSearchInput(searchInputEl);

  onMount(() => {
    return controller.mount();
  });
</script>

<svelte:head>
  <title>KDE Icon Helper</title>

  <style>
    .workspace {
      grid-template-columns: minmax(300px, 0.92fr) minmax(0, 1.28fr) !important;
      grid-template-rows: minmax(0, 1fr) auto !important;
      gap: 8px !important;
    }

    .listPanel {
      grid-column: 1 !important;
      grid-row: 1 / span 2 !important;
    }

    .inspectorPanel {
      grid-column: 2 !important;
      grid-row: 1 !important;
    }

    .listScroll {
      padding: 8px !important;
      gap: 8px !important;
    }

    .itemCard {
      min-height: 42px !important;
      padding: 0 10px !important;
      grid-template-columns: 20px minmax(0, 1fr) max-content !important;
      gap: 10px !important;
    }

    .itemCard:focus,
    .itemCard:focus-visible {
      outline: none;
    }

    .itemIcon {
      width: 20px !important;
      height: 20px !important;
    }

    .itemName {
      font-size: 0.84rem !important;
    }

    .badge {
      font-size: 0.58rem !important;
      padding: 3px 7px !important;
    }

    .utilityBadge {
      font-size: 0.78rem;
      opacity: 0.82;
    }

    @media (max-width: 980px) {
      .workspace {
        grid-template-columns: 1fr !important;
        grid-template-rows: minmax(220px, auto) minmax(0, 1fr) auto !important;
      }

      .listPanel,
      .inspectorPanel {
        grid-column: 1 !important;
        grid-row: auto !important;
      }
    }
  </style>
</svelte:head>

<div class="app">
  <header class="topbar">
    <div class="brand">
      <img class="brandIcon" src={appIcon} alt="KDE Icon Helper" />
      <div class="brandText">
        <div class="brandTitle">KDE Icon Helper</div>
        <div class="brandSubline">Launcher icon inspection and repair</div>
      </div>
    </div>

    <div class="toolbar">
      <div class="toolbarMain">
        <div class="searchWrap">
          <input
            type="text"
            placeholder="Search"
            bind:this={searchInputEl}
            value={$controller.query}
            on:input={(event) =>
              controller.setQuery((event.currentTarget as HTMLInputElement).value)}
          />
        </div>

        <div class="toolbarFilters">
          <div class="selectWrap">
            <select
              value={$controller.statusFilter}
              on:change={(event) =>
                controller.setStatusFilter((event.currentTarget as HTMLSelectElement).value as any)}
            >
              {#each statusFilterOptions as option}
                <option value={option.value}>{option.label}</option>
              {/each}
            </select>
          </div>

          <div class="selectWrap">
            <select
              value={$controller.kindFilter}
              on:change={(event) =>
                controller.setKindFilter((event.currentTarget as HTMLSelectElement).value as any)}
            >
              {#each kindFilterOptions as option}
                <option value={option.value}>{option.label}</option>
              {/each}
            </select>
          </div>
        </div>
      </div>

      <div class="toolbarActions">
        <div class="toolbarStats">
          <div
            class="pill"
            title="/ focus search · Ctrl+R scan · Ctrl+L log · Ctrl+B backups · Ctrl+D diagnostics · Ctrl+M maintenance · Ctrl+Shift+R reset"
          >
            {$controller.shownCount} items
          </div>

          <div class="pill quietPill">
            {$controller.busy ? 'Busy' : 'Ready'}
          </div>
        </div>

        <button type="button" class="ghost utilityButton" on:click={() => controller.toggleUtilityOpen()}>
          Utility
          {#if $controller.diagnosticsMissingCount > 0 || ($controller.maintenance?.orphanGeneratedIconsCount ?? 0) > 0}
            <span class="utilityBadge">•</span>
          {/if}
        </button>

        <button type="button" class="ghost" on:click={() => controller.resetUiPreferences()}>
          Reset UI
        </button>

        <button
          class="primary"
          type="button"
          on:click={() => controller.scan()}
          disabled={$controller.busy}
        >
          {$controller.busy ? 'Working…' : 'Scan'}
        </button>
      </div>
    </div>
  </header>

  <UtilityDrawer
    open={$controller.utilityOpen}
    activeTab={$controller.utilityTab}
    diagnostics={$controller.diagnostics}
    diagnosticsBusy={$controller.diagnosticsBusy}
    diagnosticsMissingCount={$controller.diagnosticsMissingCount}
    maintenance={$controller.maintenance}
    maintenanceBusy={$controller.maintenanceBusy}
    lastCleanupResult={$controller.lastCleanupResult}
    backups={$controller.backups}
    backupsBusy={$controller.backupsBusy}
    backupsRestoreBusy={$controller.backupsRestoreBusy}
    selectedBackupPath={$controller.selectedBackupPath}
    onOpenTab={controller.openUtilityTab}
    onClose={controller.closeUtility}
    onRefreshDiagnostics={controller.refreshDiagnostics}
    onRefreshMaintenance={controller.refreshMaintenance}
    onRefreshBackups={controller.refreshBackups}
    onMaintenanceDryRun={() => controller.runGeneratedCleanup(true)}
    onMaintenanceCleanup={() => controller.runGeneratedCleanup(false)}
    onSelectBackup={controller.selectBackup}
    onCopyBackupPath={controller.copySelectedBackupPath}
    onCopyBackupOriginalPath={controller.copySelectedBackupOriginalPath}
    onRestoreBackup={controller.restoreBackupFromSelection}
  />

  <main class="workspace">
    <LauncherList
      filteredEntries={$controller.filteredEntries}
      selected={$controller.selected}
      listIconUrl={controller.listIconUrl}
      {rowGlyph}
      {statusClass}
      {statusText}
      onSelect={controller.selectEntry}
      onContextMenu={controller.openItemContextMenu}
    />

    <InspectorPanel
      selected={$controller.selected}
      busy={$controller.busy}
      selectedIconUrl={$controller.selectedIconUrl}
      iconLoadFailed={$controller.iconLoadFailed}
      selectedHasThemeIcon={$controller.selectedHasThemeIcon}
      selectedExecName={$controller.selectedExecName}
      {entryActionItems}
      {statusClass}
      {statusText}
      {previewFallbackGlyph}
      canRunEntryAction={controller.canRunEntryAction}
      runEntryAction={controller.runEntryAction}
      onPreviewError={() => controller.setIconLoadFailed(true)}
    />
  </main>

  <ContextMenu
    open={$controller.contextMenuOpen}
    mode={$controller.contextMenuMode}
    entry={$controller.contextMenuEntry}
    x={$controller.contextMenuX}
    y={$controller.contextMenuY}
    {inputActionItems}
    availableEntryActions={controller.availableEntryActions}
    onInputAction={controller.runInputContextAction}
    onEntryAction={controller.runContextAction}
    onEscape={controller.handleContextMenuEscape}
  />

  <section class="panel logPanel">
    <div class="panelHeader logHeader">
      <div class="panelTitleWrap">
        <div class="panelTitle">Log</div>
        <div class="panelSubline">Recent scans, repairs and restore actions</div>
      </div>

      <div class="logHeaderMeta">
        <div class="panelMetaChip">{$controller.log.length}</div>
        <button type="button" class="ghost" on:click={() => controller.toggleLogOpen()}>
          {$controller.logOpen ? 'Hide' : 'Show'}
        </button>
      </div>
    </div>

    {#if $controller.logOpen}
      <div class="logScroll">
        <div class="logBodyCard">
          {#if $controller.log.length === 0}
            <div class="empty compact">
              <strong>No activity yet</strong>
              <span>Logs will appear here after scans and repairs.</span>
            </div>
          {:else}
            {#each $controller.log as line}
              <div class="logLine">{line}</div>
            {/each}
          {/if}
        </div>
      </div>
    {/if}
  </section>
</div>
