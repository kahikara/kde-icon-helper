export type LauncherStatus =
  | 'ok'
  | 'missing_icon'
  | 'broken_icon_path'
  | 'exe_detected_needs_fixed_icon'
  | 'missing_exec_target'
  | 'invalid_desktop_file'
  | 'unsupported_exec'
  | 'direct_exe_link';

export interface LauncherEntry {
  name: string;
  path: string;
  exec: string;
  icon: string | null;
  resolvedIconPath: string | null;
  status: LauncherStatus;
  targetPath: string | null;
  message: string | null;
  backupPath: string | null;
  canRestoreDefaultIcon: boolean;
}

export interface FixResult {
  ok: boolean;
  path: string;
  message: string;
  updatedEntry?: LauncherEntry;
}

export interface ToolDiagnostic {
  name: string;
  found: boolean;
  path: string | null;
  version: string | null;
  note: string | null;
  requiredFor: string[];
}

export interface RuntimeDiagnostics {
  desktopDir: string;
  desktopDirExists: boolean;
  tools: ToolDiagnostic[];
}

export interface GeneratedAssetStats {
  generatedIconsCount: number;
  generatedIconsBytes: number;
  manualIconsCount: number;
  manualIconsBytes: number;
  backupsCount: number;
  backupsBytes: number;
  orphanGeneratedIconsCount: number;
  orphanGeneratedIconsBytes: number;
  totalBytes: number;
}

export interface CleanupResult {
  dryRun: boolean;
  removedFilesCount: number;
  removedBytes: number;
  removedPaths: string[];
  statsBefore: GeneratedAssetStats;
  statsAfter: GeneratedAssetStats;
}
