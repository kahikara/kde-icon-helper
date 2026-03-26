# KDE Icon Helper

KDE Icon Helper is a KDE focused desktop utility for Linux that helps inspect, repair, and restore launcher icons.

It is built for Plasma style workflows and works best with `.desktop` launchers, especially mixed setups such as Wine, Lutris, Steam, Flatpak, and AppImage.

## Highlights

- scan desktop launchers and launcher directories
- inspect the selected launcher in a compact details view
- preview the current resolved icon
- detect missing, broken, or mismatched icon values
- fix launcher icons automatically when a strong match is available
- set a custom icon manually
- restore the original icon from backup
- browse additional icon variants and apply them directly
- source aware icon ranking for native KDE, Flatpak, Steam, Wine, and AppImage launchers
- utility workbench with backups, maintenance, and diagnostics
- bulk fix visible issues from the current filtered list
- keyboard shortcuts and persistent UI preferences

## Main workflow

1. Scan launchers
2. Filter the list
3. Inspect the selected launcher
4. Apply an automatic fix, choose another icon, or restore the original state
5. Use the utility workbench for backups, diagnostics, and maintenance

## Inspector

The inspector is designed to stay clean and focused.

It keeps the current icon as the main anchor, shows only the most relevant actions, and moves deeper technical context into a quieter expandable section.

Current capabilities include:

- current icon preview
- direct actions for fix, manual icon, and restore
- compact variant browser for other matching icons
- recommended candidate ranking
- technical details for launcher path, icon values, resolved icon, and source context

## Icon variants

When multiple usable icons are available, KDE Icon Helper can surface additional candidates directly in the inspector.

The current implementation focuses on keeping the UI compact while still making the next useful choice obvious.

It currently supports:

- current icon detection
- additional icon candidates
- source labels such as Hicolor, Breeze, Breeze Dark, Pixmaps, Manual, Generated, and Local
- recommended candidate ranking
- direct apply from the inspector

## Source aware matching

KDE Icon Helper stays KDE focused, but it understands common launcher sources and uses that information to improve icon suggestions.

Current launcher source handling includes:

- native KDE and standard desktop launchers
- Flatpak
- Steam
- Wine
- AppImage

This helps the app choose better lookup terms and produce more useful icon candidates.

## Utility workbench

The utility workbench is a separate calm workspace for tasks that should not disrupt the main launcher flow.

It currently includes:

- Backup Browser
- Maintenance
- Diagnostics

### Backup Browser

- browse backups with metadata
- inspect backup details in a split layout
- restore backups directly
- create a safety backup before restore when possible

### Maintenance

- generated asset stats
- orphan cleanup dry run
- orphan cleanup execution
- bulk fix for visible launcher issues

### Diagnostics

- runtime diagnostics
- dependency and tool visibility
- quick environment checks for troubleshooting

## Keyboard shortcuts and preferences

The app includes persistent UI preferences and several global shortcuts for the utility workbench.

Current shortcuts include:

- `Ctrl+B` for Backups
- `Ctrl+D` for Diagnostics
- `Ctrl+M` for Maintenance

## Scope

KDE Icon Helper is intentionally KDE focused.

It is not trying to become a general purpose launcher fixer for every Linux desktop. The product direction is Plasma first, clean, compact, and practical for real launcher repair workflows.

## Packages

Linux release bundles are built as:

- `deb`
- `rpm`
- `AppImage`

For AppImage users on some Wayland systems, the release can also include a small launcher wrapper script that starts the AppImage with a safer X11 and WebKitGTK fallback setup.

## Status

Version `1.0.6` focuses on polishing the main workflow, improving icon variant handling, adding source aware ranking, and keeping the overall UI calmer and more consistent.
