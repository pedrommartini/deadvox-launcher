# Verification

Native tests cover Steam configuration parsing, reversible game configuration
changes, legacy configuration migration, file ownership, package integrity and
voice selection. `scripts/build.ps1` runs these before building the executable.

Alpha.5 additionally tests independent character mounts, exact restoration of
Original, corrupt/missing character rejection, metadata-commit rollback, retention
of old versioned packages, interrupted multi-pack downloads and reuse of completed
files on retry. A synthetic 100-character catalog totals 20 GB while every
individual asset remains below 2 GiB. Read-only status hashes are cached by file
size/modification time; download and installation verification remains uncached.

DeadVox checks also cover released character coverage without a local VPK, using
temporary game and pack fixtures. Editorial availability remains distinct from
installation and activation. Browser verification covers the original/PT-BR
rosters, search, status filters, hero details, About/Settings/Updates dialogs and
the minimum supported 800 × 620 layout. It does not switch actual game voices.

Update tests cover fixed embedded source and rejection of obsolete overrides,
removed source-changing actions, semantic version ordering, repository/HTTPS origin checks,
manifest parsing over a local test transport, partial/corrupt download recovery
and the Windows replacement helper against temporary fixture files. Operation
tests cover concurrent-command rejection and an actual attempt to execute a
fixture image while the exclusive application lock is held. No real game is
launched by those tests. Metadata-write failures also restore the previous voice
selection and pack.

The new-version dialog and progress-button rendering can be checked with
simulated UI state in a read-only browser preview. This checks layout and button
states, not a real GitHub release. A configured repository with published test
channels is required to validate a complete live update/restart flow.

`scripts/verify-portable.ps1` tests a copy of the finished application under the
build cache, away from the development source. With Deadlock closed, it checks
status, Original selection, PT-BR selection and original configuration recovery.
It leaves PT-BR enabled. It does not launch the game.

`scripts/audit-publication.ps1` examines the private source Git candidate list and local
portable folder against separate allowlists. Portable content accepts only the
executable, user README and finished optional VPK. Existing `user-data` is listed
separately as local runtime data, never distribution content; remove it from any
future externally distributed copy. Saved settings, caches and local packs are
ignored by Git. The report is written to `builds/publication-audit.json`.
This audit does not publish anything, and the build does not create a ZIP.

Launch the standalone window and test voice playback in Deadlock after release
changes. Automated file checks do not establish that every voice line has been
listened to in-game.
