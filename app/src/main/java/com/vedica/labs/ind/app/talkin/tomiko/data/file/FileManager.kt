package com.vedica.labs.ind.app.talkin.tomiko.data.file

import java.io.File

class FileManager(private val filesDir: File) {

    fun getCharactersDir(): File = File(filesDir, "characters").also { it.mkdirs() }

    fun getRecordingsDir(): File = File(filesDir, "recordings").also { it.mkdirs() }

    fun getConfigDir(): File = File(filesDir, "config").also { it.mkdirs() }

    fun getCacheDir(): File = File(filesDir, "cache").also { it.mkdirs() }

    fun createRecordingFile(): File {
        val dir = getRecordingsDir()
        val name = "rec_${System.currentTimeMillis()}"
        return File(dir, "$name.pcm")
    }

    fun createProcessedFile(): File {
        val dir = getRecordingsDir()
        val name = "proc_${System.currentTimeMillis()}.wav"
        return File(dir, name)
    }

    fun clearCache() {
        getCacheDir().deleteRecursively()
        getCacheDir().mkdirs()
    }

    fun getStorageUsedBytes(): Long {
        return listOf(getCharactersDir(), getRecordingsDir(), getConfigDir(), getCacheDir())
            .sumOf { it.walkTopDown().filter { f -> f.isFile }.sumOf { f -> f.length() } }
    }
}
