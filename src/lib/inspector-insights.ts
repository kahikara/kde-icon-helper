import { kindOf, type ContextAction } from '$lib/launcher-ui';
import type { LauncherEntry } from '$lib/types';

export interface InspectorInsight {
  issueTitle: string;
  issueSummary: string;
  recommendationTitle: string;
  recommendationReason: string;
  recommendedAction: ContextAction | null;
  recommendedActionLabel: string;
  iconSourceLabel: string;
  iconSourceDetail: string;
  launcherKindLabel: string;
  launcherKindDetail: string;
  targetStateLabel: string;
  targetStateDetail: string;
  currentValueLabel: string;
  currentValueDetail: string;
}

function isIconPathValue(icon: string | null): boolean {
  if (!icon) return false;
  return (
    icon.includes('/') ||
    icon.includes('\\') ||
    /\.(png|svg|xpm|ico)$/i.test(icon)
  );
}

function basename(path: string | null): string {
  if (!path) return 'None';
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

function iconSource(entry: LauncherEntry): Pick<
  InspectorInsight,
  'iconSourceLabel' | 'iconSourceDetail' | 'currentValueLabel' | 'currentValueDetail'
> {
  if (!entry.icon && !entry.resolvedIconPath) {
    return {
      iconSourceLabel: 'No icon value',
      iconSourceDetail: 'This launcher currently has no usable icon source to resolve.',
      currentValueLabel: 'Stored icon value',
      currentValueDetail: 'None'
    };
  }

  if (entry.icon && entry.resolvedIconPath) {
    if (isIconPathValue(entry.icon)) {
      return {
        iconSourceLabel: 'Path based icon',
        iconSourceDetail: 'The launcher stores an explicit icon file path and it currently resolves.',
        currentValueLabel: 'Stored icon value',
        currentValueDetail: entry.icon
      };
    }

    return {
      iconSourceLabel: 'Theme icon',
      iconSourceDetail: 'The launcher stores a theme icon name and Plasma resolved it to a file.',
      currentValueLabel: 'Stored icon value',
      currentValueDetail: entry.icon
    };
  }

  if (entry.icon && !entry.resolvedIconPath) {
    if (isIconPathValue(entry.icon)) {
      return {
        iconSourceLabel: 'Broken path reference',
        iconSourceDetail: 'The launcher points at an icon file path that no longer resolves cleanly.',
        currentValueLabel: 'Stored icon value',
        currentValueDetail: entry.icon
      };
    }

    return {
      iconSourceLabel: 'Theme name only',
      iconSourceDetail: 'The launcher has an icon name, but no preview file could be resolved yet.',
      currentValueLabel: 'Stored icon value',
      currentValueDetail: entry.icon
    };
  }

  return {
    iconSourceLabel: 'Resolved icon',
    iconSourceDetail: 'A resolved icon file is available.',
    currentValueLabel: 'Stored icon value',
    currentValueDetail: entry.icon ?? 'None'
  };
}

function launcherKind(entry: LauncherEntry): Pick<
  InspectorInsight,
  'launcherKindLabel' | 'launcherKindDetail'
> {
  const kind = kindOf(entry);

  if (kind === 'exe_link') {
    return {
      launcherKindLabel: 'Direct EXE link',
      launcherKindDetail: 'This is a more fragile workflow for Plasma and often worth normalizing.'
    };
  }

  return {
    launcherKindLabel: 'KDE desktop launcher',
    launcherKindDetail: 'Standard desktop entry flow with better desktop integration.'
  };
}

function targetState(entry: LauncherEntry): Pick<
  InspectorInsight,
  'targetStateLabel' | 'targetStateDetail'
> {
  if (entry.targetPath) {
    return {
      targetStateLabel: 'Target found',
      targetStateDetail: basename(entry.targetPath)
    };
  }

  return {
    targetStateLabel: 'Target missing',
    targetStateDetail: 'No executable target could be resolved right now.'
  };
}

function issueMapping(entry: LauncherEntry): Pick<
  InspectorInsight,
  'issueTitle' | 'issueSummary' | 'recommendationTitle' | 'recommendationReason' | 'recommendedAction' | 'recommendedActionLabel'
> {
  const message = entry.message?.trim();

  switch (entry.status) {
    case 'ok':
      return {
        issueTitle: 'Healthy KDE launcher',
        issueSummary:
          message || 'The launcher and its current icon resolution look healthy.',
        recommendationTitle: 'No immediate action needed',
        recommendationReason:
          'This item already looks stable. Keep it as is unless you want to change the icon manually.',
        recommendedAction: null,
        recommendedActionLabel: ''
      };

    case 'missing_icon':
      return {
        issueTitle: 'No icon is defined',
        issueSummary:
          message || 'The launcher does not currently provide an icon value for Plasma to resolve.',
        recommendationTitle: 'Set a manual icon',
        recommendationReason:
          'For a launcher without any icon value, a manual icon is the safest direct KDE friendly fix.',
        recommendedAction: 'manual',
        recommendedActionLabel: 'Set icon manually'
      };

    case 'broken_icon_path':
      return {
        issueTitle: 'Stored icon path is broken',
        issueSummary:
          message || 'The current icon path no longer points to a usable icon file.',
        recommendationTitle: 'Run Fix',
        recommendationReason:
          'The automatic fix can replace a broken file path with a more stable local icon reference.',
        recommendedAction: 'fix',
        recommendedActionLabel: 'Run Fix'
      };

    case 'exe_detected_needs_fixed_icon':
      return {
        issueTitle: 'Needs KDE friendly icon handling',
        issueSummary:
          message ||
          'This launcher behaves like an executable driven entry and usually benefits from a normalized icon setup.',
        recommendationTitle: 'Run Fix',
        recommendationReason:
          'The fixer can generate or normalize the icon setup into something Plasma handles more reliably.',
        recommendedAction: 'fix',
        recommendedActionLabel: 'Run Fix'
      };

    case 'direct_exe_link':
      return {
        issueTitle: 'Direct executable link detected',
        issueSummary:
          message ||
          'This item points directly at an executable instead of behaving like a cleaner desktop launcher.',
        recommendationTitle: 'Run Fix',
        recommendationReason:
          'The fix flow can create a calmer KDE friendly icon and launcher setup for direct executable links.',
        recommendedAction: 'fix',
        recommendedActionLabel: 'Run Fix'
      };

    case 'missing_exec_target':
      return {
        issueTitle: 'Launch target is missing',
        issueSummary:
          message || 'The icon may not be the core issue here because the executable target itself is missing.',
        recommendationTitle: 'Review launcher target first',
        recommendationReason:
          'Restore or correct the launch target before spending time on icon polish. Icon changes alone will not solve this state.',
        recommendedAction: null,
        recommendedActionLabel: ''
      };

    case 'invalid_desktop_file':
      return {
        issueTitle: 'Desktop file needs review',
        issueSummary:
          message || 'The launcher file structure itself looks invalid or incomplete.',
        recommendationTitle: 'Review the desktop file',
        recommendationReason:
          'This is primarily a launcher definition problem. Clean up the desktop entry before focusing on icon styling.',
        recommendedAction: null,
        recommendedActionLabel: ''
      };

    case 'unsupported_exec':
      return {
        issueTitle: 'Unsupported launcher target',
        issueSummary:
          message || 'This launch target is outside the currently supported fix flow.',
        recommendationTitle: 'Handle this one manually',
        recommendationReason:
          'The safest route here is manual review, because the current automated KDE focused workflows do not fully support this target.',
        recommendedAction: null,
        recommendedActionLabel: ''
      };

    default:
      return {
        issueTitle: 'Needs review',
        issueSummary: message || 'This launcher needs a closer look.',
        recommendationTitle: 'Check the launcher',
        recommendationReason:
          'Start with verification before changing anything.',
        recommendedAction: 'check',
        recommendedActionLabel: 'Check'
      };
  }
}

export function deriveInspectorInsight(entry: LauncherEntry | null): InspectorInsight {
  if (!entry) {
    return {
      issueTitle: 'No item selected',
      issueSummary: 'Select a launcher to inspect its current state.',
      recommendationTitle: 'Choose a launcher',
      recommendationReason: 'The detail area becomes useful once an item is selected.',
      recommendedAction: null,
      recommendedActionLabel: '',
      iconSourceLabel: 'No data',
      iconSourceDetail: 'No icon source available.',
      launcherKindLabel: 'No data',
      launcherKindDetail: 'No launcher selected.',
      targetStateLabel: 'No data',
      targetStateDetail: 'No target available.',
      currentValueLabel: 'Stored icon value',
      currentValueDetail: 'None'
    };
  }

  return {
    ...issueMapping(entry),
    ...iconSource(entry),
    ...launcherKind(entry),
    ...targetState(entry)
  };
}
