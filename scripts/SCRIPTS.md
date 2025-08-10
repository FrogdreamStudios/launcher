# Scripts
These scripts are used to build and sign the app for macOS.
It can create a release build, a debug build, or a build with a DMG file.

Without signing, the app can be run on macOS, but with security annoying warnings.

We're also using certificates to sign the app bundle, which is required for distribution on macOS.
If we don't have a certificate, the script will create an ad-hoc certificate as a fallback.

## Structure
- build_and_sign.sh: main script
- create_app_bundle.sh: creating app bundle from binary file
- setup_dev_signing.sh: creating certificate (ad-hoc as fallback)
- sign_app.sh: signing app bundle
### Release build
```
./scripts/build_and_sign.sh
```

### Release build with DMG
```
./scripts/build_and_sign.sh --dmg
```

### Debug build
```
./scripts/build_and_sign.sh --debug
```

### Sign app bundle
```
./scripts/sign_app.sh
```
