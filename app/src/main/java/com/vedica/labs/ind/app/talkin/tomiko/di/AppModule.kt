package com.vedica.labs.ind.app.talkin.tomiko.di

import android.app.Application
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider

/**
 * Minimal application scope. The native engine owns audio/rendering; Compose
 * only needs the application context and a small ViewModel factory.
 */
object AppModule {

    lateinit var application: Application
        private set

    fun init(app: Application) {
        application = app
    }

    class Factory(private val creator: () -> ViewModel) : ViewModelProvider.Factory {
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            @Suppress("UNCHECKED_CAST")
            return creator() as T
        }
    }
}
