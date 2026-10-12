# Changelog

All notable changes to Lastre are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [0.1.1] - 2026-10-12

### Fixed

- Items larger than the Recycle Bin's capacity were deleted permanently instead
  of being recycled. Lastre now refuses to send anything that Windows would not
  keep in the Recycle Bin: items that do not fit, items that would push older
  items out of the bin, and items on drives without a Recycle Bin.
- The used/free text overflowed the drive card on the start screen.

### Added

- System folders and files (Windows, Program Files, ProgramData, user profiles,
  drive roots, `pagefile.sys` and similar) can no longer be added to the basket.
- A warning in the confirmation dialog for items inside program or app-data folders.

## [0.1.0] - 2026-10-12

### Added

- Parallel scanner for a drive or folder, with progress and cancellation.
- Summary, Folders, File types and Large files views, always sorted by size.
- Cleanup basket that sends everything to the Recycle Bin after confirmation.
- Settings with Spanish and English and four color palettes.

[0.1.1]: https://github.com/adridesweb/lastre/releases/tag/v0.1.1
[0.1.0]: https://github.com/adridesweb/lastre/releases/tag/v0.1.0
