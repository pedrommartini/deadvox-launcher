# Microsoft Store distribution

The Microsoft Store package is built as an unsigned MSIX package for Store submission. Microsoft assigns the final package identity when the DeadVox product is reserved in Partner Center and re-signs the accepted submission.

## Build a submission package

After the Store account is verified and the product identity is reserved, obtain the Identity name, Publisher and Publisher display name from Partner Center. Build with the exact values:

```powershell
./scripts/build-store-package.ps1 \
  -IdentityName 'PARTNER_CENTER_IDENTITY_NAME' \
  -Publisher 'CN=PARTNER_CENTER_PUBLISHER' \
  -PublisherDisplayName 'PARTNER_CENTER_PUBLISHER_DISPLAY_NAME' \
  -PackageVersion '1.0.0.0'
```

The script builds the launcher with the `store` feature, uses `%LOCALAPPDATA%\DeadVox` for state and downloaded voice packs, and creates `builds/store/DeadVox.msix`. It does not sign, upload or submit anything.

Store builds obtain launcher updates from Microsoft Store. Voice-pack updates remain verified downloads from the configured public GitHub release channel.
