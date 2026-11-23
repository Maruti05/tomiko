package com.vedica.labs.ind.app.talkin.tomiko.util

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import androidx.core.content.ContextCompat

fun Context.hasRecordAudioPermission(): Boolean {
    return ContextCompat.checkSelfPermission(
        this, Manifest.permission.RECORD_AUDIO
    ) == PackageManager.PERMISSION_GRANTED
}

fun isAudioRecordAvailable(): Boolean {
    return try {
        val record = AudioRecord(
            MediaRecorder.AudioSource.MIC,
            44100,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT,
            4096
        )
        val available = record.state == AudioRecord.STATE_INITIALIZED
        record.release()
        available
    } catch (e: Exception) {
        false
    }
}
