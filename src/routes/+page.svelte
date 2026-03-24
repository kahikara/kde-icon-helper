<script lang="ts">
  import appIcon from '$lib/assets/kde-icon-helper.svg';
  import LauncherList from '$lib/components/LauncherList.svelte';
  import InspectorPanel from '$lib/components/InspectorPanel.svelte';
  import ContextMenu from '$lib/components/ContextMenu.svelte';
  import DiagnosticsPanel from '$lib/components/DiagnosticsPanel.svelte';
  import MaintenancePanel from '$lib/components/MaintenancePanel.svelte';
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
      min-height: 44px !important;
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

    .toolbarRight {
      display: flex;
      align-items: center;
      gap: 8px;
      flex-wrap: wrap;
    }

    .diagToolbarBadge {
      font-size: 0.72rem;
      opacity: 0.85;
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
    <div class="brand"><img class="brandIcon" src={appIcon} alt="KDE Icon Helper" /></div>

    <div class="toolbar">
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

      <div class="pill" title="/ focus search · Ctrl+R scan · Ctrl+L log · Ctrl+D diagnostics · Ctrl+M maintenance · Ctrl+Shift+R reset">
        {$controller.shownCount} items
      </div>

      <div class="toolbarRight">
        <button type="button" class="ghost" on:click={() => controller.toggleDiagnosticsOpen()}>
          Diagnostics
          {#if $controller.diagnostics}
            <span class="diagToolbarBadge">
              {$controller.diagnosticsMissingCount === 0
                ? 'OK'
                : `${$controller.diagnosticsMissingCount} missing`}
            </span>
          {/if}
        </button>

        <button type="button" class="ghost" on:click={() => controller.toggleMaintenanceOpen()}>
          Maintenance
          {#if $controller.maintenance}
            <span class="diagToolbarBadge">
              {$controller.maintenance.orphanGeneratedIconsCount} orphan
            </span>
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

  <DiagnosticsPanel
    diagnostics={$controller.diagnostics}
    diagnosticsOpen={$controller.diagnosticsOpen}
    diagnosticsBusy={$controller.diagnosticsBusy}
    missingCount={$controller.diagnosticsMissingCount}
    onToggle={() => controller.toggleDiagnosticsOpen()}
    onRefresh={() => controller.refreshDiagnostics()}
  />

  <MaintenancePanel
    maintenance={$controller.maintenance}
    maintenanceOpen={$controller.maintenanceOpen}
    maintenanceBusy={$controller.maintenanceBusy}
    lastCleanupResult={$controller.lastCleanupResult}
    onToggle={() => controller.toggleMaintenanceOpen()}
    onRefresh={() => controller.refreshMaintenance()}
    onDryRun={() => controller.runGeneratedCleanup(true)}
    onCleanup={() => controller.runGeneratedCleanup(false)}
  />

  <section class="panel logPanel">
    <div class="panelHeader logHeader">
      <div class="panelTitle">Log</div>
      <button type="button" class="ghost" on:click={() => controller.toggleLogOpen()}>
        {$controller.logOpen ? 'Hide' : 'Show'}
      </button>
    </div>

    {#if $controller.logOpen}
      <div class="logScroll">
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
    {/if}
  </section>
</div>
