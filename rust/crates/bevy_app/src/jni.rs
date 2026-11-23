//! JNI exports. These live in the cdylib crate so the linker cannot drop them
//! from a dependency archive.
//!
//! Package: com.vedica.labs.ind.app.talkin.tomiko.native.NativeEngine
//! All methods are static and return integer/float status values.

use jni::EnvUnowned;
use jni::errors::ThrowRuntimeExAndDefault;
use jni::objects::{JClass, JString};
use jni::sys::{jfloat, jint};

fn status(result: Result<(), shared_types::EngineError>) -> jint {
    result.map_or_else(i32::from, |()| 0)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeInitialize<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> {
            Ok(status(android_bridge::initialize()))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeShutdown<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> {
            Ok(status(android_bridge::shutdown()))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativePause<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> {
            Ok(status(android_bridge::pause()))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeResume<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> {
            Ok(status(android_bridge::resume()))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeStartListening<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> {
            Ok(status(android_bridge::start_listening()))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeStopListening<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> {
            Ok(status(android_bridge::stop_listening()))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativePlayLast<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> {
            Ok(status(android_bridge::play_last()))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeSetVoicePreset<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    id: jint,
    intensity: jfloat,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> {
            Ok(status(android_bridge::set_voice_preset(id, intensity)))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeSetQuality<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    tier: jint,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> {
            Ok(status(android_bridge::set_quality(tier)))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeSetCharacter<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    id: JString<'caller>,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> {
            let id: String = id.to_string();
            Ok(status(android_bridge::set_character(id)))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeGetEngineState<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> {
            Ok(android_bridge::snapshot().engine_state)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeGetCharacterState<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> {
            Ok(android_bridge::snapshot().character_state)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeGetAmplitude<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jfloat {
    unowned_env
        .with_env(|_env| -> Result<jfloat, jni::errors::Error> {
            Ok(android_bridge::snapshot().amplitude)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeGetProgress<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jfloat {
    unowned_env
        .with_env(|_env| -> Result<jfloat, jni::errors::Error> {
            Ok(android_bridge::snapshot().progress)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeGetMouthOpen<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jfloat {
    unowned_env
        .with_env(|_env| -> Result<jfloat, jni::errors::Error> {
            Ok(android_bridge::snapshot().mouth_open)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeGetErrorCode<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> {
            Ok(android_bridge::snapshot().error_code)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativeGetAbiVersion<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> {
            Ok(shared_types::ENGINE_ABI_VERSION as jint)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vedica_labs_ind_app_talkin_tomiko_native_NativeEngine_nativePing<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    unowned_env
        .with_env(|_env| -> Result<jint, jni::errors::Error> { Ok(0x544F_4D49) })
        .resolve::<ThrowRuntimeExAndDefault>()
}
