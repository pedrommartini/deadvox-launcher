# DeadVox

An independent community dubbing platform for Deadlock, built around
character-by-character voice packs and a dedicated Windows launcher.
**AI Assisted • Human Directed.** The dark interface
offers Original and PT-BR audio, hero portraits, voice-pack coverage and a
Play Deadlock button. Current client version: `0.1.0-alpha.5`.

Private source repository: `pedrommartini/deadvox-launcher`. This repository is
not the public download channel. The separate official download-only repository
is `pedrommartini/deadvox-downloads`.

The launcher is implemented in Tauri 2, Rust and React/TypeScript. It detects
Steam libraries, verifies finished voice packs and reversibly activates them.
It runs without Python, a local web server, or access to a production workspace.

## Run the portable build

Open `builds/pre-alpha/DeadVox.exe` from the local writable build folder.
Keep the `voice-packs` folder beside the executable.
Close Deadlock before switching between Original and PT-BR. Start the game with
Play Deadlock. After a Steam update restores the game's configuration, select
PT-BR again with the game closed.

The current local build includes Lady Geist: 1,835 PT-BR speech lines. Original
sounds and creative performances remain preserved. Other heroes appear in the
catalog, but do not yet have an available voice pack in this release.

Windows x64, Steam, Deadlock and the WebView2 runtime are required. Saved settings
and backups are placed in `user-data` beside the executable and are not part of
future distributions. Game voice archives from Valve are not overwritten.

## Develop and build

Install Node.js, Rust and the Windows C++ build tools. From this repository:

```powershell
./scripts/build.ps1
```

This builds the launcher without bundling voice packs. For a local test release
with a finished VPK already placed in the repository's ignored `voice-packs`
directory:

```powershell
./scripts/build.ps1 -WithVoicePack
```

Outputs are in `builds/pre-alpha`; no ZIP is generated and nothing is published.
Build dependencies and caches
stay under `.cache`; `app/node_modules` contains frontend dependencies. The private source Git
allowlist permits only application source, public portraits, public metadata and
documentation. Executables, audio packs, saved settings and build caches are
excluded from the source repository. Finished binaries and voice packs can later
be distributed as GitHub release assets from a separate download-only repository.
The launcher source remains private; do not publish this development folder.
Public releases contain compiled application files, required third-party notices,
user documentation and finished packs, not application source or build scripts.

## Public pack contract

The application receives only a finished VPK, version, language, checksum, byte
size and public hero coverage. See [the contract](docs/public-pack-contract.md).
It never queries an audio-production service. Portraits are credited to Valve
via Deadlock API; this is an independent community project.

The catalog supports independently versioned character packs with HTTPS download URLs.
The launcher supports independent GitHub client/voice updates, a new-version
dialog, verified downloads and progress inside the Play Deadlock button.
The developer sets the official download repository in
`app/src-tauri/resources/update-source.json` before building for distribution.
Users cannot change the update source through Settings, native commands or saved
configuration. The official repository is `pedrommartini/deadvox-downloads`;
live channels must be published and verified before distribution. Local packs continue working offline.
Settings also provides a native folder chooser and Steam auto-detection.

See [update delivery](docs/UPDATES.md) for the separate client and voice channels
and commands to prepare local release folders. Preparation does not publish.

Alpha.5 supports a complete game roster spread across multiple packs, downloads
only missing or changed character files, and commits a new catalog after all files
pass integrity checks. Voice delivery remains separate from client delivery.

See [testing](docs/TESTING.md) and [release notes](RELEASE-NOTES.md).

Third-party notices are generated from the locked Windows dependencies and
frontend production dependencies during the build, embedded in About, and copied
beside the executable. Missing upstream license files are supplemented from
their exact source revisions. Before sharing a portable folder, create a clean
copy rather than copying a development or previously used folder:

```powershell
./scripts/prepare-portable.ps1 -ReleaseName alpha.5-candidate -WithVoicePack
```

This writes `builds/distribution/alpha.5-candidate`, refuses to replace an existing
folder, verifies the copied files and excludes saved settings/backups. It does
not publish or create a ZIP. Use `-BuildName` when building into a custom folder.
