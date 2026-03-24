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
  BackupEntry,
  BackupRestoreResult,
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
const ICON_PRELOAD_CONCURRENCY = 6;
const UI_PREFERENCES_KEY = 'kde-icon-helper.ui-preferences.v1';

type UtilityTab = 'backups' | 'maintenance' | 'diagnostics';

type StoredUiPreferences = {
  query: string;
  statusFilter: StatusFilter;
  kindFilter: KindFilter;
  logOpen: boolean;
  utilityTab: UtilityTab;
};

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
  diagnosticsBusy: boolean;
  diagnosticsMissingCount: number;

  maintenance: GeneratedAssetStats | null;
  maintenanceBusy: boolean;
  lastCleanupResult: CleanupResult | null;

  backups: BackupEntry[];
  backupsBusy: boolean;
  backupsRestoreBusy: boolean;
  selectedBackupPath: string | null;

  utilityOpen: boolean;
  utilityTab: UtilityTab;
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
    diagnosticsBusy: false,
    diagnosticsMissingCount: 0,

    maintenance: null,
    maintenanceBusy: false,
    lastCleanupResult: null,

    backups: [],
    backupsBusy: false,
    backupsRestoreBusy: false,
    selectedBackupPath: null,

    utilityOpen: false,
    utilityTab: 'backups'
  };
}

