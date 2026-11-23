package com.vedica.labs.ind.app.talkin.tomiko.domain.model

sealed class AppError(message: String) : Exception(message) {
    class AudioPermissionDenied : AppError("Microphone permission required")
    class RecordingFailed(reason: String) : AppError("Recording failed: $reason")
    class ProcessingFailed(reason: String) : AppError("Processing error: $reason")
    class PlaybackFailed(reason: String) : AppError("Playback failed: $reason")
    class StorageFull : AppError("Device storage is full")
    class CharacterNotFound(id: String) : AppError("Character not found: $id")
    class Unknown(cause: Throwable) : AppError("Unexpected error: ${cause.message}")
}
