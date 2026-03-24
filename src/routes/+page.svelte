<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import appIcon from '$lib/assets/kde-icon-helper.svg';
  import type { FixResult, LauncherEntry, LauncherStatus } from '$lib/types';

  type StatusFilter = 'all' | LauncherStatus;
  type KindFilter = 'all' | 'launcher' | 'exe_link';
  type ContextMenuMode = 'entry' | 'input';
  type ContextAction = 'check' | 'fix' | 'manual' | 'restore';
  type InputContextAction = 'cut' | 'copy' | 'paste' | 'selectAll';

  const BOOT_RESCAN_DELAY_MS = 800;
  const KEYBOARD_PAGE_STEP = 8;
  const MAX_PRELOADED_LIST_ICONS = 80;
  const MAX_LOG_LINES = 250;

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

  const statusFilterOptions: Array<{ value: StatusFilter; label: string }> = [
    { value: 'all', label: 'All' },
    { value: 'ok', label: 'Healthy' },
    { value: 'exe_detected_needs_fixed_icon', label: 'Needs icon fix' },
    { value: 'direct_exe_link', label: 'Direct EXE link' },
    { value: 'broken_icon_path', label: 'Broken icon path' },
    { value: 'missing_icon', label: 'Missing icon' },
    { value: 'missing_exec_target', label: 'Missing EXE target' },
    { value: 'invalid_desktop_file', label: 'Invalid desktop file' },
    { value: 'unsupported_exec', label: 'Unsupported item' }
  ];

  const kindFilterOptions: Array<{ value: KindFilter; label: string }> = [
    { value: 'all', label: 'All items' },
    { value: 'launcher', label: 'Launchers' },
    { value: 'exe_link', label: 'EXE links' }
  ];

  const entryActionItems: Array<{
    id: ContextAction;
    label: string;
    contextLabel: string;
    primary?: boolean;
  }> = [
    { id: 'check', label: 'Check', contextLabel: 'Check selected' },
    { id: 'fix', label: 'Fix', contextLabel: 'Fix selected', primary: true },
    { id: 'manual', label: 'Manual', contextLabel: 'Set icon manually' },
    { id: 'restore', label: 'Restore', contextLabel: 'Restore default icon' }
  ];

  const inputActionItems: Array<{
    id: InputContextAction;
    label: string;
  }> = [
    { id: 'cut', label: 'Cut' },
    { id: 'copy', label: 'Copy' },
    { id: 'paste', label: 'Paste' },
    { id: 'selectAll', label: 'Select all' }
  ];

  const entryActionHandlers: Record<ContextAction, () => Promise<void>> = {
    check: () => checkSelected(),
    fix: () => fixSelected(),
    manual: () => setManualIcon(),
    restore: () => restoreDefaultIcon()
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
  let contextMenuMode: ContextMenuMode = 'entry';
  let contextMenuInput: HTMLInputElement | HTMLTextAreaElement | null = null;

  let iconLoadFailed = false;
  let selectedPreviewUrl: string | null = null;
  let selectedPreviewFor = '';
  let lastSelectedPath = '';
  let itemIconUrls: Record<string, string> = {};
  let itemIconLoading: Record<string, boolean> = {};

  let filteredEntries: LauncherEntry[] = [];
  let shownCount = 0;
  let selectedIconUrl: string | null = null;
  let selectedExecName = 'None';
  let selectedHasThemeIcon = false;

  function pushLog(message: string) {
    log = [`${new Date().toLocaleTimeString()} ${message}`, ...log].slice(0, MAX_LOG_LINES);
  }

  async function withBusy(task: () => Promise<void>) {
    busy = true;
    try {
      await task();
    } finally {
      busy = false;
    }
  }

  function statusText(status?: string | null) {
    return status ? statusLabel[status] ?? status : 'Unknown';
  }

  function statusClass(status?: string | null) {
    const tone = status ? statusTone[status] ?? 'muted' : 'muted';
    return `badge ${tone}`;
  }

  function rowGlyph(entry: LauncherEntry) {
    return entry.status === 'direct_exe_link' ? 'EXE' : 'APP';
  }

  function previewFallbackGlyph(entry: LauncherEntry) {
    return entry.status === 'direct_exe_link' ? 'EXE' : '?';
  }

  function isSelectedEntry(entry: LauncherEntry) {
    return selected?.path === entry.path;
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

  function entrySearchText(entry: LauncherEntry) {
    return `${entry.name} ${entry.path} ${entry.exec} ${entry.icon ?? ''}`.toLowerCase();
  }

  function matchesQuery(entry: LauncherEntry) {
    return query.trim() === '' || entrySearchText(entry).includes(query.toLowerCase());
  }

  function matchesStatus(entry: LauncherEntry) {
    return statusFilter === 'all' || entry.status === statusFilter;
  }

  function matchesKind(entry: LauncherEntry) {
    const kind = kindOf(entry);
    return (
      kindFilter === 'all' ||
      (kindFilter === 'launcher' && kind === 'launcher') ||
      (kindFilter === 'exe_link' && kind === 'exe_link')
    );
  }

  function listIconUrl(entry: LauncherEntry): string | null {
    const value = itemIconUrls[entry.path];
    return value && value.length > 0 ? value : null;
  }

  function currentFilteredIndex() {
    return selected ? filteredEntries.findIndex((entry) => entry.path === selected?.path) : -1;
  }

  function closeContextMenu() {
    contextMenuOpen = false;
    contextMenuEntry = null;
    contextMenuInput = null;
    contextMenuMode = 'entry';
  }

  function openContextMenuAt(
    x: number,
    y: number,
    mode: ContextMenuMode,
    entry: LauncherEntry | null = null,
    input: HTMLInputElement | HTMLTextAreaElement | null = null
  ) {
    contextMenuMode = mode;
    contextMenuEntry = entry;
    contextMenuInput = input;
    contextMenuX = x;
    contextMenuY = y;
    contextMenuOpen = true;
  }

  function selectEntry(entry: LauncherEntry) {
    selected = entry;
    closeContextMenu();
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

  function selectFromEntries(
    nextEntries: LauncherEntry[],
    preferredPath?: string | null,
    fallbackPath?: string | null
  ) {
    if (nextEntries.length === 0) {
      selected = null;
      return;
    }

    if (preferredPath) {
      const preferred = nextEntries.find((entry) => entry.path === preferredPath);
      if (preferred) {
        selected = preferred;
        return;
      }
    }

    if (fallbackPath) {
      const fallback = nextEntries.find((entry) => entry.path === fallbackPath);
      if (fallback) {
        selected = fallback;
        return;
      }
    }

    restoreSelection(nextEntries);
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

  async function selectFilteredIndex(index: number) {
    if (filteredEntries.length === 0) return;

    const clampedIndex = Math.max(0, Math.min(filteredEntries.length - 1, index));
    selected = filteredEntries[clampedIndex];
    await focusSelectedIntoView();
  }

  async function selectRelative(delta: number) {
    if (filteredEntries.length === 0) return;

    const currentIndex = currentFilteredIndex();
    const nextIndex =
      currentIndex === -1
        ? 0
        : Math.max(0, Math.min(filteredEntries.length - 1, currentIndex + delta));

    await selectFilteredIndex(nextIndex);
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
      .slice(0, MAX_PRELOADED_LIST_ICONS);

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

  async function applyEntries(
    nextEntries: LauncherEntry[],
    preferredPath?: string | null,
    fallbackPath?: string | null
  ) {
    entries = nextEntries;
    selectFromEntries(nextEntries, preferredPath, fallbackPath);
    await preloadListIcons(nextEntries);
  }

  async function refreshEntries(preferredPath?: string | null, fallbackPath?: string | null) {
    const refreshed = await invoke<LauncherEntry[]>('scan_launchers');
    await applyEntries(refreshed, preferredPath, fallbackPath);
  }

  async function applyFixResult(result: FixResult, fallbackPath: string) {
    pushLog(result.message);
    await refreshEntries(result.updatedEntry?.path, fallbackPath);
  }

  async function runSelectedFixCommand(
    command: 'fix_launcher_icon' | 'restore_launcher_icon_default',
    failureMessage: string
  ) {
    if (!selected) return;

    await withBusy(async () => {
      const previousPath = selected?.path;
      if (!previousPath) return;

      try {
        const result = await invoke<FixResult>(command, { path: previousPath });
        await applyFixResult(result, previousPath);
      } catch (error) {
        pushLog(`${failureMessage}: ${String(error)}`);
      }
    });
  }

  async function scan() {
    await withBusy(async () => {
      try {
        const result = await invoke<LauncherEntry[]>('scan_launchers');
        await applyEntries(result);
        pushLog(`Scan finished. ${result.length} desktop item(s) found.`);
      } catch (error) {
        pushLog(`Scan failed: ${String(error)}`);
      }
    });
  }

  async function checkSelected() {
    const current = selected;
    if (!current) return;

    await withBusy(async () => {
      try {
        const updated = await invoke<LauncherEntry>('check_launcher', { path: current.path });
        const nextEntries = entries.map((entry) => (entry.path === updated.path ? updated : entry));
        await applyEntries(nextEntries, updated.path);
        pushLog(`Checked ${updated.name}. Status is now ${statusText(updated.status)}.`);
      } catch (error) {
        pushLog(`Check failed: ${String(error)}`);
      }
    });
  }

  async function fixSelected() {
    await runSelectedFixCommand('fix_launcher_icon', 'Fix failed');
  }

  async function restoreDefaultIcon() {
    await runSelectedFixCommand('restore_launcher_icon_default', 'Restore default icon failed');
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

    await withBusy(async () => {
      const previousPath = selected?.path;
      if (!previousPath) return;

      try {
        const result = await invoke<FixResult>('set_launcher_icon_manual', {
          path: previousPath,
          sourceIconPath: chosen
        });
        await applyFixResult(result, previousPath);
      } catch (error) {
        pushLog(`Manual icon failed: ${String(error)}`);
      }
    });
  }

  async function runEntryAction(action: ContextAction) {
    await entryActionHandlers[action]();
  }

  function shouldAllowContextMenuTarget(target: EventTarget | null): boolean {
    const el = target as HTMLElement | null;
    if (!el) return false;

    const directTag = el.tagName?.toLowerCase() ?? '';
    if (directTag === 'input' || directTag === 'textarea') return true;
    if (el.isContentEditable) return true;

    const editableParent = el.closest(
      'input, textarea, [contenteditable="true"], [contenteditable=""], [contenteditable]'
    );

    return !!editableParent;
  }

  function shouldIgnoreKeyTarget(target: EventTarget | null): boolean {
    const el = target as HTMLElement | null;
    if (!el) return false;

    const tag = el.tagName?.toLowerCase() ?? '';
    return tag === 'input' || tag === 'textarea' || tag === 'select' || el.isContentEditable;
  }

  function findEditableTarget(target: EventTarget | null): HTMLInputElement | HTMLTextAreaElement | null {
    const el = target as HTMLElement | null;
    if (!el) return null;

    const found = el.closest('input, textarea');
    if (found instanceof HTMLInputElement || found instanceof HTMLTextAreaElement) {
      return found;
    }

    return null;
  }

  function openItemContextMenu(event: MouseEvent, entry: LauncherEntry) {
    event.preventDefault();
    event.stopPropagation();

    selected = entry;
    openContextMenuAt(event.clientX, event.clientY, 'entry', entry, null);
  }

  function openInputContextMenu(event: MouseEvent) {
    const editable = findEditableTarget(event.target);
    if (!editable) return;

    event.preventDefault();
    event.stopPropagation();

    openContextMenuAt(event.clientX, event.clientY, 'input', null, editable);
  }

  function runDocumentEditCommand(command: 'copy' | 'cut') {
    document.execCommand(command);
  }

  async function pasteIntoInput(el: HTMLInputElement | HTMLTextAreaElement) {
    const clip = await navigator.clipboard.readText().catch(() => '');
    if (!clip || el.readOnly || el.disabled) return;

    const start = el.selectionStart ?? el.value.length;
    const end = el.selectionEnd ?? el.value.length;

    el.value = el.value.slice(0, start) + clip + el.value.slice(end);

    const pos = start + clip.length;
    el.setSelectionRange(pos, pos);
    el.dispatchEvent(new Event('input', { bubbles: true }));
  }

  async function runInputContextAction(action: InputContextAction) {
    const el = contextMenuInput;
    closeContextMenu();

    if (!el) return;

    el.focus();

    if (action === 'selectAll') {
      el.select();
      return;
    }

    if (action === 'copy') {
      runDocumentEditCommand('copy');
      return;
    }

    if (action === 'cut') {
      if (!el.readOnly && !el.disabled) {
        runDocumentEditCommand('cut');
      }
      return;
    }

    if (action === 'paste') {
      await pasteIntoInput(el);
    }
  }

  async function runContextAction(action: ContextAction) {
    const entry = contextMenuEntry;
    closeContextMenu();
    if (!entry) return;

    selected = entry;
    await tick();
    await runEntryAction(action);
  }

  function handleContextMenuEscape(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      closeContextMenu();
    }
  }

  function handleGlobalKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      closeContextMenu();
      return;
    }

    if (shouldIgnoreKeyTarget(event.target)) return;
    if (filteredEntries.length === 0) return;

    const currentIndex = currentFilteredIndex();

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
      void selectFilteredIndex(0);
      return;
    }

    if (event.key === 'End') {
      event.preventDefault();
      closeContextMenu();
      void selectFilteredIndex(filteredEntries.length - 1);
      return;
    }

    if (event.key === 'PageDown') {
      event.preventDefault();
      closeContextMenu();
      const nextIndex =
        currentIndex === -1
          ? 0
          : Math.min(filteredEntries.length - 1, currentIndex + KEYBOARD_PAGE_STEP);
      void selectFilteredIndex(nextIndex);
      return;
    }

    if (event.key === 'PageUp') {
      event.preventDefault();
      closeContextMenu();
      const nextIndex =
        currentIndex === -1 ? 0 : Math.max(0, currentIndex - KEYBOARD_PAGE_STEP);
      void selectFilteredIndex(nextIndex);
      return;
    }

    if (event.key === 'Escape') {
      closeContextMenu();
    }
  }

  $: filteredEntries = entries.filter(
    (entry) => matchesQuery(entry) && matchesStatus(entry) && matchesKind(entry)
  );

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

    const handleDocumentContextMenu = (event: MouseEvent) => {
      openInputContextMenu(event);
    };

    const handleWindowContextMenu = (event: MouseEvent) => {
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
        await new Promise((resolve) => setTimeout(resolve, BOOT_RESCAN_DELAY_MS));
        await scan();
      }
    };

    void boot();

    document.addEventListener('contextmenu', handleDocumentContextMenu, true);
    window.addEventListener('keydown', handleGlobalKeydown);
    window.addEventListener('contextmenu', handleWindowContextMenu);
    window.addEventListener('click', handleWindowClick);

    return () => {
      cancelled = true;
      document.removeEventListener('contextmenu', handleDocumentContextMenu, true);
      window.removeEventListener('keydown', handleGlobalKeydown);
      window.removeEventListener('contextmenu', handleWindowContextMenu);
      window.removeEventListener('click', handleWindowClick);
    };
  });
