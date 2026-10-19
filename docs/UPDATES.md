# Independent launcher and voice releases

The portable launcher and finished voice packs use independent versions and
GitHub release channels. Building or preparing a delivery does not publish it.
No ZIP is created. A client delivery contains an executable and a public JSON
manifest; a voice delivery contains a finished VPK and a different manifest.

As of alpha.5 the voice catalog supports multiple character packs. Each pack has
its own immutable version, filename, size, SHA-256 and hero coverage. The complete
catalog may exceed 2 GiB in total; individual VPK assets must be below 2 GiB.
Downloads skip verified files and retain completed character packs after an
interruption. The installed catalog changes only when every listed pack is complete.
Characters are mounted independently; switching to Original restores the original
game settings and retains the downloaded packages. Multi-pack catalogs require
`minimum_client_version` of `0.1.0-alpha.5` or newer.

## Local preparation

```powershell
./scripts/build.ps1 -WithVoicePack
./scripts/prepare-release.ps1 -Kind client -Repository OWNER/REPOSITORY
./scripts/prepare-release.ps1 -Kind voices -Repository OWNER/REPOSITORY
```

The official download repository is `pedrommartini/deadvox-downloads`, configured on 2026-10-18. Preparation creates separate
folders under `builds/deploy/client/VERSION` and
`builds/deploy/voices/pt-BR/catalog-TIMESTAMP`. The script verifies the delivered files and
constructs public metadata from explicit fields. It never uploads anything.

## Publishing later

1. For an application release, upload `DeadVox.exe` to the versioned
   `client-vVERSION` release. Upload its `client-update.json` to a separate
   release tagged `client-latest` (creating that channel once, then replacing
   only its manifest for subsequent releases).
2. For a voice release, upload new VPKs to the versioned catalog release identified
   by their URLs. Preserve existing hosted pack URLs when preparing subsequent
   catalogs. Upload its
   `voice-update.json` to the release tagged `voices-pt-br-latest`.
3. Before distributing clients, set `repository` to `OWNER/REPOSITORY` in
   `app/src-tauri/resources/update-source.json` and rebuild. This source is
   embedded in the executable. Users cannot change it through the UI, native
   commands, command-line arguments or saved state. Obsolete saved repository
   settings are ignored. The official source is `pedrommartini/deadvox-downloads`;
   `pedrommartini/deadvox-launcher` contains private development source only.

Keep application source in a private development repository. Use a separate
public download-only repository for release assets and user-facing documentation;
never upload this source tree there. GitHub-generated source archives of that
download repository must contain only its public download-page material, not
the launcher source. Do not embed access tokens for a private repository in a
distributed client. Distribution requires reviewing and including applicable
third-party license notices. The current delivery is portable; a dedicated
installer and its update behavior require separate packaging work.

The current downloader implements GitHub Releases only. A future storage/CDN
can replace it through a developer-issued client update with an explicit trusted
origin policy. It is not a user-selectable provider or arbitrary URL setting.

The channel manifest points at an immutable versioned asset. Publishing voices
does not change the client channel. Publishing a client does not replace voices.
Use a new VPK file name when its content changes; do not overwrite an existing
versioned VPK with different bytes. `-MinimumClientVersion` on voice preparation
defaults to the first client supporting multiple character packs (`0.1.0-alpha.5`); raise it
only when pack compatibility requires it. It is independent of the latest
client version. Client and pack versions use semantic
versioning, including pre-release ordering (`alpha.10` is newer than `alpha.2`).

## Runtime behavior

At startup, after locating the game, configured clients check both channels.
They check again every six hours while the app is visible, or manually through
Updates. A new version opens a dismissible update dialog. Users choose a client
update, a voice update, or continue with their installed version. With no
repository configured, the application remains fully usable with local packs.
GitHub connection errors are shown in Updates and do not block normal play.

Downloads stream into temporary files. The button that normally says Play
Deadlock displays the phase, progress percentage and a filled progress bar.
Unknown-length work uses an indeterminate bar rather than an invented percentage.
Size and SHA-256 are checked before publication. HTTPS download URLs must belong
to releases in the configured repository; redirects are limited to GitHub's
asset hosts. These checks trust the owner of the configured repository.

A voice download stores its independent catalog in `user-data/voice-catalog.json`.
If PT-BR is active and the game is closed, the verified pack is applied. If the
game is running, it is downloaded for later application; an Apply update button
appears after closing the game. Existing pack files remain available for recovery.
The `minimum_client_version` field rejects a pack requiring a newer client.
Adding characters or updating one character does not require rebuilding the launcher
while the public contract remains compatible. A catalog cannot silently remove an
installed character, downgrade a pack, or rewrite an existing version.

The two metadata-channel releases must remain mutable. Do not enable global
release immutability for a repository using this discovery mechanism. Versioned
payload assets are never overwritten by the delivery workflow. The release site
can be hosted independently under an existing domain and link to those same assets;
GitHub Pages is not used to store the VPKs.

A client download validates the Windows executable and prepares a hidden
PowerShell helper. Once the client closes, the helper retains the previous EXE
in `user-data/updates`, replaces only the launcher EXE and reopens the app.
Voice packs, configuration and saved game path are retained. Helper failures
are recorded in `user-data/updates/client-update-error.txt`. Windows PowerShell
is used only for the replacement of an executable that was running.

## Game location and operation protection

Steam registry/library manifests are checked automatically. Settings also
provides a native folder chooser, a path field and Detect with Steam. Selected
folders are validated before being saved; `Deadlock`, `game`, and `game/citadel`
selections are normalized to the installation root.

The frontend disables Play, voice switching and window close during operations.
The native process also rejects concurrent commands and prevents normal window
close/Alt+F4 until the operation finishes. A Windows named mutex prevents another
launcher process from starting a conflicting operation. During mod application,
an exclusive handle on the game's executable blocks launches from Steam and
external shortcuts as well. The handle is released when the operation finishes
or fails. No game process is forcibly terminated.

Forced process termination or power loss cannot be prevented by the UI. The
installer stages and verifies copied files, keeps backups and rolls back known
configuration-write failures. Normal interruption controls are handled natively.

## Public manifests

`client-update.json`: `schema_version`, `version`, `bytes`, `sha256`,
`download_url` ending in `.exe`.

`voice-update.json`: `schema_version`, `minimum_client_version`, `packs` with
the fields from [the finished pack contract](public-pack-contract.md).
Do not include credentials, source paths, production jobs or audio-generation
information. Only finished voice assets and public coverage are distributed.
