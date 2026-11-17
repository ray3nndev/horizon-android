# Horizon Android

Android platform port of the original [Horizon Rust Minecraft engine](https://github.com/horizon-rust/horizon), maintained separately from the Vyron launcher.

## Upstream source

The upstream engine is tracked as a Git submodule at `engine/horizon-upstream`, pinned to:

`90e97a1fc2fa4aac23766e418f55cbce94e97618`

## Verified build status

The `Validate Horizon Android patches` workflow has successfully completed on 2026-11-17 for commit `18f7eee5feec2d2ae0005cf0901612f22711b552`.

The workflow verified that:
- All seven Android port patches apply cleanly.
- The patched Horizon client passes the Linux host check.
- `horizon-client` and the JNI bridge pass Android `aarch64-linux-android` cross-checks.
- The release bridge builds as an ELF64 AArch64 Android shared library (Android API 24 target).
- The resulting `libhorizon_android_bridge.so` was uploaded as the `horizon-android-arm64-native` workflow artifact.

The bridge currently accepts app-private paths, asks Horizon to validate and prepare its Android startup state, and exposes the winit GameActivity native entry point that hands that state to Horizon's mod host.

## Not yet verified

A successful native-library build is not proof of a playable Android client. This repository has not yet demonstrated:
- The bridge packaged and loaded inside Vyron.
- GameActivity launch from Vyron and successful native entry on a physical device.
- A working Android graphics surface, frame rendering, pause/resume, or context recovery.
- A complete Minecraft 26.3 resource-preparation and first-world gameplay flow on device.
- Touch, keyboard, controller, audio, and multiplayer behavior on device.

Do not describe this port as playable until those checks pass.

## Integration contract

- `horizon-android` owns the original Horizon Rust Android port and native bridge.
- `VyronLauncher` owns the existing Zalith-derived launcher UI, accounts, installation flow, and Android application.
- Keep credentials out of logs and persistent launch DTOs. The bridge must receive only the minimum session data required by the engine.
- Preserve both `.hmod` and `.hnmod` workflows.
- Do not integrate Starling as the target engine or replace Vyron's launcher UI.
- Do not remove existing Java preparation or launcher runtime components until their roles in Horizon 26.3 preparation are traced and tested.

## Next engineering gates

1. Package the ARM64 bridge in Vyron and add a matching, minification-safe Kotlin JNI wrapper.
2. Wire the existing launcher launch flow to the native GameActivity without bypassing account validation or version preparation.
3. Confirm the Activity/native entry lifecycle and renderer startup on a physical Android device.
4. Complete Minecraft 26.3 preparation, then test title screen and a real world.
5. Only after those gates pass, remove obsolete fallback launch code and runtime assets.
