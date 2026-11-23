package com.vedica.labs.ind.app.talkin.tomiko.presentation.settings

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.vedica.labs.ind.app.talkin.tomiko.data.file.FileManager
import com.vedica.labs.ind.app.talkin.tomiko.data.file.JsonConfigStore
import com.vedica.labs.ind.app.talkin.tomiko.di.AppModule
import com.vedica.labs.ind.app.talkin.tomiko.presentation.settings.components.ParentalGate

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    onBackClick: () -> Unit,
    viewModel: SettingsViewModel = viewModel(
        factory = AppModule.Factory {
            val app = AppModule.application
            val fm = FileManager(app.filesDir)
            SettingsViewModel(JsonConfigStore(fm), fm)
        }
    ),
) {
    var isUnlocked by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = "Settings",
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.Bold,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Back",
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                ),
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            if (!isUnlocked) {
                ParentalGate(
                    onSuccess = { isUnlocked = true },
                )
            } else {
                SettingsContent(viewModel)
            }
        }
    }
}

@Composable
private fun SettingsContent(viewModel: SettingsViewModel) {
    var showClearDialog by remember { mutableStateOf(false) }

    Text(
        text = "App Settings",
        style = MaterialTheme.typography.headlineMedium,
        fontWeight = FontWeight.Bold,
        color = MaterialTheme.colorScheme.onBackground,
        modifier = Modifier.fillMaxWidth(),
    )

    Spacer(modifier = Modifier.height(24.dp))

    // Volume (placeholder)
    SettingRow(
        title = "Volume",
        description = "Adjust voice playback volume",
    )

    HorizontalDivider(modifier = Modifier.padding(vertical = 12.dp))

    // Character reset
    SettingRow(
        title = "Reset Character",
        description = "Reset character position to default",
        action = {
            TextButton(onClick = { /* reset logic */ }) {
                Text("Reset", color = MaterialTheme.colorScheme.primary)
            }
        },
    )

    HorizontalDivider(modifier = Modifier.padding(vertical = 12.dp))

    // Clear data
    SettingRow(
        title = "Clear All Data",
        description = "Delete all recordings and reset settings",
        action = {
            TextButton(
                onClick = { showClearDialog = true },
            ) {
                Text("Clear", color = MaterialTheme.colorScheme.error)
            }
        },
    )

    HorizontalDivider(modifier = Modifier.padding(vertical = 12.dp))

    Spacer(modifier = Modifier.height(16.dp))

    Text(
        text = "Version ${viewModel.appVersion}",
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.fillMaxWidth(),
    )

    Spacer(modifier = Modifier.height(8.dp))

    Text(
        text = "\uD83D\uDED2 No data collected \u00B7 No internet required \u00B7 Kid safe",
        style = MaterialTheme.typography.labelMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )

    if (showClearDialog) {
        AlertDialog(
            onDismissRequest = { showClearDialog = false },
            title = {
                Text("Clear All Data?")
            },
            text = {
                Text("This will delete all recordings and reset settings. This cannot be undone.")
            },
            confirmButton = {
                Button(
                    onClick = {
                        viewModel.clearAllData()
                        showClearDialog = false
                    },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.error,
                    ),
                ) {
                    Text("Delete Everything")
                }
            },
            dismissButton = {
                TextButton(onClick = { showClearDialog = false }) {
                    Text("Cancel")
                }
            },
        )
    }
}

@Composable
private fun SettingRow(
    title: String,
    description: String,
    action: @Composable (() -> Unit)? = null,
) {
    Column(modifier = Modifier.fillMaxWidth()) {
        Text(
            text = title,
            style = MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Text(
            text = description,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        if (action != null) {
            action()
        }
    }
}
