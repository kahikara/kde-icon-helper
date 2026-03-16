<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import appIcon from '$lib/assets/kde-icon-helper.svg';
  import type { FixResult, LauncherEntry, LauncherStatus } from '$lib/types';

  type StatusFilter = 'all' | LauncherStatus;
  type KindFilter = 'all' | 'launcher' | 'exe_link';

  const statusLabel: Record<string, string> = {
    ok: 'Healthy',
    missing_icon: 'Missing icon',
    broken_icon_path: 'Broken icon path',
    exe_detected_needs_fixed_icon: 'Needs icon fix',
    missing_exec_target: 'Missing EXE target',
    invalid_desktop_file: 'Invalid desktop file',
    unsupported_exec: 'Unsupported item',
    direct_exe_link: 'Direct EXE link',
    all: 'All'
  };

  const statusTone: Record<string, string> = {
    ok: 'good',
    missing_icon: 'warn',
    broken_icon_path: 'danger',
    exe_detected_needs_fixed_icon: 'accent',
    missing_exec_target: 'danger',
    invalid_desktop_file: 'muted',
    unsupported_exec: 'muted',
    direct_exe_link: 'accent'
  };

  let entries: LauncherEntry[] = [];
  let selected: LauncherEntry | null = null;
  let busy = false;
  let query = '';
  let statusFilter: StatusFilter = 'all';
  let kindFilter: KindFilter = 'all';
  let log: string[] = [];
  let logOpen = true;
  let contextMenuOpen = false;
  let contextMenuX = 0;
  let contextMenuY = 0;
  let contextMenuEntry: LauncherEntry | null = null;

  let iconLoadFailed = false;
  let selectedPreviewUrl: string | null = null;
  let selectedPreviewFor = '';
  let lastSelectedPath = '';
  let itemIconUrls: Record<string, string> = {};
  let itemIconLoading: Record<string, boolean> = {};

  function pushLog(message: string) {
    log = [`${new Date().toLocaleTimeString()} ${message}`, ...log].slice(0, 250);
  }

  function statusText(status?: string | null) {
    return status ? statusLabel[status] ?? status : 'Unknown';
  }

  function statusClass(status?: string | null) {
    const tone = status ? statusTone[status] ?? 'muted' : 'muted';
    return `badge ${tone}`;
  }

  function kindOf(entry: LauncherEntry | null): 'launcher' | 'exe_link' | 'other' {
    if (!entry) return 'other';
    if (entry.status === 'direct_exe_link') return 'exe_link';
    if (
      [
        'ok',
        'missing_icon',
        'broken_icon_path',
        'exe_detected_needs_fixed_icon',
        'missing_exec_target',
        'invalid_desktop_file'
      ].includes(entry.status)
    ) {
      return 'launcher';
    }
    return 'other';
  }

  function listIconUrl(entry: LauncherEntry): string | null {
    const value = itemIconUrls[entry.path];
    return value && value.length > 0 ? value : null;
  }

  async function ensureListIcon(entry: LauncherEntry) {
    const path = entry.resolvedIconPath ?? null;
    if (!path) return;
    if (itemIconUrls[entry.path] || itemIconLoading[entry.path]) return;

    itemIconLoading = { ...itemIconLoading, [entry.path]: true };

    try {
      const result = await invoke<string | null>('load_icon_preview', { path });
      if (result) {
        itemIconUrls = { ...itemIconUrls, [entry.path]: result };
      }
    } finally {
      const next = { ...itemIconLoading };
      delete next[entry.path];
      itemIconLoading = next;
    }
  }

  async function preloadListIcons(entriesToLoad: LauncherEntry[]) {
    const wanted = entriesToLoad
      .filter((entry) => !!entry.resolvedIconPath)
      .filter((entry) => !itemIconUrls[entry.path])
      .slice(0, 80);

    if (wanted.length === 0) return;

    await Promise.allSettled(wanted.map((entry) => ensureListIcon(entry)));
  }

  async function loadSelectedPreview() {
    const path = selected?.resolvedIconPath ?? null;

    if (!path) {
      selectedPreviewUrl = null;
      return;
    }

    const current = path;

    try {
      const result = await invoke<string | null>('load_icon_preview', { path: current });
      if ((selected?.resolvedIconPath ?? null) === current) {
        selectedPreviewUrl = result;
      }
    } catch {
      if ((selected?.resolvedIconPath ?? null) === current) {
        selectedPreviewUrl = null;
      }
    }
  }

  function restoreSelection(nextEntries: LauncherEntry[]) {
    if (nextEntries.length === 0) {
      selected = null;
      return;
    }

    if (selected) {
      const found = nextEntries.find((entry) => entry.path === selected?.path);
      selected = found ?? nextEntries[0];
      return;
    }

    selected = nextEntries[0];
  }

  async function focusSelectedIntoView() {
    const path = selected?.path;
    if (!path) return;

    await tick();

    const escaped =
      typeof CSS !== 'undefined' && typeof CSS.escape === 'function'
        ? CSS.escape(path)
        : path.replace(/"/g, '\\"');

    const el = document.querySelector<HTMLButtonElement>(`[data-item-path="${escaped}"]`);
    el?.scrollIntoView({ block: 'nearest' });
  }

  async function selectRelative(delta: number) {
    if (filteredEntries.length === 0) return;

    const currentIndex = selected
      ? filteredEntries.findIndex((entry) => entry.path === selected?.path)
      : -1;

    const nextIndex =
      currentIndex === -1
        ? 0
        : Math.max(0, Math.min(filteredEntries.length - 1, currentIndex + delta));

    selected = filteredEntries[nextIndex];
    await focusSelectedIntoView();
  }

  function shouldAllowContextMenuTarget(target: EventTarget | null): boolean {
    const el = target as HTMLElement | null;
    if (!el) return false;

    const directTag = el.tagName?.toLowerCase() ?? '';
    if (directTag === 'input' || directTag === 'textarea') return true;
    if (el.isContentEditable) return true;

    const editableParent = el.closest('input, textarea, [contenteditable="true"], [contenteditable=""], [contenteditable]');
    return !!editableParent;
  }

  function shouldIgnoreKeyTarget(target: EventTarget | null): boolean {
    const el = target as HTMLElement | null;
    if (!el) return false;
    const tag = el.tagName?.toLowerCase() ?? '';
    return tag === 'input' || tag === 'textarea' || tag === 'select' || el.isContentEditable;
  }

  function handleGlobalKeydown(event: KeyboardEvent) {
    if (shouldIgnoreKeyTarget(event.target)) return;
    if (filteredEntries.length === 0) return;

    const currentIndex = selected
      ? filteredEntries.findIndex((entry) => entry.path === selected?.path)
      : -1;

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      closeContextMenu();
      void selectRelative(1);
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      closeContextMenu();
      void selectRelative(-1);
      return;
    }

    if (event.key === 'Home') {
      event.preventDefault();
      closeContextMenu();
      selected = filteredEntries[0];
      void focusSelectedIntoView();
      return;
    }

    if (event.key === 'End') {
      event.preventDefault();
      closeContextMenu();
      selected = filteredEntries[filteredEntries.length - 1];
      void focusSelectedIntoView();
      return;
    }

    if (event.key === 'PageDown') {
      event.preventDefault();
      closeContextMenu();
      const nextIndex = currentIndex === -1 ? 0 : Math.min(filteredEntries.length - 1, currentIndex + 8);
      selected = filteredEntries[nextIndex];
      void focusSelectedIntoView();
      return;
    }

    if (event.key === 'PageUp') {
      event.preventDefault();
      closeContextMenu();
      const nextIndex = currentIndex === -1 ? 0 : Math.max(0, currentIndex - 8);
      selected = filteredEntries[nextIndex];
      void focusSelectedIntoView();
      return;
    }

    if (event.key === 'Escape') {
      closeContextMenu();
    }
  }

  async function scan() {
    busy = true;
    try {
      const result = await invoke<LauncherEntry[]>('scan_launchers');
      entries = result;
      restoreSelection(result);
      void preloadListIcons(result);
      pushLog(`Scan finished. ${result.length} desktop item(s) found.`);
    } catch (error) {
      pushLog(`Scan failed: ${String(error)}`);
    } finally {
      busy = false;
    }
  }

  async function checkSelected() {
    if (!selected) return;

    busy = true;
    try {
      const updated = await invoke<LauncherEntry>('check_launcher', { path: selected.path });
      entries = entries.map((entry) => (entry.path === updated.path ? updated : entry));
      selected = updated;
      void preloadListIcons(entries);
      pushLog(`Checked ${updated.name}. Status is now ${statusText(updated.status)}.`);
    } catch (error) {
      pushLog(`Check failed: ${String(error)}`);
    } finally {
      busy = false;
    }
  }

  async function fixSelected() {
    if (!selected) return;

    busy = true;
    try {
      const previousPath = selected.path;
      const result = await invoke<FixResult>('fix_launcher_icon', { path: previousPath });
      pushLog(result.message);

      const refreshed = await invoke<LauncherEntry[]>('scan_launchers');
      entries = refreshed;
      void preloadListIcons(refreshed);
      selected =
        refreshed.find((entry) => entry.path === result.updatedEntry?.path) ??
        refreshed.find((entry) => entry.path === previousPath) ??
        refreshed[0] ??
        null;
    } catch (error) {
      pushLog(`Fix failed: ${String(error)}`);
    } finally {
      busy = false;
    }
  }

  async function restoreDefaultIcon() {
    if (!selected) return;

    busy = true;
    try {
      const previousPath = selected.path;
      const result = await invoke<FixResult>('restore_launcher_icon_default', {
        path: previousPath
      });
      pushLog(result.message);

      const refreshed = await invoke<LauncherEntry[]>('scan_launchers');
      entries = refreshed;
      void preloadListIcons(refreshed);
      selected =
        refreshed.find((entry) => entry.path === result.updatedEntry?.path) ??
        refreshed.find((entry) => entry.path === previousPath) ??
        refreshed[0] ??
        null;
    } catch (error) {
      pushLog(`Restore default icon failed: ${String(error)}`);
    } finally {
      busy = false;
    }
  }

  async function setManualIcon() {
    if (!selected) return;

    const chosen = await open({
      multiple: false,
      directory: false,
      filters: [
        {
          name: 'Images',
          extensions: ['png', 'svg', 'xpm', 'ico']
        }
      ]
    });

    if (!chosen || Array.isArray(chosen)) return;

    busy = true;
    try {
      const previousPath = selected.path;
      const result = await invoke<FixResult>('set_launcher_icon_manual', {
        path: previousPath,
        sourceIconPath: chosen
      });
      pushLog(result.message);

      const refreshed = await invoke<LauncherEntry[]>('scan_launchers');
      entries = refreshed;
      void preloadListIcons(refreshed);
      selected =
        refreshed.find((entry) => entry.path === result.updatedEntry?.path) ??
        refreshed.find((entry) => entry.path === previousPath) ??
        refreshed[0] ??
        null;
    } catch (error) {
      pushLog(`Manual icon failed: ${String(error)}`);
    } finally {
      busy = false;
    }
  }

  function closeContextMenu() {
    contextMenuOpen = false;
    contextMenuEntry = null;
  }

  function openItemContextMenu(event: MouseEvent, entry: LauncherEntry) {
    event.preventDefault();
    event.stopPropagation();

    selected = entry;
    contextMenuEntry = entry;
    contextMenuX = event.clientX;
    contextMenuY = event.clientY;
    contextMenuOpen = true;
  }

  async function runContextAction(action: 'check' | 'fix' | 'manual' | 'restore') {
    const entry = contextMenuEntry;
    closeContextMenu();
    if (!entry) return;

    selected = entry;
    await tick();

    if (action === 'check') {
      await checkSelected();
    } else if (action === 'fix') {
      await fixSelected();
    } else if (action === 'manual') {
      await setManualIcon();
    } else if (action === 'restore') {
      await restoreDefaultIcon();
    }
  }

  function adviceFor(entry: LauncherEntry | null): string {
    if (!entry) return 'Select an item from the left to inspect it.';

    switch (entry.status) {
      case 'ok':
        return 'This item already looks healthy.';
      case 'exe_detected_needs_fixed_icon':
        return 'This item points to a Windows EXE and is ready for icon extraction and rewrite.';
      case 'broken_icon_path':
        return 'The launcher references an icon file that no longer exists.';
      case 'missing_icon':
        return 'The launcher has no icon value. If it points to a Windows EXE, repair can help.';
      case 'missing_exec_target':
        return 'The referenced EXE target does not exist right now. Fix the target first.';
      case 'invalid_desktop_file':
        return 'The desktop file could not be parsed correctly.';
      case 'unsupported_exec':
        return 'This item is outside the current Windows launcher repair flow.';
      case 'direct_exe_link':
        return 'This is a direct desktop link to a Windows EXE. Fix will convert it into a proper desktop launcher and keep a backup of the original link.';
      default:
        return 'Inspect the item details and decide how to proceed.';
    }
  }

  function repairMode(entry: LauncherEntry | null): string {
    if (!entry) return 'None';
    if (entry.status === 'direct_exe_link') return 'Convert EXE link';
    if (entry.status === 'exe_detected_needs_fixed_icon') return 'Rewrite launcher icon';
    if (entry.status === 'broken_icon_path') return 'Replace broken icon path';
    return 'No automatic repair';
  }

  function resultText(entry: LauncherEntry | null): string {
    if (!entry) return 'Not processed yet';
    if (entry.backupPath) return 'Backup created';
    if (entry.status === 'ok') return 'No action needed';
    return 'Not processed yet';
  }

  $: filteredEntries = entries.filter((entry) => {
    const haystack = `${entry.name} ${entry.path} ${entry.exec} ${entry.icon ?? ''}`.toLowerCase();
    const matchesQuery = query.trim() === '' || haystack.includes(query.toLowerCase());
    const matchesStatus = statusFilter === 'all' || entry.status === statusFilter;

    const kind = kindOf(entry);
    const matchesKind =
      kindFilter === 'all' ||
      (kindFilter === 'launcher' && kind === 'launcher') ||
      (kindFilter === 'exe_link' && kind === 'exe_link');

    return matchesQuery && matchesStatus && matchesKind;
  });

  $: shownCount = filteredEntries.length;
  $: selectedIconUrl = selectedPreviewUrl;
  $: selectedExecName = selected?.targetPath
    ? selected.targetPath.split(/[\\/]/).filter(Boolean).pop() ?? selected.targetPath
    : 'None';
  $: selectedHasThemeIcon = !!selected?.icon && !selected?.resolvedIconPath;

  $: if ((selected?.path ?? '') !== lastSelectedPath) {
    lastSelectedPath = selected?.path ?? '';
    iconLoadFailed = false;
  }

  $: if ((selected?.resolvedIconPath ?? '') !== selectedPreviewFor) {
    selectedPreviewFor = selected?.resolvedIconPath ?? '';
    void loadSelectedPreview();
  }

  onMount(() => {
    let cancelled = false;

    const handleContextMenu = (event: MouseEvent) => {
      if (!shouldAllowContextMenuTarget(event.target)) {
        event.preventDefault();
      }
    };

    const handleWindowClick = () => {
      closeContextMenu();
    };

    const boot = async () => {
      await tick();

      if (!cancelled) {
        await scan();
      }

      if (!cancelled) {
        await new Promise((resolve) => setTimeout(resolve, 800));
        await scan();
      }
    };

    void boot();
    window.addEventListener('keydown', handleGlobalKeydown);
    window.addEventListener('contextmenu', handleContextMenu);
    window.addEventListener('click', handleWindowClick);

    return () => {
      cancelled = true;
      window.removeEventListener('keydown', handleGlobalKeydown);
      window.removeEventListener('contextmenu', handleContextMenu);
      window.removeEventListener('click', handleWindowClick);
    };
  });
