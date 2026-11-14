package dev.ruurd.photos

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.ruurd.photos.domain.model.AuthState
import dev.ruurd.photos.features.auth.AuthScreen
import dev.ruurd.photos.features.auth.AuthViewModel
import dev.ruurd.photos.features.main.MainScreen
import dev.ruurd.photos.features.main.MainViewModel
import dev.ruurd.photos.ui.theme.RuurdPhotosTheme

class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        val app = application as PhotosApplication
        val container = app.container

        setContent {
            RuurdPhotosTheme {
                val authState by container.authRepository.authState.collectAsState()

                when (authState) {
                    is AuthState.Loading -> {
                        Box(
                            modifier = Modifier
                                .fillMaxSize()
                                .background(MaterialTheme.colorScheme.background),
                            contentAlignment = Alignment.Center
                        ) {
                            CircularProgressIndicator()
                        }
                    }
                    is AuthState.Unauthenticated -> {
                        val authViewModel: AuthViewModel = viewModel(
                            factory = AuthViewModel.factory(container.authRepository)
                        )
                        AuthScreen(viewModel = authViewModel)
                    }
                    is AuthState.Authenticated -> {
                        val mainViewModel: MainViewModel = viewModel(
                            factory = MainViewModel.factory(
                                authRepository = container.authRepository,
                                mediaRepository = container.mediaRepository,
                                syncTimelineUseCase = container.syncTimelineUseCase,
                                networkMonitor = container.networkMonitor
                            )
                        )
                        MainScreen(viewModel = mainViewModel)
                    }
                }
            }
        }
    }
}
