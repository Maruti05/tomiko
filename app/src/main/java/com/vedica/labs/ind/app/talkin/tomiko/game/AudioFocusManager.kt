package com.vedica.labs.ind.app.talkin.tomiko.game

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager

/**
 * Owns Android audio focus for the native AAudio engine.
 *
 * Kotlin only requests/abandons focus and reports interruptions. Capture and
 * playback remain owned by Rust.
 */
class AudioFocusManager(
    context: Context,
    private val onFocusLost: () -> Unit,
    private val onFocusGained: () -> Unit,
) {
    private val audioManager =
        context.applicationContext.getSystemService(Context.AUDIO_SERVICE) as AudioManager
    private var request: AudioFocusRequest? = null

    fun request(): Boolean {
        if (request != null) return true
        val attributes = AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_MEDIA)
            .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
            .build()
        val focusRequest = AudioFocusRequest.Builder(
            AudioManager.AUDIOFOCUS_GAIN_TRANSIENT_EXCLUSIVE
        )
            .setAudioAttributes(attributes)
            .setAcceptsDelayedFocusGain(false)
            .setOnAudioFocusChangeListener { change ->
                when (change) {
                    AudioManager.AUDIOFOCUS_LOSS -> {
                        request = null
                        onFocusLost()
                    }
                    AudioManager.AUDIOFOCUS_LOSS_TRANSIENT,
                    AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK -> onFocusLost()
                    AudioManager.AUDIOFOCUS_GAIN -> onFocusGained()
                }
            }
            .build()

        val result = audioManager.requestAudioFocus(focusRequest)
        if (result == AudioManager.AUDIOFOCUS_REQUEST_GRANTED) {
            request = focusRequest
            return true
        }
        return false
    }

    fun abandon() {
        request?.let(audioManager::abandonAudioFocusRequest)
        request = null
    }
}
