# Finished voice-pack contract

`app/src-tauri/resources/voice-packs.json` is the only pack metadata compiled
into the application. Its schema is strict: unknown fields are rejected.

| Field | Meaning |
| --- | --- |
| `schema_version` | Catalog schema version, currently 1 |
| `packs[].id` | Public pack identifier |
| `language` | Voice language, currently `pt-BR` |
| `version` | Public version of the pack |
| `file` | Relative VPK location beside the portable executable |
| `sha256`, `bytes` | Integrity and completeness checks |
| `entry_count` | Number of packaged voice assets |
| `heroes[].asset_id` | Hero asset identifier |
| `heroes[].total`, `available` | Public speech coverage |
| `download_url` | Future HTTPS release asset URL; currently null |

Only completed voice assets are delivered. The catalog contains no source audio
paths, production jobs, account credentials or production-service endpoints.
The application is responsible for installing and switching finished packs;
pack creation is outside this repository.

Client updates and voice-pack distribution are distinct. The private source repository
can build the executable without possessing the VPK. A portable local release
can bundle the VPK. Future GitHub releases can supply the same VPK as a separate
asset in a separate download-only repository, checked against the published
checksum before installation. Source and build scripts are not distributed.

The alpha.5 catalog supports up to 512 PT-BR packs with unique identities, filenames
and non-overlapping hero coverage. Each VPK must be self-contained, versioned and
smaller than 2 GiB; the combined catalog may be larger. Downloaded voice releases have their
own version and catalog stored under user-data; they do not require bundling the
voices into a new client. See [update delivery](UPDATES.md) for GitHub channels,
download validation and client compatibility. More voice languages are a future
extension.

The application snapshot exposes `heroes[].content_available` from the public
release catalog, independently of `heroes[].available`, which describes valid
locally present content. DeadVox uses the first value for editorial availability
and coverage; installation and activation are shown separately. A missing local
VPK therefore does not make a released character appear as Coming Later. This
does not change the strict pack-catalog schema or expose production progress.
