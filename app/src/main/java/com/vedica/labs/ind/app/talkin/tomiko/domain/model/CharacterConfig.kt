package com.vedica.labs.ind.app.talkin.tomiko.domain.model

import kotlinx.serialization.Serializable

@Serializable
data class CharacterConfig(
    val scale: Float = 1.0f,
    val initialRotation: List<Float> = listOf(0f, 180f, 0f),
    val camera: CameraConfig = CameraConfig(),
    val animations: AnimationConfig = AnimationConfig(),
    val morphTargets: Map<String, Int> = emptyMap(),
) {
    @Serializable
    data class CameraConfig(
        val distance: Float = 2.5f,
        val angleX: Float = 15f,
        val angleY: Float = 0f,
    )

    @Serializable
    data class AnimationConfig(
        val idle: ClipConfig = ClipConfig(),
        val listening: ClipConfig = ClipConfig(clip = 1, speed = 1f, loop = true),
        val talking: ClipConfig = ClipConfig(clip = 2, speed = 1f, loop = false),
        val happy: ClipConfig = ClipConfig(clip = 3, speed = 1.2f, loop = false),
    )

    @Serializable
    data class ClipConfig(
        val clip: Int = 0,
        val speed: Float = 0.8f,
        val loop: Boolean = true,
    )
}
