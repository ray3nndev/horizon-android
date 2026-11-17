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
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use horizon_client::entry::Startup;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchContext {
    pub game_dir: String,
    pub home_dir: String,
    pub resources_dir: String,
}

static LAUNCH_CONTEXT: OnceLock<Mutex<Option<LaunchContext>>> = OnceLock::new();
static PREPARED_STARTUP: OnceLock<Mutex<Option<Startup>>> = OnceLock::new();

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
) -> jint {
    let context = match (
        java_string(&mut env, game_dir),
        java_string(&mut env, home_dir),
        java_string(&mut env, resources_dir),
    ) {
        (Ok(game_dir), Ok(home_dir), Ok(resources_dir)) => LaunchContext {
            game_dir,
            home_dir,
            resources_dir,
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
        Vec::new(),
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
