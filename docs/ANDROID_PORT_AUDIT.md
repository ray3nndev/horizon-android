# Android port source audit

Audit target: Horizon upstream commit `90e97a1fc2fa4aac23766e418f55cbce94e97618`.

This document records verified source boundaries before the first engine patch. It is deliberately explicit about what exists and what does not.

## Confirmed reusable Android work

- `crates/horizon-client-render/src/android_runtime.rs` contains platform-independent Android input state for touch/keypad and controller selection.
- `crates/horizon-client-render/src/android_gamepad.rs` maps Android key codes and motion axes into Horizon's pad state.
- Both files state that they are pure logic. They do not create an Android activity, receive Android lifecycle callbacks, or provide a game launch entry point.
- `horizon-client` already exposes `entry::start`, but its current `entry::prepare` constructs launch arguments from the desktop executable and process arguments.

## Startup blockers

1. `entry::Startup` has private fields; outside callers cannot construct a valid startup value.
2. `entry::prepare` uses `std::process::Command::new(std::env::current_exe()?)` and `std::env::args_os()`. That is not an Android app launch contract.
3. `game::Options` is re-exported as `pub(crate)`; the host cannot create it through the public API.
4. `Options::from_launch` is internal and derives its resource directory from `current_exe()`. It also consumes desktop environment state.
5. `Options::read` changes the process working directory, reads a session from a file or stdin, and performs launcher-related account setup. Android needs explicit app-private paths and a safe account handoff.
6. `horizon-client` forbids unsafe code. JNI/NativeActivity glue therefore belongs in a separate Android bridge crate, not in unsafe code added to that crate.
7. `horizon-client` and `horizon-client-render` depend on winit/wgpu. Android surface creation, event-loop ownership, Vulkan/GLES selection, and lifecycle pause/resume still need to be proven on-device.

## Required implementation sequence

### A. Establish a platform API inside Horizon

Add a narrow, safe Android-facing API within `horizon-client` that accepts explicit launch context (game directory, home/resources paths, player/session input, and arguments) and constructs validated startup state without consulting `current_exe()` or process arguments.

Keep the existing desktop `prepare()` behavior unchanged. Do not make Android fabricate a desktop executable path.

### B. Add a separate Android bridge

Create an Android `cdylib` bridge that owns JNI exports and lifecycle integration. It should call the safe platform API from A, retain the Android app/window handles for the lifetime required by winit, and translate Android lifecycle/input events into the engine's existing runtime.

### C. Prove graphics and lifecycle

Build only `arm64-v8a` first. Verify surface creation, resize, pause/resume, context loss, touch, keyboard, controller, and clean shutdown on a physical device. Vulkan support must be detected from actual device capabilities; do not hard-require Vulkan 1.3 without a GLES fallback test.

### D. Integrate with Vyron

Only after an Android engine smoke test succeeds should Vyron call the new native library. Keep Starling available as a temporary rollback path until a real Minecraft world reaches gameplay. Preserve both `.hmod` and `.hnmod` workflows during the transition.

## Non-goals for the first patch

- Do not remove Android's Kotlin/Java application code.
- Do not remove Java preparation/runtime assets until their exact role is traced.
- Do not claim that the existing Android input modules constitute a working Android engine.
- Do not claim Minecraft gameplay support based only on a successful Rust library build.

## Current status

Source audit complete. No JNI bridge, Android startup API, Android native library, or device-tested gameplay has been implemented yet.
