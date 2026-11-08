# Verification

Native tests cover Steam configuration parsing, reversible game configuration
changes, legacy configuration migration, file ownership, package integrity and
voice selection. `scripts/build.ps1` runs these before building the executable.

Priority checks cover DMM addons before base-game paths, LF/CRLF preservation,
recovery after removed or demoted PT-BR mounts, migration of active older states,
Original preference persistence and rollback if saving that preference fails.
Recovery runs before Play Deadlock while the game is closed; status stays read-only.

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

For a clean external copy, use `scripts/prepare-portable.ps1` and audit with
`-ForDistribution`. This mode requires third-party notices and rejects local
user-data. The preparation verifies copied files against the build receipt and
voice-pack catalog. About embeds the notices so executable-only client updates
also deliver their applicable notices.

Launch the standalone window and test voice playback in Deadlock after release
changes. Automated file checks do not establish that every voice line has been
listened to in-game.

## Release candidate checked on 2026-10-18

The compiled alpha.5 candidate was copied to a clean writable folder and checked
with the native diagnostic commands. The executable matches its build receipt
and embeds `pedrommartini/deadvox-downloads`. Without a local VPK, public coverage
remains visible while installation is unavailable. After copying the verified
Lady Geist VPK, the native status detects it correctly. Read-only status and
update checks preserved the game configuration hash and selected voice language;
the game was not launched and voices were not switched.

The exact portable distribution passed the distribution allowlist with four
files and no local settings/backups. Both client and voice delivery manifests
passed the private local bundle validator. The official online channels returned
404 because no releases have been published. This establishes correct source
embedding and error reporting; it does not validate live download, activation or
client replacement through the official release channel. Complete that flow
after the initial release is authorized and the launch content is ready.
