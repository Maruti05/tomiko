package com.vedica.labs.ind.app.talkin.tomiko.presentation.navigation

sealed class Screen(val route: String) {
    data object Home : Screen("home")
    data object Settings : Screen("settings")
}
