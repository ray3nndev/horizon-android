//! JNI boundary for Vyron's Android host.
//!
//! The bridge receives app-private paths, prepares a real Horizon Startup, and
//! retains that validated state for the later Android runtime/lifecycle adapter.
//! This is not yet the renderer/game loop entry point.

use jni::{
    JNIEnv,
    objects::{JClass, JString},
    sys::jint,
};
use std::{
    env,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use horizon_client::entry::Startup;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchContext {
    pub game_dir: String,
    pub home_dir: String,
    pub resources_dir: String,
    pub launch_args_json: String,
}

static LAUNCH_CONTEXT: OnceLock<Mutex<Option<LaunchContext>>> = OnceLock::new();
static PREPARED_STARTUP: OnceLock<Mutex<Option<Startup>>> = OnceLock::new();
/// Serializes Java-preparer invocations because Horizon's preparer reads the
/// Java launcher configuration from the process environment.
static PREPARE_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn prepare_env_lock() -> &'static Mutex<()> {
    PREPARE_ENV_LOCK.get_or_init(|| Mutex::new(()))
}

struct RestoredEnv {
    values: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl RestoredEnv {
    fn capture(names: &'static [&'static str]) -> Self {
        Self {
            values: names.iter().map(|name| (*name, env::var_os(name))).collect(),
        }
    }
}

impl Drop for RestoredEnv {
    fn drop(&mut self) {
        for (name, value) in self.values.drain(..) {
            match value {
                Some(value) => unsafe { env::set_var(name, value) },
                None => unsafe { env::remove_var(name) },
            }
        }
    }
}

fn context_slot() -> &'static Mutex<Option<LaunchContext>> {
    LAUNCH_CONTEXT.get_or_init(|| Mutex::new(None))
}

fn startup_slot() -> &'static Mutex<Option<Startup>> {
    PREPARED_STARTUP.get_or_init(|| Mutex::new(None))
}

fn java_string(env: &mut JNIEnv<'_>, value: JString<'_>) -> Result<String, jni::errors::Error> {
    env.get_string(&value).map(|value| value.into())
}

/// Configure the filesystem roots supplied by the Android launcher.
/// Returns 0 on success and -1 when a Java string or lock operation fails.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_ray3nndev_vyron_nativebridge_HorizonBridge_nativeConfigure(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    game_dir: JString<'_>,
    home_dir: JString<'_>,
    resources_dir: JString<'_>,
    launch_args_json: JString<'_>,
) -> jint {
    let context = match (
        java_string(&mut env, game_dir),
        java_string(&mut env, home_dir),
        java_string(&mut env, resources_dir),
        java_string(&mut env, launch_args_json),
    ) {
        (Ok(game_dir), Ok(home_dir), Ok(resources_dir), Ok(launch_args_json)) => {
            if serde_json::from_str::<Vec<String>>(&launch_args_json).is_err() { return -2; }
            LaunchContext { game_dir, home_dir, resources_dir, launch_args_json }
        },
        _ => return -1,
    };

    match context_slot().lock() {
        Ok(mut slot) => {
            *slot = Some(context);
            // A new context invalidates any Startup prepared from older paths.
            if let Ok(mut startup) = startup_slot().lock() {
                *startup = None;
            } else {
                return -1;
            }
            0
        }
        Err(_) => -1,
    }
}

/// Ask Horizon itself to validate and prepare the Android launch.
/// Returns 0 on success, -1 if not configured, -2 if Horizon rejects the
/// launch context, and -3 if the bridge's state lock is poisoned.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_ray3nndev_vyron_nativebridge_HorizonBridge_nativePrepare(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jint {
    let context = match context_slot().lock() {
        Ok(slot) => match slot.clone() {
            Some(context) => context,
            None => return -1,
        },
        Err(_) => return -3,
    };

    let startup = match horizon_client::entry::prepare_android(
        PathBuf::from(context.game_dir),
        PathBuf::from(context.home_dir),
        PathBuf::from(context.resources_dir),
        match serde_json::from_str::<Vec<String>>(&context.launch_args_json) {
            Ok(args) => args,
            Err(_) => return -2,
        },
    ) {
        Ok(startup) => startup,
        Err(_) => return -2,
    };

    match startup_slot().lock() {
        Ok(mut slot) => {
            *slot = Some(startup);
            0
        }
        Err(_) => -3,
    }
}

/// Native GameActivity entry point. Winit's Android backend calls into this
/// function on its dedicated native thread after the Activity loads this
/// library. The launcher must call nativePrepare before opening GameActivity.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    if horizon_client::entry::set_android_app(app).is_err() {
        eprintln!("Vyron: Horizon Android Activity was attached more than once");
        return;
    }

    let startup = match startup_slot().lock() {
        Ok(mut slot) => slot.take(),
        Err(_) => {
            eprintln!("Vyron: Horizon startup state lock is poisoned");
            return;
        }
    };
    let Some(startup) = startup else {
        eprintln!("Vyron: nativePrepare must succeed before GameActivity starts");
        return;
    };

    // Start with an intentionally empty native-code inventory. The official
    // Horizon host still performs content-package discovery, dependency checks,
    // hook installation, and the actual client startup sequence.
    horizon_mod_host::main_with(Vec::new(), &[], &[], startup);
}

