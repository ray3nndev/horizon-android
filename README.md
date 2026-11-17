# Horizon Android

Android platform port of the original [Horizon Rust Minecraft engine](https://github.com/horizon-rust/horizon).

## Upstream source

The upstream engine is tracked as a Git submodule at `engine/horizon-upstream`, pinned to:

`90e97a1fc2fa4aac23766e418f55cbce94e97618`

Upstream repository: https://github.com/horizon-rust/horizon

## Porting status

This repository is the dedicated workspace for the Android port. It is **not yet a working Android engine**.

Planned work:
1. Add an Android-native library entry point and JNI/NativeActivity lifecycle.
2. Replace desktop process-argument startup with an Android launch/configuration API.
3. Supply Android implementations for runtime, storage, account/session handoff, input, and graphics.
4. Build and validate for `arm64-v8a`.
5. Integrate the resulting engine into Vyron, then remove the temporary Starling dependency only after end-to-end gameplay works.

The desktop executable is not an Android runtime. The port must reuse the engine's Rust crates and adapt platform boundaries rather than trying to run the PC executable on Android.

## Compatibility requirements

- Preserve both `.hmod` and `.hnmod` package workflows.
- Do not remove Android launcher JVM code or any Java preparation stage until its role is verified.
- Do not claim gameplay support until tested on a physical Android device.
