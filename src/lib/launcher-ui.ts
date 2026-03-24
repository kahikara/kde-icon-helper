import type { LauncherEntry, LauncherStatus } from '$lib/types';

export type StatusFilter = 'all' | LauncherStatus;
export type KindFilter = 'all' | 'launcher' | 'exe_link';
export type ContextMenuMode = 'entry' | 'input';
export type ContextAction = 'check' | 'fix' | 'manual' | 'restore';
export type InputContextAction = 'cut' | 'copy' | 'paste' | 'selectAll';

export interface EntryActionItem {
  id: ContextAction;
  label: string;
  contextLabel: string;
  primary?: boolean;
}

export interface InputActionItem {
  id: InputContextAction;
  label: string;
}

export const statusLabel: Record<string, string> = {
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

export const statusTone: Record<string, string> = {
  ok: 'good',
  missing_icon: 'warn',
  broken_icon_path: 'danger',
  exe_detected_needs_fixed_icon: 'accent',
  missing_exec_target: 'danger',
  invalid_desktop_file: 'muted',
  unsupported_exec: 'muted',
  direct_exe_link: 'accent'
};

export const statusFilterOptions: Array<{ value: StatusFilter; label: string }> = [
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

export const kindFilterOptions: Array<{ value: KindFilter; label: string }> = [
  { value: 'all', label: 'All items' },
  { value: 'launcher', label: 'Launchers' },
  { value: 'exe_link', label: 'EXE links' }
];

export const entryActionItems: EntryActionItem[] = [
  { id: 'check', label: 'Check', contextLabel: 'Check selected' },
  { id: 'fix', label: 'Fix', contextLabel: 'Fix selected', primary: true },
  { id: 'manual', label: 'Manual', contextLabel: 'Set icon manually' },
  { id: 'restore', label: 'Restore', contextLabel: 'Restore default icon' }
];

export const inputActionItems: InputActionItem[] = [
  { id: 'cut', label: 'Cut' },
  { id: 'copy', label: 'Copy' },
  { id: 'paste', label: 'Paste' },
  { id: 'selectAll', label: 'Select all' }
];

export function statusText(status?: string | null) {
  return status ? statusLabel[status] ?? status : 'Unknown';
}

export function statusClass(status?: string | null) {
  const tone = status ? statusTone[status] ?? 'muted' : 'muted';
  return `badge ${tone}`;
}

export function rowGlyph(entry: LauncherEntry) {
  return entry.status === 'direct_exe_link' ? 'EXE' : 'APP';
}

export function previewFallbackGlyph(entry: LauncherEntry) {
  return entry.status === 'direct_exe_link' ? 'EXE' : '?';
}

export function isDesktopLauncher(entry: LauncherEntry | null) {
  return !!entry?.path && entry.path.toLowerCase().endsWith('.desktop');
}

export function canRunEntryAction(action: ContextAction, entry: LauncherEntry | null) {
  if (!entry) return false;

  if (action === 'check') {
    return true;
  }

  if (action === 'fix') {
    return (
      entry.status === 'direct_exe_link' ||
      entry.status === 'exe_detected_needs_fixed_icon' ||
      entry.status === 'broken_icon_path'
    );
  }

  if (action === 'manual') {
    return isDesktopLauncher(entry);
  }

  if (action === 'restore') {
    return isDesktopLauncher(entry) && entry.canRestoreDefaultIcon;
  }

  return false;
}

export function availableEntryActions(entry: LauncherEntry | null) {
  return entryActionItems.filter((action) => canRunEntryAction(action.id, entry));
}

export function kindOf(entry: LauncherEntry | null): 'launcher' | 'exe_link' | 'other' {
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
