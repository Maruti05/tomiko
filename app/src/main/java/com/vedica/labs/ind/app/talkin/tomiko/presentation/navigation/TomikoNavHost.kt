package com.vedica.labs.ind.app.talkin.tomiko.presentation.navigation

import android.content.Intent
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import com.vedica.labs.ind.app.talkin.tomiko.game.CharacterGameActivity
import com.vedica.labs.ind.app.talkin.tomiko.presentation.home.HomeScreen
import com.vedica.labs.ind.app.talkin.tomiko.presentation.settings.SettingsScreen

@Composable
fun TomikoNavHost(
    navController: NavHostController,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    NavHost(
        navController = navController,
        startDestination = Screen.Home.route,
        modifier = modifier,
    ) {
        composable(Screen.Home.route) {
            HomeScreen(
                onPlay = {
                    context.startActivity(
                        Intent(context, CharacterGameActivity::class.java)
                    )
                },
                onSettingsClick = {
                    navController.navigate(Screen.Settings.route)
                },
            )
        }

        composable(Screen.Settings.route) {
            SettingsScreen(
                onBackClick = { navController.popBackStack() },
            )
        }
    }
}