</script>

<svelte:head>
  <title>KDE Icon Helper</title>
</svelte:head>

<div class="app">
  <header class="topbar">
    <div class="brand"><img class="brandIcon" src={appIcon} alt="KDE Icon Helper" /></div>

    <div class="toolbar">
      <div class="searchWrap">
        <input type="text" placeholder="Search" bind:value={query} />
      </div>

      <div class="selectWrap">
        <select bind:value={statusFilter}>
          <option value="all">All</option>
          <option value="ok">Healthy</option>
          <option value="exe_detected_needs_fixed_icon">Needs icon fix</option>
          <option value="direct_exe_link">Direct EXE link</option>
          <option value="broken_icon_path">Broken icon path</option>
          <option value="missing_icon">Missing icon</option>
          <option value="missing_exec_target">Missing EXE target</option>
          <option value="invalid_desktop_file">Invalid desktop file</option>
          <option value="unsupported_exec">Unsupported item</option>
        </select>
      </div>

      <div class="selectWrap">
        <select bind:value={kindFilter}>
          <option value="all">All items</option>
          <option value="launcher">Launchers</option>
          <option value="exe_link">EXE links</option>
        </select>
      </div>

      <div class="pill">{shownCount} items</div>

      <button class="primary" type="button" on:click={scan} disabled={busy}>
        {busy ? 'Working…' : 'Scan'}
      </button>
    </div>
  </header>

  <main class="workspace">
    <aside class="panel listPanel">
      <div class="panelHeader">
        <div class="panelTitle">Items</div>
      </div>

      <div class="listScroll">
        {#if filteredEntries.length === 0}
          <div class="empty">
            <strong>No items found</strong>
            <span>Try a different search or filter.</span>
          </div>
        {/if}

        {#each filteredEntries as entry}
          {@const rowIconUrl = listIconUrl(entry)}
          <button
            type="button"
            data-item-path={entry.path}
            class:selected={selected?.path === entry.path}
            class="itemCard"
            on:click={() => { selected = entry; closeContextMenu(); }}
            on:contextmenu={(event) => openItemContextMenu(event, entry)}
          >
            <div class="itemIcon">
              {#if rowIconUrl}
                <img src={rowIconUrl} alt={`Icon for ${entry.name}`} />
              {:else}
                <span>{entry.status === 'direct_exe_link' ? 'EXE' : 'APP'}</span>
              {/if}
            </div>

            <div class="itemName" title={entry.name}>{entry.name}</div>

            <div class="itemStatus">
              <span class={statusClass(entry.status)}>{statusText(entry.status)}</span>
            </div>
          </button>
        {/each}
      </div>
    </aside>

    <section class="panel inspectorPanel">
      <div class="panelHeader">
        <div class="panelTitle">Inspector</div>
      </div>

      {#if selected}
        <div class="inspectorScroll">
          <div class="field">
            <div class="label">Name</div>
            <div class="value">{selected.name}</div>
          </div>

          <div class="field">
            <div class="label">Status</div>
            <div class="value">
              <span class={statusClass(selected.status)}>{statusText(selected.status)}</span>
            </div>
          </div>

          <div class="field previewField">
            <div class="label">Preview</div>
            <div class="preview">
              {#if selectedIconUrl && !iconLoadFailed}
                <img
                  src={selectedIconUrl}
                  alt={`Current icon for ${selected.name}`}
                  on:error={() => (iconLoadFailed = true)}
                />
              {:else if selectedHasThemeIcon}
                <div class="fallback">
                  <div class="fallbackGlyph">☆</div>
                  <strong>Theme icon</strong>
                  <span>The icon name was found, but no preview file could be loaded yet.</span>
                </div>
              {:else}
                <div class="fallback">
                  <div class="fallbackGlyph">{selected.status === 'direct_exe_link' ? 'EXE' : '?'}</div>
                  <strong>No preview available</strong>
                  <span>The current icon is missing, broken, or not previewable yet.</span>
                </div>
              {/if}
            </div>
          </div>

          <div class="field">
            <div class="label">Actions</div>
            <div class="inspectorActions">
              <button type="button" on:click={checkSelected} disabled={busy || !selected}>Check</button>
              <button class="primary" type="button" on:click={fixSelected} disabled={busy || !selected}>Fix</button>
              <button type="button" on:click={setManualIcon} disabled={busy || !selected}>Manual</button>
              <button type="button" on:click={restoreDefaultIcon} disabled={busy || !selected}>Restore</button>
            </div>
          </div>

          <div class="facts">
            <div class="factKey">Desktop item</div>
            <div class="factValue code">{selected.path}</div>

            <div class="factKey">Target EXE</div>
            <div class="factValue code">{selected.targetPath ?? 'None'}</div>

            <div class="factKey">Icon value</div>
            <div class="factValue code">{selected.icon ?? 'None'}</div>

            <div class="factKey">Resolved icon</div>
            <div class="factValue code">{selected.resolvedIconPath ?? 'None'}</div>

            <div class="factKey">Target name</div>
            <div class="factValue">{selectedExecName}</div>

            <div class="factKey">Message</div>
            <div class="factValue">{selected.message ?? 'No message available.'}</div>
          </div>
        </div>
      {:else}
        <div class="empty">
          <strong>No item selected</strong>
          <span>Pick one from the list to inspect it.</span>
        </div>
      {/if}
    </section>
  </main>

  {#if contextMenuOpen && contextMenuEntry}
    <div
      class="contextMenu"
      role="menu"
      tabindex="-1"
      style={`left:${contextMenuX}px; top:${contextMenuY}px;`}
      on:click|stopPropagation
      on:keydown={(event) => {
        if (event.key === 'Escape') {
          closeContextMenu();
        }
      }}
    >
      <button type="button" class="contextMenuItem" on:click={() => runContextAction('check')}>Check selected</button>
      <button type="button" class="contextMenuItem" on:click={() => runContextAction('fix')}>Fix selected</button>
      <button type="button" class="contextMenuItem" on:click={() => runContextAction('manual')}>Set icon manually</button>
      <button type="button" class="contextMenuItem" on:click={() => runContextAction('restore')}>Restore default icon</button>
    </div>
  {/if}

  <section class="panel logPanel">
    <div class="panelHeader logHeader">
      <div class="panelTitle">Log</div>
      <button type="button" class="ghost" on:click={() => (logOpen = !logOpen)}>
        {logOpen ? 'Hide' : 'Show'}
      </button>
    </div>

    {#if logOpen}
      <div class="logScroll">
        {#if log.length === 0}
          <div class="empty compact">
            <strong>No activity yet</strong>
            <span>Logs will appear here after scans and repairs.</span>
          </div>
        {:else}
          {#each log as line}
            <div class="logLine">{line}</div>
          {/each}
        {/if}
      </div>
    {/if}
  </section>
</div>
