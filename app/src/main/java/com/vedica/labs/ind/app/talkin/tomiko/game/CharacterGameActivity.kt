package com.vedica.labs.ind.app.talkin.tomiko.game

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import android.util.Log
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.google.androidgamesdk.GameActivity
import com.vedica.labs.ind.app.talkin.tomiko.native.NativeEngine

/**
 * Full-screen Bevy character stage.
 *
 * The activity owns the Android lifecycle, microphone permission, audio focus,
 * and system UI. Rendering, character state, capture, DSP, and playback live in
 * the Rust cdylib.
 */
class CharacterGameActivity : GameActivity() {

    companion object {
        private const val TAG = "TomikoGame"
        private const val REQUEST_RECORD_AUDIO = 1001

        init {
            // Must load before GameActivity's native setup.
            System.loadLibrary("tomiko_game")
        }
    }

    private lateinit var audioFocus: AudioFocusManager
    private var recordPermissionGranted = false
    private var permissionRequested = false
    private var listening = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        NativeEngine.nativeInitialize()

        audioFocus = AudioFocusManager(
            this,
            onFocusLost = {
                stopListening()
                NativeEngine.nativePause()
            },
            onFocusGained = {
                if (recordPermissionGranted) {
                    startListening()
                    NativeEngine.nativeResume()
                }
            },
        )

        recordPermissionGranted = ContextCompat.checkSelfPermission(
            this,
            Manifest.permission.RECORD_AUDIO,
        ) == PackageManager.PERMISSION_GRANTED

        hideSystemUi()
        if (!recordPermissionGranted) {
            requestRecordPermission()
        }
    }

    override fun onResume() {
        super.onResume()
        hideSystemUi()
        NativeEngine.nativeResume()
        if (recordPermissionGranted) {
            startListening()
        }
    }

    override fun onPause() {
        stopListening()
        audioFocus.abandon()
        NativeEngine.nativePause()
        super.onPause()
    }

    override fun onStop() {
        NativeEngine.nativeStopListening()
        super.onStop()
    }

    override fun onDestroy() {
        NativeEngine.nativeShutdown()
        super.onDestroy()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode != REQUEST_RECORD_AUDIO) return
        recordPermissionGranted =
            grantResults.isNotEmpty() && grantResults[0] == PackageManager.PERMISSION_GRANTED
        if (recordPermissionGranted) {
            startListening()
        } else {
            Log.w(TAG, "Microphone permission denied; talk mode is unavailable")
        }
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) {
            hideSystemUi()
        }
    }

    private fun startListening() {
        if (listening) return
        if (!recordPermissionGranted) {
            requestRecordPermission()
            return
        }
        if (!audioFocus.request()) {
            Log.w(TAG, "Audio focus was not granted; not starting capture")
            return
        }
        NativeEngine.nativeStartListening()
        listening = true
    }

    private fun stopListening() {
        if (!listening) return
        NativeEngine.nativeStopListening()
        listening = false
    }

    private fun requestRecordPermission() {
        if (permissionRequested) return
        permissionRequested = true
        ActivityCompat.requestPermissions(
            this,
            arrayOf(Manifest.permission.RECORD_AUDIO),
            REQUEST_RECORD_AUDIO,
        )
    }

    private fun hideSystemUi() {
        WindowCompat.setDecorFitsSystemWindows(window, false)
        WindowInsetsControllerCompat(window, window.decorView).apply {
            hide(WindowInsetsCompat.Type.systemBars())
            systemBarsBehavior =
                WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }
    }
}
