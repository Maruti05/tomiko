package com.vedica.labs.ind.app.talkin.tomiko.native

/**
 * JNI facade for the Rust/Bevy engine.
 *
 * All functions return a stable native error code. No per-frame or
 * per-audio-buffer JNI calls are allowed; high-level UI state is polled at a
 * low rate.
 */
class NativeEngine private constructor() {

    companion object {
        init {
            System.loadLibrary("tomiko_game")
        }

        const val ABI_VERSION = 1

        @JvmStatic
        external fun nativeInitialize(): Int

        @JvmStatic
        external fun nativeShutdown(): Int

        @JvmStatic
        external fun nativePause(): Int

        @JvmStatic
        external fun nativeResume(): Int

        @JvmStatic
        external fun nativeStartListening(): Int

        @JvmStatic
        external fun nativeStopListening(): Int

        @JvmStatic
        external fun nativePlayLast(): Int

        @JvmStatic
        external fun nativeSetVoicePreset(id: Int, intensity: Float): Int

        @JvmStatic
        external fun nativeSetQuality(tier: Int): Int

        @JvmStatic
        external fun nativeSetCharacter(id: String): Int

        @JvmStatic
        external fun nativeGetEngineState(): Int

        @JvmStatic
        external fun nativeGetCharacterState(): Int

        @JvmStatic
        external fun nativeGetAmplitude(): Float

        @JvmStatic
        external fun nativeGetProgress(): Float

        @JvmStatic
        external fun nativeGetMouthOpen(): Float

        @JvmStatic
        external fun nativeGetErrorCode(): Int

        @JvmStatic
        external fun nativeGetAbiVersion(): Int

        @JvmStatic
        external fun nativePing(): Int
    }
}