</script>

<svelte:head>
  <title>KDE Icon Helper</title>

  <style>
    /* compact two column head override */
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
        <input type="text" placeholder="Search" bind:value={query} />
      </div>

      <div class="selectWrap">
        <select bind:value={statusFilter}>
          {#each statusFilterOptions as option}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </div>

      <div class="selectWrap">
        <select bind:value={kindFilter}>
          {#each kindFilterOptions as option}
            <option value={option.value}>{option.label}</option>
          {/each}
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
            class:selected={isSelectedEntry(entry)}
            class="itemCard"
            on:click={() => selectEntry(entry)}
            on:contextmenu={(event) => openItemContextMenu(event, entry)}
          >
            <div class="itemIcon">
              {#if rowIconUrl}
                <img src={rowIconUrl} alt={`Icon for ${entry.name}`} />
              {:else}
                <span>{rowGlyph(entry)}</span>
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
                  <div class="fallbackGlyph">{previewFallbackGlyph(selected)}</div>
                  <strong>No preview available</strong>
                  <span>The current icon is missing, broken, or not previewable yet.</span>
                </div>
              {/if}
            </div>
          </div>

          <div class="field">
            <div class="label">Actions</div>
            <div class="inspectorActions">
              {#each entryActionItems as action}
                <button
                  type="button"
                  class:primary={!!action.primary}
                  on:click={() => runEntryAction(action.id)}
                  disabled={busy || !selected}
                >
                  {action.label}
                </button>
              {/each}
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

  {#if contextMenuOpen}
    <div
      class="contextMenu"
      role="menu"
      tabindex="-1"
      style={`left:${contextMenuX}px; top:${contextMenuY}px;`}
      on:click|stopPropagation
      on:keydown={handleContextMenuEscape}
    >
      {#if contextMenuMode === 'input'}
        {#each inputActionItems as action}
          <button
            type="button"
            class="contextMenuItem"
            on:click={() => runInputContextAction(action.id)}
          >
            {action.label}
          </button>
        {/each}
      {:else if contextMenuEntry}
        {#each entryActionItems as action}
          <button
            type="button"
            class="contextMenuItem"
            on:click={() => runContextAction(action.id)}
          >
            {action.contextLabel}
          </button>
        {/each}
      {/if}
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
