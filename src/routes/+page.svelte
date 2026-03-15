<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
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

  let iconLoadFailed = false;
  let selectedPreviewUrl: string | null = null;
  let selectedPreviewFor = '';
  let lastSelectedPath = '';

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

  function shouldIgnoreKeyTarget(target: EventTarget | null): boolean {
    const el = target as HTMLElement | null;
    if (!el) return false;
    const tag = el.tagName?.toLowerCase() ?? '';
    return tag === 'input' || tag === 'textarea' || tag === 'select' || el.isContentEditable;
  }

  function handleGlobalKeydown(event: KeyboardEvent) {
    if (shouldIgnoreKeyTarget(event.target)) return;
    if (filteredEntries.length === 0) return;

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      void selectRelative(1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      void selectRelative(-1);
    }
  }

  async function scan() {
    busy = true;
    try {
      const result = await invoke<LauncherEntry[]>('scan_launchers');
      entries = result;
      restoreSelection(result);
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
      pushLog(`Checked ${updated.name}. Status is now ${statusText(updated.status)}.`);
    } catch (error) {
      pushLog(`Check failed: ${String(error)}`);
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
          extensions: ['png', 'svg', 'xpm']
        }
      ]
    });

    if (!chosen || Array.isArray(chosen)) return;

    busy = true;
    try {
      const previousPath = selected.path;
      const result = await invoke<FixResult>('set_launcher_icon_manual', {
        path: previousPath,
        source_icon_path: chosen
      });
      pushLog(result.message);

      const refreshed = await invoke<LauncherEntry[]>('scan_launchers');
      entries = refreshed;
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

  async function fixSelected() {
    if (!selected) return;
    busy = true;
    try {
      const previousPath = selected.path;
      const result = await invoke<FixResult>('fix_launcher_icon', { path: previousPath });
      pushLog(result.message);

      const refreshed = await invoke<LauncherEntry[]>('scan_launchers');
      entries = refreshed;
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
    void scan();
    window.addEventListener('keydown', handleGlobalKeydown);

    return () => {
      window.removeEventListener('keydown', handleGlobalKeydown);
    };
  });
</script>

<svelte:head>
  <title>KDE Icon Helper</title>
</svelte:head>

<div class="app">
  <header class="topbar">
    <div class="brand">KDE Icon Helper</div>

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
          <button
            type="button"
            data-item-path={entry.path}
            class:selected={selected?.path === entry.path}
            class="itemCard"
            on:click={() => (selected = entry)}
          >
            <div class="itemIcon">
              <span>{entry.status === 'direct_exe_link' ? 'EXE' : 'APP'}</span>
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

    <aside class="panel actionPanel">
      <div class="panelHeader">
        <div class="panelTitle">Action panel</div>
      </div>

      <div class="actionScroll">
        <div class="sectionBlock">
          <div class="sectionTitle">Primary actions</div>
          <div class="buttonStack">
            <button type="button" on:click={checkSelected} disabled={busy || !selected}>Check selected</button>
            <button class="primary" type="button" on:click={fixSelected} disabled={busy || !selected}>Fix selected</button>
            <button type="button" on:click={setManualIcon} disabled={busy || !selected}>Set icon manually</button>
            <button type="button" disabled>Restore</button>
          </div>
        </div>

        <div class="sectionBlock">
          <div class="sectionTitle">Next step</div>
          <div class="value">{adviceFor(selected)}</div>
        </div>

        <div class="sectionBlock">
          <div class="sectionTitle">Repair mode</div>
          <div class="value">{repairMode(selected)}</div>
        </div>

        <div class="sectionBlock">
          <div class="sectionTitle">Backup</div>
          <div class="value code">{selected?.backupPath ?? 'None yet'}</div>
        </div>

        <div class="sectionBlock">
          <div class="sectionTitle">Last result</div>
          <div class="value">{resultText(selected)}</div>
        </div>
      </div>
    </aside>
  </main>

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

<style>
  :global(html) {
    width: 100%;
    height: 100%;
    margin: 0;
    padding: 0;
    background: #0c1015 !important;
    overflow: hidden;
    scrollbar-width: thin;
    scrollbar-color: #394555 #0c1015;
  }

  :global(body) {
    width: 100%;
    height: 100%;
    margin: 0;
    padding: 0;
    background: #0c1015 !important;
    color: #e6ebf2;
    font-family: Inter, ui-sans-serif, system-ui, sans-serif;
    font-size: 14px;
    overflow: hidden;
    scrollbar-width: thin;
    scrollbar-color: #394555 #0c1015;
  }

  :global(#svelte),
  :global(#sveltekit),
  :global(#app),
  :global(body > div) {
    width: 100%;
    height: 100%;
    margin: 0;
    padding: 0;
    background: #0c1015 !important;
    overflow: hidden;
  }

  :global(*) {
    box-sizing: border-box;
  }

  :global(*)::-webkit-scrollbar:horizontal {
    height: 0 !important;
    display: none !important;
  }

  :global(html::-webkit-scrollbar),
  :global(body::-webkit-scrollbar) {
    width: 9px;
    height: 9px;
    background: #0c1015;
  }

  :global(html::-webkit-scrollbar-track),
  :global(body::-webkit-scrollbar-track),
  :global(html::-webkit-scrollbar-corner),
  :global(body::-webkit-scrollbar-corner) {
    background: #0c1015;
  }

  :global(html::-webkit-scrollbar-thumb),
  :global(body::-webkit-scrollbar-thumb) {
    background: #394555;
    border-radius: 999px;
    border: 2px solid #0c1015;
  }

  .app {
    --bg: #0c1015;
    --panel: #121821;
    --panelSoft: #10161e;
    --panelRow: #0f141b;
    --border: #222b37;
    --borderStrong: #2a3442;

    --text: #e6ebf2;
    --textSoft: #c9d1dc;
    --textMuted: #97a3b2;

    --inputBg: #0f151d;
    --buttonBg: #151d27;
    --buttonHover: #1b2532;
    --buttonPrimary: #243246;
    --buttonPrimaryHover: #2a3a50;

    --goodBg: rgba(88, 120, 104, 0.16);
    --goodText: #c6d7cc;
    --goodBorder: rgba(88, 120, 104, 0.28);

    --warnBg: rgba(126, 110, 82, 0.16);
    --warnText: #d9ccb1;
    --warnBorder: rgba(126, 110, 82, 0.28);

    --dangerBg: rgba(128, 90, 90, 0.16);
    --dangerText: #dfc0c0;
    --dangerBorder: rgba(128, 90, 90, 0.28);

    --accentBg: rgba(97, 122, 166, 0.16);
    --accentText: #c8d4e8;
    --accentBorder: rgba(97, 122, 166, 0.30);

    --mutedBg: rgba(124, 136, 154, 0.14);
    --mutedText: #c7cfdb;
    --mutedBorder: rgba(124, 136, 154, 0.24);

    --radius: 12px;

    width: 100%;
    height: 100%;
    margin: 0;
    padding: 8px;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    gap: 8px;
    background: var(--bg);
    color: var(--text);
    overflow: hidden;
  }

  .topbar,
  .panel {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .topbar {
    padding: 8px 10px;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 10px;
    align-items: center;
    min-width: 0;
  }

  .brand {
    font-size: 0.98rem;
    font-weight: 700;
    line-height: 1.2;
    color: var(--text);
    white-space: nowrap;
  }

  .toolbar {
    display: grid;
    grid-template-columns: 240px 140px 130px max-content max-content;
    gap: 8px;
    align-items: center;
    justify-content: start;
    min-width: 0;
    overflow: hidden;
  }

  .searchWrap,
  .selectWrap {
    min-width: 0;
    max-width: 100%;
  }

  .searchWrap {
    width: 240px;
    max-width: 100%;
    justify-self: start;
  }

  .selectWrap {
    position: relative;
  }

  .selectWrap::after {
    content: '▾';
    position: absolute;
    right: 11px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--textMuted);
    font-size: 0.72rem;
    pointer-events: none;
  }

  .workspace {
    display: grid;
    grid-template-columns: minmax(340px, 1.5fr) minmax(300px, 0.9fr) minmax(220px, 0.72fr);
    gap: 8px;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .panelHeader {
    padding: 9px 10px;
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
  }

  .panelTitle {
    font-size: 0.9rem;
    font-weight: 700;
    line-height: 1.2;
    color: var(--text);
  }

  input,
  select,
  button {
    font: inherit;
  }

  input,
  select {
    width: 100%;
    min-height: 36px;
    padding: 7px 10px;
    border-radius: 9px;
    border: 1px solid var(--borderStrong);
    background: var(--inputBg);
    color: var(--text);
    font-size: 0.83rem;
    outline: none;
    min-width: 0;
  }

  select {
    appearance: none;
    -webkit-appearance: none;
    -moz-appearance: none;
    padding-right: 32px;
    cursor: pointer;
  }

  select::-ms-expand {
    display: none;
  }

  option {
    background: #161d27;
    color: var(--text);
  }

  input::placeholder {
    color: var(--textMuted);
  }

  input:focus,
  select:focus {
    border-color: #3b4b60;
    box-shadow: inset 0 0 0 1px rgba(97, 122, 166, 0.18);
  }

  button {
    min-height: 36px;
    padding: 7px 10px;
    border-radius: 9px;
    border: 1px solid var(--borderStrong);
    background: var(--buttonBg);
    color: var(--text);
    font-size: 0.81rem;
    cursor: pointer;
    min-width: 0;
  }

  button:hover:enabled {
    background: var(--buttonHover);
  }

  button:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .primary {
    background: var(--buttonPrimary);
    color: #eef3fa;
  }

  .primary:hover:enabled {
    background: var(--buttonPrimaryHover);
  }

  .ghost {
    background: transparent;
  }

  .pill {
    height: 36px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0 10px;
    border-radius: 999px;
    border: 1px solid var(--borderStrong);
    background: var(--inputBg);
    color: var(--textSoft);
    font-size: 0.76rem;
    white-space: nowrap;
  }

  .listPanel,
  .inspectorPanel,
  .actionPanel {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .listScroll,
  .inspectorScroll,
  .actionScroll,
  .logScroll {
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden !important;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: #394555 #0c1015;
  }

  .listScroll::-webkit-scrollbar,
  .inspectorScroll::-webkit-scrollbar,
  .actionScroll::-webkit-scrollbar,
  .logScroll::-webkit-scrollbar {
    width: 9px;
    height: 9px;
  }

  .listScroll::-webkit-scrollbar-track,
  .inspectorScroll::-webkit-scrollbar-track,
  .actionScroll::-webkit-scrollbar-track,
  .logScroll::-webkit-scrollbar-track,
  .listScroll::-webkit-scrollbar-corner,
  .inspectorScroll::-webkit-scrollbar-corner,
  .actionScroll::-webkit-scrollbar-corner,
  .logScroll::-webkit-scrollbar-corner {
    background: #0c1015;
  }

  .listScroll::-webkit-scrollbar-thumb,
  .inspectorScroll::-webkit-scrollbar-thumb,
  .actionScroll::-webkit-scrollbar-thumb,
  .logScroll::-webkit-scrollbar-thumb {
    background: #394555;
    border-radius: 999px;
    border: 2px solid #0c1015;
  }

  .listScroll::-webkit-scrollbar-thumb:hover,
  .inspectorScroll::-webkit-scrollbar-thumb:hover,
  .actionScroll::-webkit-scrollbar-thumb:hover,
  .logScroll::-webkit-scrollbar-thumb:hover {
    background: #4a5870;
  }

  .listScroll {
    padding: 12px;
    display: grid;
    gap: 10px;
    align-content: start;
  }

  .itemCard {
    width: 100%;
    min-width: 0;
    min-height: 52px;
    padding: 0 12px;
    border-radius: 11px;
    border: 1px solid var(--borderStrong);
    background: linear-gradient(180deg, rgba(255,255,255,0.015), rgba(255,255,255,0.00)), var(--panelRow);
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr) max-content;
    gap: 12px;
    align-items: center;
    overflow: hidden;
    text-align: left;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.02);
  }

  .itemCard.selected {
    border-color: #4a607b;
    background: #131a23;
    box-shadow:
      inset 0 0 0 1px rgba(97, 122, 166, 0.22),
      0 0 0 1px rgba(97, 122, 166, 0.10);
  }

  .itemIcon {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    overflow: hidden;
    color: var(--textSoft);
    font-size: 0.52rem;
    font-weight: 700;
    letter-spacing: 0.04em;
  }

  .itemName {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.88rem;
    font-weight: 700;
    line-height: 1.2;
    color: var(--text);
  }

  .itemStatus {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    min-width: 0;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    white-space: nowrap;
    border-radius: 999px;
    font-weight: 700;
    padding: 3px 8px;
    font-size: 0.61rem;
    border: 1px solid transparent;
    flex: 0 0 auto;
  }

  .good {
    background: var(--goodBg);
    color: var(--goodText);
    border-color: var(--goodBorder);
  }

  .warn {
    background: var(--warnBg);
    color: var(--warnText);
    border-color: var(--warnBorder);
  }

  .danger {
    background: var(--dangerBg);
    color: var(--dangerText);
    border-color: var(--dangerBorder);
  }

  .accent {
    background: var(--accentBg);
    color: var(--accentText);
    border-color: var(--accentBorder);
  }

  .muted {
    background: var(--mutedBg);
    color: var(--mutedText);
    border-color: var(--mutedBorder);
  }

  .inspectorScroll,
  .actionScroll {
    padding: 12px;
    display: grid;
    gap: 14px;
    align-content: start;
  }

  .field,
  .sectionBlock {
    display: grid;
    gap: 6px;
  }

  .label,
  .sectionTitle {
    color: var(--textMuted);
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .value {
    color: var(--text);
    font-size: 0.83rem;
    line-height: 1.45;
  }

  .code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.77rem;
    color: var(--textSoft);
    word-break: break-word;
    overflow-wrap: anywhere;
  }

  .previewField {
    gap: 8px;
  }

  .preview {
    min-height: 230px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--panelSoft);
    display: grid;
    place-items: center;
    padding: 16px;
  }

  .preview img {
    max-width: 190px;
    max-height: 190px;
    object-fit: contain;
  }

  .fallback {
    max-width: 280px;
    display: grid;
    gap: 8px;
    justify-items: center;
    text-align: center;
    color: var(--textSoft);
    font-size: 0.82rem;
  }

  .fallbackGlyph {
    width: 62px;
    height: 62px;
    border-radius: 14px;
    border: 1px solid var(--borderStrong);
    background: var(--panelRow);
    display: grid;
    place-items: center;
    color: var(--textSoft);
    font-size: 1rem;
    font-weight: 800;
  }

  .facts {
    display: grid;
    grid-template-columns: 120px minmax(0, 1fr);
    gap: 10px 12px;
    min-width: 0;
  }

  .factKey {
    color: var(--textMuted);
    font-size: 0.76rem;
    font-weight: 700;
  }

  .factValue {
    color: var(--text);
    font-size: 0.83rem;
    line-height: 1.45;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .buttonStack {
    display: grid;
    gap: 8px;
  }

  .logPanel {
    display: grid;
    grid-template-rows: auto;
    min-height: 0;
    overflow: hidden;
  }

  .logHeader {
    align-items: center;
  }

  .logScroll {
    padding: 10px 12px 12px;
    height: 128px;
  }

  .logLine {
    padding: 7px 0;
    border-bottom: 1px solid #18202c;
    color: var(--textSoft);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.77rem;
  }

  .logLine:last-child {
    border-bottom: 0;
  }

  .empty {
    padding: 16px 14px;
    display: grid;
    gap: 4px;
    color: var(--textSoft);
    font-size: 0.82rem;
  }

  .empty.compact {
    padding: 0;
  }

  @media (max-width: 1280px) {
    :global(html),
    :global(body) {
      overflow: auto;
    }

    .app {
      height: auto;
      min-height: 100vh;
      grid-template-rows: auto auto auto;
      overflow: visible;
    }

    .workspace {
      grid-template-columns: 1fr;
    }

    .listPanel,
    .inspectorPanel,
    .actionPanel {
      grid-template-rows: auto auto;
    }

    .listScroll,
    .inspectorScroll,
    .actionScroll {
      overflow: visible;
    }
  }

  @media (max-width: 980px) {
    .topbar {
      grid-template-columns: 1fr;
    }

    .toolbar {
      grid-template-columns: 1fr;
      justify-content: stretch;
    }

    .searchWrap {
      width: 100%;
    }

    .facts {
      grid-template-columns: 1fr;
    }
  }
</style>
