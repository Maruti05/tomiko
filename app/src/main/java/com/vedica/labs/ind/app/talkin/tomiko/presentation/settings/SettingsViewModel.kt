package com.vedica.labs.ind.app.talkin.tomiko.presentation.settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.vedica.labs.ind.app.talkin.tomiko.data.file.FileManager
import com.vedica.labs.ind.app.talkin.tomiko.data.file.JsonConfigStore
import kotlinx.coroutines.launch

class SettingsViewModel(
    private val configStore: JsonConfigStore,
    private val fileManager: FileManager,
) : ViewModel() {

    val appVersion = "1.0.0"

    fun clearAllData() {
        viewModelScope.launch {
            fileManager.clearCache()
            configStore.clearAll()
        }
    }
}
