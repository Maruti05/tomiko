package com.vedica.labs.ind.app.talkin.tomiko.util

object Constants {
    const val MAX_RECORD_DURATION_MS = 10_000L
    const val SAMPLE_RATE = 44100
    const val AUDIO_BUFFER_SIZE = 4096
    const val FRAME_SIZE = 512
    const val SILENCE_THRESHOLD = 0.02f
    const val MAX_FILE_SIZE_BYTES = 10 * 1024 * 1024L // 10 MB
    const val CHARACTERS_DIR = "characters"
    const val RECORDINGS_DIR = "recordings"
    const val CONFIG_DIR = "config"
    const val CACHE_DIR = "cache"
}
