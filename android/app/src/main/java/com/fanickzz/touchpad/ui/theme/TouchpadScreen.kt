package com.fanickzz.touchpad.ui.theme

import android.net.nsd.NsdServiceInfo
import android.util.Log
import androidx.annotation.StringRes
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavDestination.Companion.hierarchy
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import com.fanickzz.touchpad.R
import com.fanickzz.touchpad.data.NetworkServiceDiscoveryViewModel

sealed class Screen(val route: String, @StringRes val resourceId: Int, val icon: ImageVector) {
    object Home : Screen("home", R.string.home, Icons.Filled.Home)
    object Settings : Screen("setting", R.string.setting, Icons.Filled.Settings)
    object Operate : Screen("operate", R.string.operate, Icons.Filled.Home) // Icon not used on bottom bar
}

val bottomNavItems = listOf(
    Screen.Home,
    Screen.Settings,
)

@Composable
fun TouchpadApp(
    discoverServiceModel: NetworkServiceDiscoveryViewModel = viewModel(),
    navController: NavHostController = rememberNavController()
) {
    Scaffold(
        bottomBar = {
            NavigationBar {
                val navBackStackEntry by navController.currentBackStackEntryAsState()
                val currentDestination = navBackStackEntry?.destination
                bottomNavItems.forEach { screen ->
                    NavigationBarItem(
                        icon = { Icon(screen.icon, contentDescription = null) },
                        label = { Text(stringResource(screen.resourceId)) },
                        selected = currentDestination?.hierarchy?.any { it.route == screen.route } == true,
                        onClick = {
                            navController.navigate(screen.route) {
                                popUpTo(navController.graph.findStartDestination().id) {
                                    saveState = true
                                }
                                launchSingleTop = true
                                restoreState = true
                            }
                        }
                    )
                }
            }
        }
    ) { innerPadding ->
        NavHost(
            navController = navController,
            startDestination = Screen.Home.route,
            modifier = Modifier.padding(innerPadding)
        ) {
            composable(Screen.Home.route) {
                HomeScreen(
                    discoverServiceModel = discoverServiceModel,
                    onDeviceSelected = { service ->
                        discoverServiceModel.sendConnectMessage(service)
                        discoverServiceModel.stopServiceDiscovery()
                        val host = service.host?.hostAddress ?: "unknown"
                        navController.navigate("${Screen.Operate.route}/${service.serviceName}/$host/${service.port}")
                    }
                )
            }
            composable(route = "${Screen.Operate.route}/{deviceName}/{deviceHost}/{devicePort}") { backStackEntry ->
                val deviceName = backStackEntry.arguments?.getString("deviceName")
                val deviceHost = backStackEntry.arguments?.getString("deviceHost")
                val devicePort = backStackEntry.arguments?.getString("devicePort")?.toIntOrNull()
                OperateScreen(
                    deviceName = deviceName,
                    deviceHost = deviceHost,
                    devicePort = devicePort,
                    onNavigateUp = { navController.navigateUp() }
                )
            }
            composable(Screen.Settings.route) {
                SettingsScreen()
            }
        }
    }
}

@Composable
fun HomeScreen(
    discoverServiceModel: NetworkServiceDiscoveryViewModel,
    onDeviceSelected: (NsdServiceInfo) -> Unit
) {
    val discoveredServices by discoverServiceModel.discoveredServices.collectAsState()
    val isSearching by discoverServiceModel.isSearching.collectAsState()
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current

    DisposableEffect(lifecycleOwner, discoverServiceModel) {
        val observer = LifecycleEventObserver { _, event ->
            when (event) {
                Lifecycle.Event.ON_START -> {
                    discoverServiceModel.startServiceDiscovery(context, "_touchpad._tcp")
                }
                Lifecycle.Event.ON_STOP -> {
                    discoverServiceModel.stopServiceDiscovery()
                }
                else -> {}
            }
        }

        lifecycleOwner.lifecycle.addObserver(observer)

        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text(stringResource(R.string.home), style = MaterialTheme.typography.headlineMedium)
        Spacer(Modifier.height(16.dp))

        if (isSearching && discoveredServices.isEmpty()) {
            Column(
                modifier = Modifier.fillMaxSize(),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                CircularProgressIndicator()
                Spacer(modifier = Modifier.height(16.dp))
                Text("Searching for devices...")
            }
        } else if (!isSearching && discoveredServices.isEmpty()) {
            Column(
                modifier = Modifier.fillMaxSize(),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Text("No devices found.", style = MaterialTheme.typography.bodyLarge)
                Spacer(modifier = Modifier.height(16.dp))
                Button(onClick = { discoverServiceModel.startServiceDiscovery(context, "_touchpad._tcp") }) {
                    Text("Retry Search")
                }
            }
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                items(discoveredServices) { service ->
                    DeviceItem(service = service, onClick = { onDeviceSelected(service) })
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DeviceItem(service: NsdServiceInfo, onClick: () -> Unit) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        onClick = onClick,
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp)
    ) {
        Row(
            modifier = Modifier
                .padding(16.dp)
                .fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Column {
                Text(text = service.serviceName, style = MaterialTheme.typography.titleMedium)
                Spacer(modifier = Modifier.height(4.dp))
                val hostAddress = service.host?.hostAddress ?: "Resolving..."
                Text(text = "$hostAddress:${service.port}", style = MaterialTheme.typography.bodyMedium)
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun OperateScreen(deviceName: String?, deviceHost: String?, devicePort: Int?, onNavigateUp: () -> Unit) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(deviceName ?: "Operate") },
                navigationIcon = {
                    IconButton(onClick = onNavigateUp) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = stringResource(R.string.back_button)
                        )
                    }
                }
            )
        }
    ) { padding ->
        Column(modifier = Modifier
            .padding(padding)
            .padding(16.dp)) {
            Text(text = "Device: ${deviceName ?: "Unknown"}")
            Text(text = "Address: ${deviceHost ?: "Unknown"}:${devicePort ?: "N/A"}")
            // TODO: Implement QUIC communication and controls here
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen() {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.setting)) },
            )
        }
    ) { padding ->
        Column(modifier = Modifier
            .padding(padding)
            .padding(16.dp)) {
            // TODO: Add actual settings here
            Text("Version 1.0", modifier = Modifier.padding(top = 8.dp))
        }
    }
}
