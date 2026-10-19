# 0.1.0-alpha.5

- Third-party notices embedded in About and included beside the portable executable.
- Clean portable preparation excludes user settings/backups and verifies copied files.
- Multiple independently versioned character packs in one PT-BR catalog, ready for the full game roster.
- Verified downloads skip complete files and preserve completed characters across interrupted attempts.
- A new catalog becomes installed only after every pack passes size and SHA-256 checks.
- Separate immutable character mounts and reversible Original/PT-BR selection; old packages are retained.
- Voice deployment remains separate from launcher deployment, with compatibility checks for older clients.
- Repaired local release preparation and added a build receipt to prevent distributing a client with an outdated embedded source.
- Official download repository: pedrommartini/deadvox-downloads, separate from private launcher source.
- Lady Geist audio and its independent version remain unchanged.

# 0.1.0-alpha.4

- Official DeadVox identity: supplied wordmark, crimson/ivory palette and serif display typography.
- Green patina now marks the primary Play action and positive ready states; crimson remains the brand and selection color.
- Added a warm ivory light theme with a persistent Dark/Light switch in Settings.
- Light-theme language selection now uses green; background geometry and edge texture follow the approved reference more closely.
- Language changes now open a branded confirmation with a staged project-support prompt; the support destination remains unset until an official channel is defined.
- The light wordmark now uses clean ink lettering with the crimson accent limited to its microphone mark.
- Added paired dark/light Windows icons that follow the selected launcher theme, plus a simplified character-led support scene.
- Success notifications now use the DeadVox diamond, texture and semantic green treatment.
- Home, roster, About, Settings and public application metadata rebranded.
- Available, In Production and Coming Later describe content; installation remains a separate state.
- Released content remains visible when its local voice pack is missing.
- Portable executable is DeadVox.exe; existing user-data, package paths and legacy installation markers remain compatible.
- The existing voice pack retains version 0.1.0-alpha.1. No audio was regenerated.
- No public release or domain deployment performed.

# 0.1.0-alpha.3

- Official update source embedded at build time; no user-selectable repository.
- Removed repository editing from Settings, native actions and CLI options.
- Legacy saved repository overrides are ignored without losing game settings.
- Closed-source distribution: private development repository and separate
  public download-only releases; no publication performed.
- Existing PT-BR voice pack remains unchanged at `0.1.0-alpha.1`.

# 0.1.0-alpha.2

- Independent GitHub channels and version checks for launcher and PT-BR voices.
- Dismissible update dialog and verified downloads with progress in Play Deadlock.
- Manual game-folder selection and Steam auto-detection in Settings.
- Native window-close protection, concurrent-operation rejection and exclusive
  game executable lock during voice application.
- Hidden helper for replacing only the launcher executable after restart.
- Separate local client/voice release preparation, without ZIP or publication.

An update repository must be configured before GitHub checks are available.
The existing voice package retains its independent version `0.1.0-alpha.1`.

# 0.1.0-alpha.1

- Independent Windows application with the existing minimal dark interface.
- Native Steam/Deadlock discovery and reversible voice-pack activation.
- Original and PT-BR voice selection, public hero roster and coverage.
- Local Lady Geist pack with 1,835 PT-BR speech lines.
- Checksums, atomic configuration writes and backups of owned packages.
- No dependency on an audio-production workspace or Python installation.
- Public source and portable release audited using explicit file allowlists.

An installer and additional voice languages are planned separately.
