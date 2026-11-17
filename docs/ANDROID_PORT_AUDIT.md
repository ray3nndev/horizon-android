# Android port source audit and implementation status

Audit target: Horizon upstream commit `90e97a1fc2fa4aac23766e418f55cbce94e97618`.

This document records the original platform-boundary findings and the implementation now present in this repository.

## Original source findings

The upstream tree contained reusable Android input-state helpers, but the desktop startup path depended on process arguments, `current_exe()`, desktop resource discovery, and launcher/session setup. Those assumptions were not a suitable Android application contract. The upstream client also forbids unsafe code, so JNI belongs in a separate bridge crate.

Android still requires proof of winit/wgpu surface creation, event-loop ownership, graphics backend selection, and lifecycle behavior on a physical device.

## Implemented in this repository

- Seven ordered patches adapt resource paths, enable winit's Android GameActivity backend, provide clipboard and keyboard platform hooks, adapt Android window/input handling, expose an Android startup-preparation API, and route the event loop through the retained Android app handle.
- `bridge` is a Rust `cdylib` with JNI exports for Vyron's package:
  - `Java_com_ray3nndev_vyron_nativebridge_HorizonBridge_nativeConfigure`
  - `Java_com_ray3nndev_vyron_nativebridge_HorizonBridge_nativePrepare`
- `nativeConfigure` accepts the app-private game, home, and resources paths.
- `nativePrepare` calls Horizon's `prepare_android` API and stores the validated `Startup`.
- The Android `android_main` entry attaches the winit `AndroidApp`, takes the prepared startup state, and calls `horizon_mod_host::main_with`.
- GitHub Actions successfully applied all patches, checked the Linux client, cross-checked the Android client and bridge for `aarch64-linux-android`, built the release shared library, and verified its ELF header as AArch64 Android API 24.
- The successful run is [workflow run #38](https://github.com/ray3nndev/horizon-android/actions/runs/44064071070), commit `18f7eee5feec2d2ae0005cf0901612f22711b552`.

## Remaining blockers / proof gates

1. **Vyron packaging:** the native library is not yet packaged in Vyron's APK, and the matching Kotlin JNI wrapper is not yet wired into the launcher.
2. **Activity launch:** Vyron does not yet launch the required GameActivity/native entry from its existing validated launch flow.
3. **Graphics:** successful compilation does not prove Android surface creation, Vulkan/GLES compatibility, frame rendering, resize, pause/resume, or context recovery.
4. **Game preparation:** the full 26.3 resource preparation and required assets must be supplied through explicit Android paths.
5. **Device validation:** title screen, real-world gameplay, touch, keyboard, controller, audio, shutdown, and multiplayer need physical-device tests.

## Required integration sequence

### A. Package and call the native bridge

Add the ARM64 shared library to Vyron's Android native library packaging and add a matching minification-safe Kotlin wrapper. Do not call it until the JNI ABI and library loading path are verified.

### B. Preserve the existing launcher flow

Keep Zalith's account validation/refresh, version selection, download/repair, and game-directory semantics. Pass only the minimum session data required by Horizon; never pass passwords or refresh tokens.

### C. Launch and validate the native Activity

Start the GameActivity only after native startup preparation succeeds. Verify lifecycle, surface creation, rendering, input, audio, and shutdown on a physical ARM64 Android device.

### D. Verify Minecraft 26.3 gameplay

Complete the install/prepare stage before Play is enabled. Reach the title screen and load a real world before claiming engine integration complete.

## Non-goals

- Do not use Starling as the target engine.
- Do not replace Vyron's original Zalith-derived UI.
- Do not remove Java preparation/runtime components until their role is understood and the replacement path is proven.
- Do not claim gameplay based only on `cargo check` or a successful `.so` build.
