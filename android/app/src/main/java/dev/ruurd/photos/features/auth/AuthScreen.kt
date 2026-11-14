package dev.ruurd.photos.features.auth

import android.content.pm.PackageManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Dns
import androidx.compose.material.icons.filled.Email
import androidx.compose.material.icons.filled.Error
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material.icons.filled.VisibilityOff
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusDirection
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import dev.ruurd.photos.R
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.ui.theme.PhotosRadii
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull

private const val PERMISSION_ACCESS_LOCAL_NETWORK = "android.permission.ACCESS_LOCAL_NETWORK"

@Composable
fun AuthScreen(
    viewModel: AuthViewModel,
    modifier: Modifier = Modifier
) {
    val uiState by viewModel.uiState.collectAsState()
    val context = LocalContext.current
    val focusManager = LocalFocusManager.current
    var passwordVisible by remember { mutableStateOf(false) }
    var localNetworkError by remember { mutableStateOf<String?>(null) }
    var pendingAction by remember { mutableStateOf<(() -> Unit)?>(null) }
    var hasLocalNetworkPermission by remember {
        mutableStateOf(
            Build.VERSION.SDK_INT < 37 ||
                    ContextCompat.checkSelfPermission(context, PERMISSION_ACCESS_LOCAL_NETWORK) ==
                    PackageManager.PERMISSION_GRANTED
        )
    }

    val localNetworkPermissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestPermission()
    ) { granted ->
        hasLocalNetworkPermission = granted
        val action = pendingAction
        pendingAction = null
        if (granted) {
            localNetworkError = null
            action?.invoke()
        } else {
            localNetworkError =
                "Local network permission is required to connect to a server on your Wi-Fi/LAN."
        }
    }

    fun runWithLocalNetworkCheck(action: () -> Unit) {
        localNetworkError = null
        if (Build.VERSION.SDK_INT >= 37) {
            val granted = ContextCompat.checkSelfPermission(
                context,
                PERMISSION_ACCESS_LOCAL_NETWORK
            ) == PackageManager.PERMISSION_GRANTED
            hasLocalNetworkPermission = granted

            if (!granted && isLikelyLocalServerUrl(uiState.serverUrl)) {
                pendingAction = action
                localNetworkPermissionLauncher.launch(PERMISSION_ACCESS_LOCAL_NETWORK)
                return
            }
        }
        action()
    }

    val displayedError = localNetworkError ?: uiState.errorMessage

    Box(
        modifier = modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
            .imePadding(),
        contentAlignment = Alignment.Center
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 24.dp, vertical = 32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            // App Brand Mark
            Image(
                painter = painterResource(id = R.drawable.icon_transparent),
                contentDescription = "Ruurd Photos",
                contentScale = ContentScale.Fit,
                modifier = Modifier.size(72.dp)
            )

            Spacer(modifier = Modifier.height(16.dp))

            Text(
                text = "Ruurd Photos",
                style = MaterialTheme.typography.headlineMedium.copy(
                    fontWeight = FontWeight.Bold,
                    letterSpacing = (-0.5).sp
                ),
                color = MaterialTheme.colorScheme.onBackground
            )

            Text(
                text = "Sign in to connect to your personal library",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 4.dp, bottom = 28.dp)
            )

            // Auth Card
            Card(
                shape = RoundedCornerShape(PhotosRadii.Card),
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surfaceContainerHigh
                ),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(
                    modifier = Modifier.padding(20.dp),
                    verticalArrangement = Arrangement.spacedBy(16.dp)
                ) {
                    // Server URL Field
                    OutlinedTextField(
                        value = uiState.serverUrl,
                        onValueChange = {
                            localNetworkError = null
                            viewModel.onServerUrlChanged(it)
                        },
                        label = { Text("Server URL") },
                        placeholder = { Text("http://10.0.2.2:5272") },
                        leadingIcon = {
                            Icon(Icons.Default.Dns, contentDescription = null)
                        },
                        trailingIcon = {
                            if (uiState.isTestingServer) {
                                CircularProgressIndicator(
                                    modifier = Modifier.size(20.dp),
                                    strokeWidth = 2.dp
                                )
                            } else if (uiState.serverTestSuccess == true) {
                                Icon(
                                    Icons.Default.CheckCircle,
                                    contentDescription = "Online",
                                    tint = MaterialTheme.colorScheme.primary
                                )
                            } else if (uiState.serverTestSuccess == false) {
                                Icon(
                                    Icons.Default.Error,
                                    contentDescription = "Error",
                                    tint = MaterialTheme.colorScheme.error
                                )
                            }
                        },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(
                            keyboardType = KeyboardType.Uri,
                            imeAction = ImeAction.Next
                        ),
                        keyboardActions = KeyboardActions(
                            onNext = { focusManager.moveFocus(FocusDirection.Down) }
                        ),
                        shape = RoundedCornerShape(PhotosRadii.NestedCard),
                        modifier = Modifier.fillMaxWidth()
                    )

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.End
                    ) {
                        OutlinedButton(
                            onClick = { runWithLocalNetworkCheck(viewModel::testServerConnection) },
                            shape = CircleShape,
                            enabled = !uiState.isTestingServer && !uiState.isLoading
                        ) {
                            Text("Test Connection", style = MaterialTheme.typography.labelMedium)
                        }
                    }

                    // Email Field
                    OutlinedTextField(
                        value = uiState.email,
                        onValueChange = {
                            localNetworkError = null
                            viewModel.onEmailChanged(it)
                        },
                        label = { Text("Email") },
                        leadingIcon = {
                            Icon(Icons.Default.Email, contentDescription = null)
                        },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(
                            keyboardType = KeyboardType.Email,
                            imeAction = ImeAction.Next
                        ),
                        keyboardActions = KeyboardActions(
                            onNext = { focusManager.moveFocus(FocusDirection.Down) }
                        ),
                        shape = RoundedCornerShape(PhotosRadii.NestedCard),
                        modifier = Modifier.fillMaxWidth()
                    )

                    // Password Field
                    OutlinedTextField(
                        value = uiState.password,
                        onValueChange = {
                            localNetworkError = null
                            viewModel.onPasswordChanged(it)
                        },
                        label = { Text("Password") },
                        leadingIcon = {
                            Icon(Icons.Default.Lock, contentDescription = null)
                        },
                        trailingIcon = {
                            IconButton(onClick = { passwordVisible = !passwordVisible }) {
                                Icon(
                                    imageVector = if (passwordVisible) Icons.Default.Visibility else Icons.Default.VisibilityOff,
                                    contentDescription = if (passwordVisible) "Hide password" else "Show password"
                                )
                            }
                        },
                        visualTransformation = if (passwordVisible) VisualTransformation.None else PasswordVisualTransformation(),
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(
                            keyboardType = KeyboardType.Password,
                            imeAction = ImeAction.Done
                        ),
                        keyboardActions = KeyboardActions(
                            onDone = {
                                focusManager.clearFocus()
                                runWithLocalNetworkCheck(viewModel::login)
                            }
                        ),
                        shape = RoundedCornerShape(PhotosRadii.NestedCard),
                        modifier = Modifier.fillMaxWidth()
                    )

                    // Error Message Banner
                    AnimatedVisibility(visible = displayedError != null) {
                        displayedError?.let { error ->
                            Card(
                                colors = CardDefaults.cardColors(
                                    containerColor = MaterialTheme.colorScheme.errorContainer
                                ),
                                shape = RoundedCornerShape(PhotosRadii.NestedCard),
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Column(modifier = Modifier.padding(12.dp)) {
                                    Text(
                                        text = error,
                                        color = MaterialTheme.colorScheme.onErrorContainer,
                                        style = MaterialTheme.typography.bodySmall
                                    )
                                    if (Build.VERSION.SDK_INT >= 37 && !hasLocalNetworkPermission) {
                                        TextButton(
                                            onClick = {
                                                localNetworkPermissionLauncher.launch(
                                                    PERMISSION_ACCESS_LOCAL_NETWORK
                                                )
                                            }
                                        ) {
                                            Text("Allow Local Network Access")
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Sign In Button
                    Button(
                        onClick = {
                            focusManager.clearFocus()
                            runWithLocalNetworkCheck(viewModel::login)
                        },
                        shape = CircleShape,
                        enabled = !uiState.isLoading,
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(52.dp)
                    ) {
                        if (uiState.isLoading) {
                            CircularProgressIndicator(
                                color = MaterialTheme.colorScheme.onPrimary,
                                modifier = Modifier.size(24.dp),
                                strokeWidth = 2.dp
                            )
                        } else {
                            Text(
                                text = "Sign In",
                                style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.SemiBold)
                            )
                        }
                    }
                }
            }
        }
    }
}

private fun isLikelyLocalServerUrl(rawUrl: String): Boolean {
    val normalized = SessionManager.normalizeUrl(rawUrl)
    if (normalized.isEmpty()) return false
    val host = normalized.toHttpUrlOrNull()?.host?.lowercase() ?: return false

    if (host == "localhost" || host == "127.0.0.1" || host == "::1") {
        return false
    }
    if (host.endsWith(".local") || host.endsWith(".lan") || host.endsWith(".home") || !host.contains('.')) {
        return true
    }
    val octets = host.split('.')
    if (octets.size == 4) {
        val first = octets[0].toIntOrNull() ?: return false
        val second = octets[1].toIntOrNull() ?: return false
        return when {
            first == 10 -> true
            first == 192 && second == 168 -> true
            first == 172 && second in 16..31 -> true
            first == 169 && second == 254 -> true
            else -> false
        }
    }
    return false
}