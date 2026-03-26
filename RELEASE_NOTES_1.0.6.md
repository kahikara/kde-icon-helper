# KDE Icon Helper v1.0.6

KDE Icon Helper 1.0.6 is a focused polish and workflow release.

This version keeps the app firmly KDE and Plasma focused, improves the main inspection flow, and adds better guidance when multiple icon candidates are available.

## Highlights

### Cleaner main workflow

- calmer and more consistent main window
- compact one line toolbar
- less repeated status and less visual noise
- inspector refined to keep the current icon as the main anchor

### Better icon selection

- additional icon variants directly in the inspector
- source labels for detected icon candidates
- recommended candidate ranking
- direct apply from the variant browser
- source aware matching for:
  - native KDE launchers
  - Flatpak
  - Steam
  - Wine
  - AppImage

### Improved utility workbench

- backup browser stays available in a quieter dedicated workspace
- maintenance and diagnostics visually aligned with the rest of the app
- bulk fix workflow for visible issues
- persistent UI preferences and utility shortcuts remain part of the workflow

### Restore and maintenance improvements

- backup restore with metadata support
- safety backup before restore when possible
- generated asset cleanup and dry run support
- better foundation for larger maintenance workflows

### Packaging

This release is shipped as Linux bundles:

- deb
- rpm
- AppImage

For the AppImage release, a helper launcher script is included as well:

- `run-kde-icon-helper-appimage.sh`

This wrapper starts the AppImage with a safer compatibility setup for systems that hit WebKitGTK or EGL startup issues on Wayland.

## Product direction

KDE Icon Helper remains intentionally KDE focused.

The goal is not to become a generic launcher fixer for every desktop environment. The focus stays on Plasma friendly launcher inspection, icon repair, restore workflows, and compact utility tooling.

## Notes

This release is mainly about making the existing workflow feel more solid, more predictable, and more helpful without bloating the interface.
