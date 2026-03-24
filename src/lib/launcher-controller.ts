import { tick } from 'svelte';
import { get, writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import {
  availableEntryActions as availableEntryActionsForEntry,
  canRunEntryAction as canRunEntryActionForEntry,
  kindOf,
  statusText,
  type ContextAction,
  type ContextMenuMode,
  type InputContextAction,
  type KindFilter,
  type StatusFilter
} from '$lib/launcher-ui';
import type {
  CleanupResult,
  FixResult,
  GeneratedAssetStats,
  LauncherEntry,
  RuntimeDiagnostics
} from '$lib/types';

const KEYBOARD_PAGE_STEP = 8;
const MAX_PRELOADED_LIST_ICONS = 80;
const MAX_LOG_LINES = 250;
const BOOT_RESCAN_DELAY_MS = 850;
const BOOT_ICON_RETRY_DELAY_MS = 900;

export interface LauncherControllerState {
  entries: LauncherEntry[];
  selected: LauncherEntry | null;
  busy: boolean;
  query: string;
  statusFilter: StatusFilter;
  kindFilter: KindFilter;
  log: string[];
  logOpen: boolean;

  contextMenuOpen: boolean;
  contextMenuX: number;
  contextMenuY: number;
  contextMenuEntry: LauncherEntry | null;
  contextMenuMode: ContextMenuMode;

  iconLoadFailed: boolean;
  selectedPreviewUrl: string | null;
  itemIconUrls: Record<string, string>;
  itemIconLoading: Record<string, boolean>;
  previewDataUrls: Record<string, string>;

  filteredEntries: LauncherEntry[];
  shownCount: number;
  selectedIconUrl: string | null;
  selectedExecName: string;
  selectedHasThemeIcon: boolean;

  diagnostics: RuntimeDiagnostics | null;
  diagnosticsOpen: boolean;
  diagnosticsBusy: boolean;
  diagnosticsMissingCount: number;

  maintenance: GeneratedAssetStats | null;
  maintenanceOpen: boolean;
  maintenanceBusy: boolean;
  lastCleanupResult: CleanupResult | null;
}

function initialState(): LauncherControllerState {
  return {
    entries: [],
    selected: null,
    busy: false,
    query: '',
    statusFilter: 'all',
    kindFilter: 'all',
    log: [],
    logOpen: true,

    contextMenuOpen: false,
    contextMenuX: 0,
    contextMenuY: 0,
    contextMenuEntry: null,
    contextMenuMode: 'entry',

    iconLoadFailed: false,
    selectedPreviewUrl: null,
    itemIconUrls: {},
    itemIconLoading: {},
    previewDataUrls: {},

    filteredEntries: [],
    shownCount: 0,
    selectedIconUrl: null,
    selectedExecName: 'None',
    selectedHasThemeIcon: false,

    diagnostics: null,
    diagnosticsOpen: false,
    diagnosticsBusy: false,
    diagnosticsMissingCount: 0,

    maintenance: null,
    maintenanceOpen: false,
    maintenanceBusy: false,
    lastCleanupResult: null
  };
}

export function createLauncherController() {
  const store = writable<LauncherControllerState>(initialState());

  let destroyed = false;
  let contextMenuInput: HTMLInputElement | HTMLTextAreaElement | null = null;

  function current() {
    return get(store);
  }

  function matchesQuery(entry: LauncherEntry, query: string) {
    const text = `${entry.name} ${entry.path} ${entry.exec} ${entry.icon ?? ''}`.toLowerCase();
    return query.trim() === '' || text.includes(query.toLowerCase());
  }

  function matchesStatus(entry: LauncherEntry, statusFilter: StatusFilter) {
    return statusFilter === 'all' || entry.status === statusFilter;
  }

  function matchesKind(entry: LauncherEntry, kindFilter: KindFilter) {
    const kind = kindOf(entry);
    return (
      kindFilter === 'all' ||
      (kindFilter === 'launcher' && kind === 'launcher') ||
      (kindFilter === 'exe_link' && kind === 'exe_link')
    );
  }

  function deriveState(state: LauncherControllerState): LauncherControllerState {
    const filteredEntries = state.entries.filter(
      (entry) =>
        matchesQuery(entry, state.query) &&
        matchesStatus(entry, state.statusFilter) &&
        matchesKind(entry, state.kindFilter)
    );

    let selected = state.selected;
    if (filteredEntries.length === 0) {
      selected = null;
    } else if (selected && !filteredEntries.some((entry) => entry.path === selected?.path)) {
      selected = filteredEntries[0];
    }

    const selectedExecName = selected?.targetPath
      ? selected.targetPath.split(/[\\/]/).filter(Boolean).pop() ?? selected.targetPath
      : 'None';

    const selectedHasThemeIcon = !!selected?.icon && !selected?.resolvedIconPath;
    const diagnosticsMissingCount = state.diagnostics
      ? state.diagnostics.tools.filter((tool) => !tool.found).length
      : 0;

    return {
      ...state,
      selected,
      filteredEntries,
      shownCount: filteredEntries.length,
      selectedIconUrl: state.selectedPreviewUrl,
      selectedExecName,
      selectedHasThemeIcon,
      diagnosticsMissingCount
    };
  }

  function patch(mutator: (state: LauncherControllerState) => LauncherControllerState) {
    const before = current();
    const prevSelectedPath = before.selected?.path ?? '';
    const prevResolvedPath = before.selected?.resolvedIconPath ?? '';

    store.update((state) => deriveState(mutator(state)));
    const after = current();

    const nextSelectedPath = after.selected?.path ?? '';
    const nextResolvedPath = after.selected?.resolvedIconPath ?? '';

    if (nextSelectedPath !== prevSelectedPath) {
      store.update((state) => deriveState({ ...state, iconLoadFailed: false }));
    }

    if (nextResolvedPath !== prevResolvedPath) {
      store.update((state) => deriveState({ ...state, selectedPreviewUrl: null }));
      void loadSelectedPreview();
    }
  }

  function pushLog(message: string) {
    patch((state) => ({
      ...state,
      log: [`${new Date().toLocaleTimeString()} ${message}`, ...state.log].slice(0, MAX_LOG_LINES)
    }));
  }

  async function withBusy(task: () => Promise<void>) {
    patch((state) => ({ ...state, busy: true }));
    try {
      await task();
    } finally {
      patch((state) => ({ ...state, busy: false }));
    }
  }

  function closeContextMenu() {
    contextMenuInput = null;
    patch((state) => ({
      ...state,
      contextMenuOpen: false,
      contextMenuEntry: null,
      contextMenuMode: 'entry'
    }));
  }

  function openContextMenuAt(
    x: number,
    y: number,
    mode: ContextMenuMode,
    entry: LauncherEntry | null = null,
    input: HTMLInputElement | HTMLTextAreaElement | null = null
  ) {
    contextMenuInput = input;
    patch((state) => ({
      ...state,
      contextMenuOpen: true,
      contextMenuX: x,
      contextMenuY: y,
      contextMenuMode: mode,
      contextMenuEntry: entry
    }));
  }

  function selectEntry(entry: LauncherEntry) {
    patch((state) => ({ ...state, selected: entry }));
    closeContextMenu();
  }

  function selectFromEntries(
    nextEntries: LauncherEntry[],
    preferredPath?: string | null,
    fallbackPath?: string | null
  ) {
    let nextSelected: LauncherEntry | null = null;

    if (nextEntries.length > 0) {
      if (preferredPath) {
        nextSelected = nextEntries.find((entry) => entry.path === preferredPath) ?? null;
      }

      if (!nextSelected && fallbackPath) {
        nextSelected = nextEntries.find((entry) => entry.path === fallbackPath) ?? null;
      }

      if (!nextSelected) {
        const currentSelectedPath = current().selected?.path;
        nextSelected =
          nextEntries.find((entry) => entry.path === currentSelectedPath) ?? nextEntries[0];
      }
    }

    patch((state) => ({
      ...state,
      entries: nextEntries,
      selected: nextSelected
    }));
  }

  async function focusSelectedIntoView() {
    const path = current().selected?.path;
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
    const state = current();
    if (state.filteredEntries.length === 0) return;

    const clampedIndex = Math.max(0, Math.min(state.filteredEntries.length - 1, index));
    patch((prev) => ({
      ...prev,
      selected: prev.filteredEntries[clampedIndex]
    }));
    await focusSelectedIntoView();
  }

  async function selectRelative(delta: number) {
    const state = current();
    if (state.filteredEntries.length === 0) return;

    const currentIndex = state.selected
      ? state.filteredEntries.findIndex((entry) => entry.path === state.selected?.path)
      : -1;

    const nextIndex =
      currentIndex === -1
        ? 0
        : Math.max(0, Math.min(state.filteredEntries.length - 1, currentIndex + delta));

    await selectFilteredIndex(nextIndex);
  }

  async function ensureListIcon(entry: LauncherEntry) {
    const state = current();
    const path = entry.resolvedIconPath ?? null;
    if (!path) return;

    const cachedByResolvedPath = state.previewDataUrls[path];
    if (cachedByResolvedPath) {
      patch((prev) => ({
        ...prev,
        itemIconUrls: { ...prev.itemIconUrls, [entry.path]: cachedByResolvedPath }
      }));
      return;
    }

    if (state.itemIconUrls[entry.path] || state.itemIconLoading[entry.path]) return;

    patch((prev) => ({
      ...prev,
      itemIconLoading: { ...prev.itemIconLoading, [entry.path]: true }
    }));

    try {
      const result = await invoke<string | null>('load_icon_preview', { path });
      if (result) {
        patch((prev) => ({
          ...prev,
          previewDataUrls: { ...prev.previewDataUrls, [path]: result },
          itemIconUrls: { ...prev.itemIconUrls, [entry.path]: result }
        }));
      }
    } finally {
      patch((prev) => {
        const nextLoading = { ...prev.itemIconLoading };
        delete nextLoading[entry.path];
        return {
          ...prev,
          itemIconLoading: nextLoading
        };
      });
    }
  }

  async function preloadListIcons(entriesToLoad: LauncherEntry[]) {
    const state = current();
    const wanted = entriesToLoad
      .filter((entry) => !!entry.resolvedIconPath)
      .filter((entry) => !state.itemIconUrls[entry.path])
      .slice(0, MAX_PRELOADED_LIST_ICONS);

    if (wanted.length === 0) return;

    await Promise.allSettled(wanted.map((entry) => ensureListIcon(entry)));
  }

  function needsBootIconRetry(entriesToCheck: LauncherEntry[]) {
    const state = current();
    return entriesToCheck.some((entry) => {
      if (entry.status === 'direct_exe_link') return false;
      if (!entry.icon && !entry.resolvedIconPath) return false;
      return !state.itemIconUrls[entry.path];
    });
  }

  async function hydrateListIconsAfterBoot(isCancelled: () => boolean) {
    await preloadListIcons(current().entries);

    if (isCancelled()) return;
    if (!needsBootIconRetry(current().entries)) return;

    await new Promise((resolve) => setTimeout(resolve, BOOT_ICON_RETRY_DELAY_MS));

    if (isCancelled()) return;

    const preferredPath = current().selected?.path ?? null;
    await refreshEntries(preferredPath, preferredPath);

    if (isCancelled()) return;

    await preloadListIcons(current().entries);
  }

  async function loadSelectedPreview() {
    const state = current();
    const path = state.selected?.resolvedIconPath ?? null;

    if (!path) {
      patch((prev) => ({ ...prev, selectedPreviewUrl: null }));
      return;
    }

    const cached = state.previewDataUrls[path];
    if (cached) {
      patch((prev) => ({ ...prev, selectedPreviewUrl: cached }));
      return;
    }

    const currentPath = path;

    try {
      const result = await invoke<string | null>('load_icon_preview', { path: currentPath });
      const latestSelectedPath = current().selected?.resolvedIconPath ?? null;

      if (latestSelectedPath === currentPath) {
        patch((prev) => ({
          ...prev,
          selectedPreviewUrl: result,
          previewDataUrls: result
            ? { ...prev.previewDataUrls, [currentPath]: result }
            : prev.previewDataUrls
        }));
      }
    } catch {
      if ((current().selected?.resolvedIconPath ?? null) === currentPath) {
        patch((prev) => ({ ...prev, selectedPreviewUrl: null }));
      }
    }
  }

  async function refreshMaintenance(silent = false) {
    patch((state) => ({ ...state, maintenanceBusy: true }));

    try {
      const result = await invoke<GeneratedAssetStats>('get_generated_asset_stats');
      patch((state) => ({ ...state, maintenance: result }));

      if (!silent) {
        pushLog('Maintenance stats refreshed.');
      }
    } catch (error) {
      pushLog(`Maintenance stats failed: ${String(error)}`);
    } finally {
      patch((state) => ({ ...state, maintenanceBusy: false }));
    }
  }

  async function runGeneratedCleanup(dryRun: boolean) {
    patch((state) => ({ ...state, maintenanceBusy: true }));

    try {
      const result = await invoke<CleanupResult>('cleanup_generated_assets', { dryRun });
      patch((state) => ({
        ...state,
        lastCleanupResult: result,
        maintenance: result.statsAfter
      }));

      if (dryRun) {
        pushLog(
          `Cleanup dry run: ${result.removedFilesCount} orphaned auto icon(s), ${result.removedBytes} bytes.`
        );
      } else {
        pushLog(
          `Cleanup applied: ${result.removedFilesCount} orphaned auto icon(s), ${result.removedBytes} bytes removed.`
        );
      }
    } catch (error) {
      pushLog(`Cleanup failed: ${String(error)}`);
    } finally {
      patch((state) => ({ ...state, maintenanceBusy: false }));
    }
  }

  async function applyEntries(
    nextEntries: LauncherEntry[],
    preferredPath?: string | null,
    fallbackPath?: string | null
  ) {
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
    await refreshMaintenance(true);
  }

  async function runSelectedFixCommand(
    command: 'fix_launcher_icon' | 'restore_launcher_icon_default',
    failureMessage: string
  ) {
    const selected = current().selected;
    if (!selected) return;

    await withBusy(async () => {
      const previousPath = selected.path;

      try {
        const result = await invoke<FixResult>(command, { path: previousPath });
        await applyFixResult(result, previousPath);
      } catch (error) {
        pushLog(`${failureMessage}: ${String(error)}`);
      }
    });
  }

  async function scan(options?: {
    silent?: boolean;
    preferredPath?: string | null;
    fallbackPath?: string | null;
  }) {
    await withBusy(async () => {
      try {
        const result = await invoke<LauncherEntry[]>('scan_launchers');
        await applyEntries(result, options?.preferredPath, options?.fallbackPath);

        if (!options?.silent) {
          pushLog(`Scan finished. ${result.length} desktop item(s) found.`);
        }
      } catch (error) {
        pushLog(`Scan failed: ${String(error)}`);
      }
    });
  }

  async function checkSelected() {
    const selected = current().selected;
    if (!selected) return;

    await withBusy(async () => {
      try {
        const updated = await invoke<LauncherEntry>('check_launcher', { path: selected.path });
        const nextEntries = current().entries.map((entry) =>
          entry.path === updated.path ? updated : entry
        );
        await applyEntries(nextEntries, updated.path);
        pushLog(`Checked ${updated.name}. Status is now ${statusText(updated.status)}.`);
      } catch (error) {
        pushLog(`Check failed: ${String(error)}`);
      }
    });
  }

  async function fixSelected() {
    if (!canRunEntryAction('fix')) return;
    await runSelectedFixCommand('fix_launcher_icon', 'Fix failed');
  }

  async function restoreDefaultIcon() {
    if (!canRunEntryAction('restore')) return;
    await runSelectedFixCommand('restore_launcher_icon_default', 'Restore default icon failed');
  }

  async function setManualIcon() {
    if (!canRunEntryAction('manual')) return;

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
      const previousPath = current().selected?.path;
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
    if (!canRunEntryAction(action)) return;

    const handlers: Record<ContextAction, () => Promise<void>> = {
      check: checkSelected,
      fix: fixSelected,
      manual: setManualIcon,
      restore: restoreDefaultIcon
    };

    await handlers[action]();
  }

  function canRunEntryAction(action: ContextAction, entry: LauncherEntry | null = current().selected) {
    return canRunEntryActionForEntry(action, entry);
  }

  function availableEntryActions(entry: LauncherEntry | null) {
    return availableEntryActionsForEntry(entry);
  }

  async function refreshDiagnostics(silent = false) {
    patch((state) => ({ ...state, diagnosticsBusy: true }));

    try {
      const result = await invoke<RuntimeDiagnostics>('get_runtime_diagnostics');
      patch((state) => ({ ...state, diagnostics: result }));

      if (!silent) {
        pushLog('Runtime diagnostics refreshed.');
      }

      const missing = result.tools.filter((tool) => !tool.found);
      if (!silent && missing.length > 0) {
        pushLog(`Missing tools: ${missing.map((tool) => tool.name).join(', ')}`);
      }
    } catch (error) {
      pushLog(`Diagnostics failed: ${String(error)}`);
    } finally {
      patch((state) => ({ ...state, diagnosticsBusy: false }));
    }
  }

  function setQuery(query: string) {
    patch((state) => ({ ...state, query }));
  }

  function setStatusFilter(statusFilter: StatusFilter) {
    patch((state) => ({ ...state, statusFilter }));
  }

  function setKindFilter(kindFilter: KindFilter) {
    patch((state) => ({ ...state, kindFilter }));
  }

  function toggleLogOpen() {
    patch((state) => ({ ...state, logOpen: !state.logOpen }));
  }

  function toggleDiagnosticsOpen() {
    patch((state) => ({ ...state, diagnosticsOpen: !state.diagnosticsOpen }));
  }

  function toggleMaintenanceOpen() {
    patch((state) => ({ ...state, maintenanceOpen: !state.maintenanceOpen }));
  }

  function setIconLoadFailed(value: boolean) {
    patch((state) => ({ ...state, iconLoadFailed: value }));
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

    patch((state) => ({ ...state, selected: entry }));
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
    const entry = current().contextMenuEntry;
    closeContextMenu();
    if (!entry || !canRunEntryAction(action, entry)) return;

    patch((state) => ({ ...state, selected: entry }));
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

    const state = current();
    if (shouldIgnoreKeyTarget(event.target)) return;
    if (state.filteredEntries.length === 0) return;

    const currentIndex = state.selected
      ? state.filteredEntries.findIndex((entry) => entry.path === state.selected?.path)
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
      void selectFilteredIndex(0);
      return;
    }

    if (event.key === 'End') {
      event.preventDefault();
      closeContextMenu();
      void selectFilteredIndex(state.filteredEntries.length - 1);
      return;
    }

    if (event.key === 'PageDown') {
      event.preventDefault();
      closeContextMenu();
      const nextIndex =
        currentIndex === -1
          ? 0
          : Math.min(state.filteredEntries.length - 1, currentIndex + KEYBOARD_PAGE_STEP);
      void selectFilteredIndex(nextIndex);
      return;
    }

    if (event.key === 'PageUp') {
      event.preventDefault();
      closeContextMenu();
      const nextIndex = currentIndex === -1 ? 0 : Math.max(0, currentIndex - KEYBOARD_PAGE_STEP);
      void selectFilteredIndex(nextIndex);
    }
  }

  function mount() {
    destroyed = false;

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

    document.addEventListener('contextmenu', handleDocumentContextMenu, true);
    window.addEventListener('keydown', handleGlobalKeydown);
    window.addEventListener('contextmenu', handleWindowContextMenu);
    window.addEventListener('click', handleWindowClick);

    void (async () => {
      await tick();

      if (!destroyed) {
        await scan({ silent: true });
      }

      if (!destroyed) {
        await preloadListIcons(current().entries);
      }

      if (!destroyed) {
        await new Promise((resolve) => setTimeout(resolve, BOOT_RESCAN_DELAY_MS));
      }

      if (!destroyed) {
        const preferredPath = current().selected?.path ?? null;
        await scan({
          silent: true,
          preferredPath,
          fallbackPath: preferredPath
        });
      }

      if (!destroyed) {
        await preloadListIcons(current().entries);
      }

      if (!destroyed) {
        await refreshDiagnostics(true);
        await refreshMaintenance(true);
      }

      if (!destroyed) {
        pushLog(`Startup ready. ${current().entries.length} desktop item(s) loaded.`);

        const diagnostics = current().diagnostics;
        if (diagnostics) {
          if (!diagnostics.desktopDirExists) {
            pushLog(`Desktop directory missing: ${diagnostics.desktopDir}`);
          }

          const missing = diagnostics.tools.filter((tool) => !tool.found);
          if (missing.length > 0) {
            pushLog(`Missing tools: ${missing.map((tool) => tool.name).join(', ')}`);
          }
        }

        const maintenance = current().maintenance;
        if (maintenance && maintenance.orphanGeneratedIconsCount > 0) {
          pushLog(
            `Maintenance: ${maintenance.orphanGeneratedIconsCount} orphaned auto icon(s) detected.`
          );
        }
      }

      if (!destroyed) {
        try {
          await invoke('reveal_main_window');
        } catch (error) {
          pushLog(`Window reveal failed: ${String(error)}`);
        }
      }

      if (!destroyed) {
        await hydrateListIconsAfterBoot(() => destroyed);
      }
    })();

    return () => {
      destroyed = true;
      document.removeEventListener('contextmenu', handleDocumentContextMenu, true);
      window.removeEventListener('keydown', handleGlobalKeydown);
      window.removeEventListener('contextmenu', handleWindowContextMenu);
      window.removeEventListener('click', handleWindowClick);
    };
  }

  return {
    subscribe: store.subscribe,
    mount,
    scan,
    selectEntry,
    openItemContextMenu,
    runEntryAction,
    runContextAction,
    runInputContextAction,
    handleContextMenuEscape,
    listIconUrl: (entry: LauncherEntry) => {
      const value = current().itemIconUrls[entry.path];
      return value && value.length > 0 ? value : null;
    },
    canRunEntryAction,
    availableEntryActions,
    setQuery,
    setStatusFilter,
    setKindFilter,
    toggleLogOpen,
    toggleDiagnosticsOpen,
    toggleMaintenanceOpen,
    refreshDiagnostics,
    refreshMaintenance,
    runGeneratedCleanup,
    setIconLoadFailed
  };
}
