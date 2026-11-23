package com.vedica.labs.ind.app.talkin.tomiko.domain.model

enum class VoiceEffect(
    val pitchShift: Float,
    val formantShift: Float,
    val reverbMix: Float,
) {
    NORMAL(1.0f, 1.0f, 0.0f),
    CHIPMUNK(1.8f, 1.3f, 0.0f),
    DEEP(0.6f, 0.7f, 0.05f),
    ROBOT(1.0f, 1.0f, 0.2f),
    ECHO(1.0f, 1.0f, 0.5f),
    ALIEN(1.5f, 0.5f, 0.3f),
}
