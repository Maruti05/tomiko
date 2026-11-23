package com.vedica.labs.ind.app.talkin.tomiko

import android.app.Application
import com.vedica.labs.ind.app.talkin.tomiko.di.AppModule

class TomikoApplication : Application() {

    override fun onCreate() {
        super.onCreate()
        AppModule.init(this)
    }
}
