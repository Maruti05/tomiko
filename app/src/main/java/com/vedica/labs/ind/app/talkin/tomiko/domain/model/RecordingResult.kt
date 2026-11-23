package com.vedica.labs.ind.app.talkin.tomiko.domain.model

import java.io.File

sealed class RecordingResult {
    data class Success(val audioFile: File, val durationMs: Long) : RecordingResult()
    data class Error(val message: String) : RecordingResult()
}
