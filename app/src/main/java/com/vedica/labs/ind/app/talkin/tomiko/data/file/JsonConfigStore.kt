package com.vedica.labs.ind.app.talkin.tomiko.data.file

import com.vedica.labs.ind.app.talkin.tomiko.domain.model.CharacterConfig
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import java.io.File

class JsonConfigStore(private val fileManager: FileManager) {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    suspend fun saveConfig(key: String, value: String) = withContext(Dispatchers.IO) {
        val file = File(fileManager.getConfigDir(), "$key.json")
        file.writeText(value)
    }

    suspend fun readConfig(key: String): String? = withContext(Dispatchers.IO) {
        val file = File(fileManager.getConfigDir(), "$key.json")
        if (file.exists()) file.readText() else null
    }

    suspend fun saveCharacterConfig(characterId: String, config: CharacterConfig) {
        saveConfig("char_${characterId}", json.encodeToString(CharacterConfig.serializer(), config))
    }

    suspend fun loadCharacterConfig(characterId: String): CharacterConfig? {
        val str = readConfig("char_${characterId}") ?: return null
        return try {
            json.decodeFromString(CharacterConfig.serializer(), str)
        } catch (_: Exception) { null }
    }

    suspend fun saveSelectedCharacterId(id: String) = saveConfig("selected_char", id)

    suspend fun getSelectedCharacterId(): String? = readConfig("selected_char")

    suspend fun clearAll() = withContext(Dispatchers.IO) {
        fileManager.getConfigDir().listFiles()?.forEach { it.delete() }
    }
}
