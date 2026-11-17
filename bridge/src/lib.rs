//! JNI boundary for Vyron's Android host.
//!
//! This crate establishes a typed, app-private launch-context handoff. The
//! engine startup API and Android GameActivity/winit lifecycle are connected
//! in subsequent port stages.

use jni::{
    JNIEnv,
    objects::{JClass, JString},
    sys::jint,
};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchContext {
    pub game_dir: String,
    pub home_dir: String,
    pub resources_dir: String,
}

static LAUNCH_CONTEXT: OnceLock<Mutex<Option<LaunchContext>>> = OnceLock::new();

fn context_slot() -> &'static Mutex<Option<LaunchContext>> {
    LAUNCH_CONTEXT.get_or_init(|| Mutex::new(None))
}

fn java_string(env: &mut JNIEnv<'_>, value: JString<'_>) -> Result<String, jni::errors::Error> {
    env.get_string(&value).map(|value| value.into())
}

/// JNI method for com.movtery.zalithlauncher.nativebridge.HorizonBridge.
/// Returns 0 on success, -1 if a Java string cannot be decoded or the lock is poisoned.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_movtery_zalithlauncher_nativebridge_HorizonBridge_nativeConfigure(
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
            0
        }
        Err(_) => -1,
    }
}

/// Returns true once Android has supplied all required paths.
pub fn has_launch_context() -> bool {
    context_slot()
        .lock()
        .map(|slot| slot.is_some())
        .unwrap_or(false)
}

/// Clones the launch context for the engine startup adapter.
pub fn launch_context() -> Option<LaunchContext> {
    context_slot().lock().ok().and_then(|slot| slot.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_context_is_absent_until_android_configures_it() {
        assert!(!has_launch_context());
        assert!(launch_context().is_none());
    }
}