/// True after Android has supplied all required paths.
pub fn has_launch_context() -> bool {
    context_slot()
        .lock()
        .map(|slot| slot.is_some())
        .unwrap_or(false)
}

/// True after Horizon's Android startup validation has succeeded.
pub fn has_prepared_startup() -> bool {
    startup_slot()
        .lock()
        .map(|slot| slot.is_some())
        .unwrap_or(false)
}

/// Clones the launch context for diagnostics and later host integration.
pub fn launch_context() -> Option<LaunchContext> {
    context_slot().lock().ok().and_then(|slot| slot.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_context_is_absent_until_android_configures_it() {
        assert!(!has_launch_context());
        assert!(!has_prepared_startup());
        assert!(launch_context().is_none());
    }
}


/// Run Horizon's pinned 26.3 resource preparer from the Android installer.
/// Paths are app-private and the Java 25 shim must be exposed as <java_home>/bin/java.
/// The root must contain research-lock.json, the pinned parity sources, and precompiled
/// original exporters; this function deliberately uses the upstream preparer, not a subset.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_ray3nndev_vyron_nativebridge_HorizonBridge_nativePrepareResources(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    root: JString<'_>,
    minecraft_dir: JString<'_>,
    server_jar: JString<'_>,
    java_home: JString<'_>,
    output: JString<'_>,
    java_launcher: JString<'_>,
    jli_library: JString<'_>,
) -> jni::sys::jstring {
    let paths = (
        java_string(&mut env, root),
        java_string(&mut env, minecraft_dir),
        java_string(&mut env, server_jar),
        java_string(&mut env, java_home),
        java_string(&mut env, output),
        java_string(&mut env, java_launcher),
        java_string(&mut env, jli_library),
    );
    let result = match paths {
        (Ok(root), Ok(minecraft_dir), Ok(server_jar), Ok(java_home), Ok(output), Ok(java_launcher), Ok(jli_library)) => {
            let _guard = match prepare_env_lock().lock() {
                Ok(guard) => guard,
                Err(_) => return env.new_string(r#"{"ok":false,"error":"Java preparation environment lock is poisoned"}"#)
                    .map(|value| value.into_raw())
                    .unwrap_or(std::ptr::null_mut()),
            };

            // Horizon's Android patch discovers the Java launcher through these
            // environment variables. Keep them scoped to the preparation call so
            // unrelated Android/native processes cannot inherit stale paths.
            let _restored = RestoredEnv::capture(&[
                "HORIZON_JAVA_LAUNCHER",
                "HORIZON_JRE_HOME",
                "HORIZON_JLI_LIBRARY",
            ]);
            unsafe {
                env::set_var("HORIZON_JAVA_LAUNCHER", &java_launcher);
                env::set_var("HORIZON_JRE_HOME", &java_home);
                env::set_var("HORIZON_JLI_LIBRARY", &jli_library);
            }

            let options = horizon_prepare::Options {
                root: PathBuf::from(root),
                import: true,
                minecraft_dir: Some(PathBuf::from(minecraft_dir)),
                server_jar: Some(PathBuf::from(server_jar)),
                java_home: Some(PathBuf::from(java_home)),
                inputs: Default::default(),
                output: Some(PathBuf::from(output)),
            };
            let log = |line: &str| eprintln!("Vyron Horizon prepare: {line}");
            let progress = |fraction: f32| {
                eprintln!("Vyron Horizon prepare progress: {:.0}%", fraction * 100.0)
            };
            match horizon_prepare::prepare_with_progress(&options, &log, &progress) {
                Ok(resources) => match horizon_prepare::check(&options.root, &resources) {
                    Ok(missing) if missing.is_empty() => serde_json::json!({
                        "ok": true,
                        "resources": resources.to_string_lossy()
                    }),
                    Ok(missing) => serde_json::json!({
                        "ok": false,
                        "error": "Horizon resource verification found missing files",
                        "missing": missing
                    }),
                    Err(error) => serde_json::json!({"ok": false, "error": error}),
                },
                Err(error) => serde_json::json!({"ok": false, "error": error}),
            }
        }
        _ => serde_json::json!({"ok": false, "error": "Invalid resource preparation path"}),
    };
    match env.new_string(result.to_string()) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Check an existing resource folder with Horizon's complete official checker.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_ray3nndev_vyron_nativebridge_HorizonBridge_nativeCheckResources(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    root: JString<'_>,
    resources: JString<'_>,
) -> jni::sys::jstring {
    let result = match (java_string(&mut env, root), java_string(&mut env, resources)) {
        (Ok(root), Ok(resources)) => {
            match horizon_prepare::check(&PathBuf::from(root), &PathBuf::from(resources)) {
                Ok(missing) => serde_json::json!({
                    "ok": missing.is_empty(),
                    "missing": missing
                }),
                Err(error) => serde_json::json!({"ok": false, "error": error}),
            }
        }
        _ => serde_json::json!({"ok": false, "error": "Invalid resource verification path"}),
    };
    match env.new_string(result.to_string()) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
