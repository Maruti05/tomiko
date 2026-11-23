package com.vedica.labs.ind.app.talkin.tomiko.util

import android.Manifest
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.core.app.ActivityCompat

enum class PermissionState {
    UNKNOWN,
    GRANTED,
    DENIED_CAN_ASK,
    DENIED_PERMANENT,
}

data class AudioPermissionHandle(
    val state: PermissionState,
    val launchRequest: () -> Unit,
    val openAppSettings: () -> Unit,
)

@Composable
fun rememberAudioPermissionState(): AudioPermissionHandle {
    val context = LocalContext.current
    var state by remember { mutableStateOf(PermissionState.UNKNOWN) }

    LaunchedEffect(Unit) {
        state = if (context.hasRecordAudioPermission()) {
            PermissionState.GRANTED
        } else {
            PermissionState.UNKNOWN
        }
    }

    val launcher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestPermission(),
        onResult = { granted ->
            if (granted) {
                state = PermissionState.GRANTED
            } else {
                val activity = context as? android.app.Activity
                val canAsk = activity != null &&
                    ActivityCompat.shouldShowRequestPermissionRationale(
                        activity, Manifest.permission.RECORD_AUDIO
                    )
                state = if (canAsk) PermissionState.DENIED_CAN_ASK
                        else PermissionState.DENIED_PERMANENT
            }
        },
    )

    return AudioPermissionHandle(
        state = state,
        launchRequest = { launcher.launch(Manifest.permission.RECORD_AUDIO) },
        openAppSettings = {
            val intent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
                data = Uri.fromParts("package", context.packageName, null)
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            context.startActivity(intent)
        },
    )
}