export function createLauncherController() {
  const store = writable<LauncherControllerState>(initialState());

  let destroyed = false;
  let preferencesHydrated = false;
  let contextMenuInput: HTMLInputElement | HTMLTextAreaElement | null = null;
  let searchInput: HTMLInputElement | null = null;
  const inflightPreviewLoads = new Map<string, Promise<string | null>>();

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

    let selectedBackupPath = state.selectedBackupPath;
    const knownBackupPaths = new Set(state.backups.map((entry) => entry.path));
    if (state.backups.length === 0) {
      selectedBackupPath = null;
    } else if (!selectedBackupPath || !knownBackupPaths.has(selectedBackupPath)) {
      selectedBackupPath = state.backups[0].path;
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
      selectedBackupPath,
      filteredEntries,
      shownCount: filteredEntries.length,
      selectedIconUrl: state.selectedPreviewUrl,
      selectedExecName,
      selectedHasThemeIcon,
      diagnosticsMissingCount
    };
  }

  function persistUiPreferences(state: LauncherControllerState) {
    if (!preferencesHydrated) return;
    if (typeof window === 'undefined' || !window.localStorage) return;

    const prefs: StoredUiPreferences = {
      query: state.query,
      statusFilter: state.statusFilter,
      kindFilter: state.kindFilter,
      logOpen: state.logOpen,
      utilityTab: state.utilityTab
    };

    try {
      window.localStorage.setItem(UI_PREFERENCES_KEY, JSON.stringify(prefs));
    } catch {
      // ignore storage failures
    }
  }

  function readUiPreferences(): Partial<StoredUiPreferences> {
    if (typeof window === 'undefined' || !window.localStorage) {
      return {};
    }

    try {
      const raw = window.localStorage.getItem(UI_PREFERENCES_KEY);
      if (!raw) {
        return {};
      }

      const parsed = JSON.parse(raw) as Partial<StoredUiPreferences> | null;
      if (!parsed || typeof parsed !== 'object') {
        return {};
      }

      const next: Partial<StoredUiPreferences> = {};

      if (typeof parsed.query === 'string') {
        next.query = parsed.query;
      }

      if (
        parsed.statusFilter === 'all' ||
        parsed.statusFilter === 'ok' ||
        parsed.statusFilter === 'missing_icon' ||
        parsed.statusFilter === 'broken_icon_path' ||
        parsed.statusFilter === 'exe_detected_needs_fixed_icon' ||
        parsed.statusFilter === 'missing_exec_target' ||
        parsed.statusFilter === 'invalid_desktop_file' ||
        parsed.statusFilter === 'unsupported_exec' ||
        parsed.statusFilter === 'direct_exe_link'
      ) {
        next.statusFilter = parsed.statusFilter;
      }

      if (
        parsed.kindFilter === 'all' ||
        parsed.kindFilter === 'launcher' ||
        parsed.kindFilter === 'exe_link'
      ) {
        next.kindFilter = parsed.kindFilter;
      }

      if (typeof parsed.logOpen === 'boolean') {
        next.logOpen = parsed.logOpen;
      }

      if (
        parsed.utilityTab === 'backups' ||
        parsed.utilityTab === 'maintenance' ||
        parsed.utilityTab === 'diagnostics'
      ) {
        next.utilityTab = parsed.utilityTab;
      }

      return next;
    } catch {
      return {};
    }
  }

  function applyStoredUiPreferences() {
    const prefs = readUiPreferences();

    patch((state) => ({
      ...state,
      query: prefs.query ?? state.query,
      statusFilter: prefs.statusFilter ?? state.statusFilter,
      kindFilter: prefs.kindFilter ?? state.kindFilter,
      logOpen: prefs.logOpen ?? state.logOpen,
      utilityTab: prefs.utilityTab ?? state.utilityTab,
      utilityOpen: false
    }));

    preferencesHydrated = true;
    persistUiPreferences(current());
  }

  function resetUiPreferences() {
    if (typeof window !== 'undefined' && window.localStorage) {
      try {
        window.localStorage.removeItem(UI_PREFERENCES_KEY);
      } catch {
        // ignore storage failures
      }
    }

    patch((state) => ({
      ...state,
      query: '',
      statusFilter: 'all',
      kindFilter: 'all',
      logOpen: true,
      utilityTab: 'backups',
      utilityOpen: false
    }));

    pushLog('UI preferences reset.');
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

    persistUiPreferences(current());
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

  function selectBackup(path: string) {
    patch((state) => ({ ...state, selectedBackupPath: path }));
  }

  async function copySelectedBackupPath() {
    const path = current().selectedBackupPath;
    if (!path) return;

    try {
      await navigator.clipboard.writeText(path);
      pushLog('Backup path copied.');
    } catch (error) {
      pushLog(`Copy backup path failed: ${String(error)}`);
    }
  }

  async function copySelectedBackupOriginalPath() {
    const state = current();
    const selected = state.backups.find((entry) => entry.path === state.selectedBackupPath);
    const path = selected?.originalPath;

    if (!path) return;

    try {
      await navigator.clipboard.writeText(path);
      pushLog('Original path copied.');
    } catch (error) {
      pushLog(`Copy original path failed: ${String(error)}`);
    }
  }

  async function restoreBackupFromSelection() {
    const state = current();
    const backup = state.backups.find((entry) => entry.path === state.selectedBackupPath);

    if (!backup || !backup.restoreAvailable) return;

    const target = backup.originalPath ?? 'Unknown target';
    const confirmed = window.confirm(
      `Restore backup "${backup.name}" to\n\n${target}\n\nA safety backup of the current target will be created automatically when possible.`
    );

    if (!confirmed) {
      pushLog('Restore backup canceled.');
      return;
    }

    patch((state) => ({ ...state, backupsRestoreBusy: true }));

    try {
      const result = await invoke<BackupRestoreResult>('restore_backup', {
        backupPath: backup.path
      });
      pushLog(result.message);
      await refreshBackups(true);
      await refreshEntries(result.restoredPath ?? null, result.restoredPath ?? null);
    } catch (error) {
      pushLog(`Restore backup failed: ${String(error)}`);
    } finally {
      patch((state) => ({ ...state, backupsRestoreBusy: false }));
    }
  }

  function pruneIconCachesForEntries(
    nextEntries: LauncherEntry[],
    state: LauncherControllerState
  ): Pick<LauncherControllerState, 'itemIconUrls' | 'itemIconLoading' | 'previewDataUrls'> {
    const liveEntryPaths = new Set(nextEntries.map((entry) => entry.path));
    const livePreviewPaths = new Set(
      nextEntries
        .map((entry) => entry.resolvedIconPath)
        .filter((value): value is string => !!value)
    );

    const itemIconUrls: Record<string, string> = {};
    for (const [path, value] of Object.entries(state.itemIconUrls)) {
      if (liveEntryPaths.has(path)) {
        itemIconUrls[path] = value;
      }
    }

    const itemIconLoading: Record<string, boolean> = {};
    for (const [path, value] of Object.entries(state.itemIconLoading)) {
      if (liveEntryPaths.has(path)) {
        itemIconLoading[path] = value;
      }
    }

    const previewDataUrls: Record<string, string> = {};
    for (const [path, value] of Object.entries(state.previewDataUrls)) {
      if (livePreviewPaths.has(path)) {
        previewDataUrls[path] = value;
      }
    }

    return {
      itemIconUrls,
      itemIconLoading,
      previewDataUrls
    };
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
      ...pruneIconCachesForEntries(nextEntries, state),
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

  async function focusSearchInput(selectText = false) {
    await tick();
    searchInput?.focus();
    if (selectText) {
      searchInput?.select();
    }
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

  async function loadPreviewDataUrl(path: string): Promise<string | null> {
    const cached = current().previewDataUrls[path];
    if (cached) {
      return cached;
    }

    const existing = inflightPreviewLoads.get(path);
    if (existing) {
      return existing;
    }

    const promise = invoke<string | null>('load_icon_preview', { path })
      .then((result) => result ?? null)
      .finally(() => {
        inflightPreviewLoads.delete(path);
      });

    inflightPreviewLoads.set(path, promise);
    return promise;
  }

  async function runWithConcurrency<T>(
    items: T[],
    limit: number,
    worker: (item: T) => Promise<void>
  ) {
    if (items.length === 0) return;

    let index = 0;

    const runners = Array.from({ length: Math.min(limit, items.length) }, async () => {
      while (index < items.length) {
        const currentIndex = index++;
        await worker(items[currentIndex]);
      }
    });

    await Promise.allSettled(runners);
  }

  async function preloadListIcons(entriesToLoad: LauncherEntry[]) {
    const state = current();

    const wanted = entriesToLoad
      .filter((entry) => !!entry.resolvedIconPath)
      .filter((entry) => !state.itemIconUrls[entry.path])
      .filter((entry) => !state.itemIconLoading[entry.path])
      .slice(0, MAX_PRELOADED_LIST_ICONS);

    if (wanted.length === 0) return;

    const loadingPaths = Object.fromEntries(wanted.map((entry) => [entry.path, true] as const));

    patch((prev) => ({
      ...prev,
      itemIconLoading: { ...prev.itemIconLoading, ...loadingPaths }
    }));

    const loadedItemIcons: Record<string, string> = {};
    const loadedPreviews: Record<string, string> = {};

    await runWithConcurrency(wanted, ICON_PRELOAD_CONCURRENCY, async (entry) => {
      const resolvedPath = entry.resolvedIconPath;
      if (!resolvedPath) return;

      const result = await loadPreviewDataUrl(resolvedPath);
      if (!result) return;

      loadedItemIcons[entry.path] = result;
      loadedPreviews[resolvedPath] = result;
    });

    patch((prev) => {
      const nextLoading = { ...prev.itemIconLoading };
      for (const entry of wanted) {
        delete nextLoading[entry.path];
      }

      return {
        ...prev,
        itemIconLoading: nextLoading,
        itemIconUrls: { ...prev.itemIconUrls, ...loadedItemIcons },
        previewDataUrls: { ...prev.previewDataUrls, ...loadedPreviews }
      };
    });
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
      const result = await loadPreviewDataUrl(currentPath);
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

  async function refreshBackups(silent = false) {
    patch((state) => ({ ...state, backupsBusy: true }));

    try {
      const result = await invoke<BackupEntry[]>('list_backups');
      patch((state) => ({ ...state, backups: result }));

      if (!silent) {
        pushLog('Backups refreshed.');
      }
    } catch (error) {
      pushLog(`Backups failed: ${String(error)}`);
    } finally {
      patch((state) => ({ ...state, backupsBusy: false }));
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
    await refreshBackups(true);
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

  function openUtilityTab(tab: UtilityTab) {
    patch((state) => ({
      ...state,
      utilityOpen: true,
      utilityTab: tab
    }));
  }

  function closeUtility() {
    patch((state) => ({
      ...state,
      utilityOpen: false
    }));
  }

  function toggleUtilityOpen() {
    patch((state) => ({
      ...state,
      utilityOpen: !state.utilityOpen
    }));
  }

  function toggleDiagnosticsOpen() {
    patch((state) => ({
      ...state,
      utilityOpen: !(state.utilityOpen && state.utilityTab === 'diagnostics'),
      utilityTab: 'diagnostics'
    }));
  }

  function toggleMaintenanceOpen() {
    patch((state) => ({
      ...state,
      utilityOpen: !(state.utilityOpen && state.utilityTab === 'maintenance'),
      utilityTab: 'maintenance'
    }));
  }

  function toggleBackupsOpen() {
    patch((state) => ({
      ...state,
      utilityOpen: !(state.utilityOpen && state.utilityTab === 'backups'),
      utilityTab: 'backups'
    }));
  }

  function setIconLoadFailed(value: boolean) {
    patch((state) => ({ ...state, iconLoadFailed: value }));
  }

  function bindSearchInput(node: HTMLInputElement | null) {
    searchInput = node;
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

  async function runPrimaryActionForSelection() {
    const state = current();
    const entry = state.selected;
    if (!entry) return;

    if (canRunEntryAction('fix', entry)) {
      await runEntryAction('fix');
      return;
    }

    await runEntryAction('check');
  }

  async function runSecondaryActionForSelection() {
    const state = current();
    const entry = state.selected;
    if (!entry) return;

    if (canRunEntryAction('restore', entry)) {
      await runEntryAction('restore');
    }
  }

  function handleGlobalKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      if (current().utilityOpen) {
        closeUtility();
        return;
      }

      closeContextMenu();
      return;
    }

    const ctrlOrMeta = event.ctrlKey || event.metaKey;

    if (!ctrlOrMeta && event.key === '/' && !shouldIgnoreKeyTarget(event.target)) {
      event.preventDefault();
      void focusSearchInput(true);
      return;
    }

    if (ctrlOrMeta && event.key.toLowerCase() === 'r' && !event.shiftKey) {
      event.preventDefault();
      void scan();
      return;
    }

    if (ctrlOrMeta && event.key.toLowerCase() === 'l' && !event.shiftKey) {
      event.preventDefault();
      toggleLogOpen();
      return;
    }

    if (ctrlOrMeta && event.key.toLowerCase() === 'd' && !event.shiftKey) {
      event.preventDefault();
      toggleDiagnosticsOpen();
      return;
    }

    if (ctrlOrMeta && event.key.toLowerCase() === 'm' && !event.shiftKey) {
      event.preventDefault();
      toggleMaintenanceOpen();
      return;
    }

    if (ctrlOrMeta && event.key.toLowerCase() === 'b' && !event.shiftKey) {
      event.preventDefault();
      toggleBackupsOpen();
      return;
    }

    if (ctrlOrMeta && event.shiftKey && event.key.toLowerCase() === 'r') {
      event.preventDefault();
      resetUiPreferences();
      return;
    }

    const state = current();
    if (shouldIgnoreKeyTarget(event.target)) return;
    if (state.filteredEntries.length === 0) return;

    const currentIndex = state.selected
      ? state.filteredEntries.findIndex((entry) => entry.path === state.selected?.path)
      : -1;

    if (event.key === 'Enter') {
      event.preventDefault();
      void runPrimaryActionForSelection();
      return;
    }

    if (event.key === 'Delete') {
      event.preventDefault();
      void runSecondaryActionForSelection();
      return;
    }

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
    applyStoredUiPreferences();

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
        await refreshBackups(true);
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

        if (current().backups.length > 0) {
          pushLog(`Backups: ${current().backups.length} backup item(s) available.`);
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
    selectBackup,
    copySelectedBackupPath,
    copySelectedBackupOriginalPath,
    restoreBackupFromSelection,
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
    refreshDiagnostics,
    refreshMaintenance,
    refreshBackups,
    runGeneratedCleanup,
    setIconLoadFailed,
    resetUiPreferences,
    bindSearchInput,
    openUtilityTab,
    closeUtility,
    toggleUtilityOpen,
    toggleDiagnosticsOpen,
    toggleMaintenanceOpen,
    toggleBackupsOpen
  };
}
