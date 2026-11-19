package dev.ruurd.photos.features.auth

import android.content.pm.PackageManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Dns
import androidx.compose.material.icons.filled.Email
import androidx.compose.material.icons.filled.Error
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material.icons.filled.VisibilityOff
import androidx.compose.material.icons.filled.WifiFind
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
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
import androidx.compose.ui.draw.clip
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import dev.ruurd.photos.R
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.core.network.isLikelyLocalServerUrl
import dev.ruurd.photos.domain.model.DiscoveredServer
import dev.ruurd.photos.ui.theme.PhotosRadii

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
                "Local network permission is required to find or connect to a server on your Wi-Fi/LAN."
        }
    }

    fun runWithLocalNetworkCheck(
        requirePermissionAlways: Boolean = false,
        action: () -> Unit
    ) {
        localNetworkError = null
        if (Build.VERSION.SDK_INT >= 37) {
            val granted = ContextCompat.checkSelfPermission(
                context,
                PERMISSION_ACCESS_LOCAL_NETWORK
            ) == PackageManager.PERMISSION_GRANTED
            hasLocalNetworkPermission = granted

            if (!granted && (requirePermissionAlways || isLikelyLocalServerUrl(uiState.serverUrl))) {
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
                    // Server URL Field + Scan LAN Trigger
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        OutlinedTextField(
                            value = uiState.serverUrl,
                            onValueChange = {
                                localNetworkError = null
                                viewModel.onServerUrlChanged(it)
                            },
                            label = { Text("Server URL") },
                            placeholder = { Text("https://photos.ruurd.dev") },
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
                            modifier = Modifier.weight(1f)
                        )

                        FilledTonalIconButton(
                            onClick = {
                                focusManager.clearFocus()
                                if (uiState.isScanningLan) {
                                    viewModel.cancelLanScan()
                                } else {
                                    runWithLocalNetworkCheck(requirePermissionAlways = true) {
                                        viewModel.scanLocalNetwork()
                                    }
                                }
                            },
                            enabled = !uiState.isLoading,
                            modifier = Modifier
                                .padding(top = 6.dp)
                                .size(50.dp)
                        ) {
                            Icon(
                                imageVector = if (uiState.isScanningLan) Icons.Default.Close else Icons.Default.WifiFind,
                                contentDescription = if (uiState.isScanningLan) {
                                    "Cancel local network scan"
                                } else {
                                    "Scan Wi-Fi for servers"
                                }
                            )
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        UrlSchemeChip(
                            scheme = "http",
                            selected = uiState.serverUrl.startsWith("http://", ignoreCase = true),
                            onClick = {
                                localNetworkError = null
                                viewModel.onServerSchemeSelected("http")
                            }
                        )
                        UrlSchemeChip(
                            scheme = "https",
                            selected = uiState.serverUrl.startsWith("https://", ignoreCase = true),
                            onClick = {
                                localNetworkError = null
                                viewModel.onServerSchemeSelected("https")
                            }
                        )
                        Spacer(modifier = Modifier.weight(1f))
                        OutlinedButton(
                            onClick = {
                                if (uiState.isTestingServer) {
                                    viewModel.cancelServerTest()
                                } else {
                                    runWithLocalNetworkCheck(action = viewModel::testServerConnection)
                                }
                            },
                            shape = CircleShape,
                            enabled = !uiState.isLoading
                        ) {
                            Text(
                                text = if (uiState.isTestingServer) "Cancel" else "Test Connection",
                                style = MaterialTheme.typography.labelMedium
                            )
                        }
                    }

                    // LAN Discovery Progress & Results
                    AnimatedVisibility(
                        visible = uiState.isScanningLan ||
                                uiState.discoveredServers.isNotEmpty() ||
                                (uiState.hasScannedLan && displayedError == null)
                    ) {
                        Surface(
                            shape = RoundedCornerShape(PhotosRadii.NestedCard),
                            color = MaterialTheme.colorScheme.surfaceContainer,
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Column(
                                modifier = Modifier.padding(12.dp),
                                verticalArrangement = Arrangement.spacedBy(8.dp)
                            ) {
                                if (uiState.isScanningLan) {
                                    Text(
                                        text = "Scanning Wi-Fi for Ruurd Photos servers…",
                                        style = MaterialTheme.typography.labelMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                    val progress = uiState.scanProgress
                                    if (progress != null) {
                                        LinearProgressIndicator(
                                            progress = { progress },
                                            modifier = Modifier
                                                .fillMaxWidth()
                                                .clip(CircleShape)
                                        )
                                    } else {
                                        LinearProgressIndicator(
                                            modifier = Modifier
                                                .fillMaxWidth()
                                                .clip(CircleShape)
                                        )
                                    }
                                }

                                if (uiState.discoveredServers.isNotEmpty()) {
                                    Text(
                                        text = "Discovered on local network",
                                        style = MaterialTheme.typography.labelSmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                    uiState.discoveredServers.forEach { server ->
                                        DiscoveredServerRow(
                                            server = server,
                                            selected = SessionManager.normalizeUrl(uiState.serverUrl) == server.url,
                                            onClick = {
                                                localNetworkError = null
                                                viewModel.selectDiscoveredServer(server)
                                            }
                                        )
                                    }
                                } else if (!uiState.isScanningLan && uiState.hasScannedLan) {
                                    Text(
                                        text = "No Ruurd Photos servers found on this Wi-Fi network.",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                }
                            }
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
                                runWithLocalNetworkCheck(action = viewModel::login)
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
                            runWithLocalNetworkCheck(action = viewModel::login)
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

@Composable
private fun DiscoveredServerRow(
    server: DiscoveredServer,
    selected: Boolean,
    onClick: () -> Unit
) {
    Surface(
        shape = RoundedCornerShape(12.dp),
        color = if (selected) {
            MaterialTheme.colorScheme.secondaryContainer
        } else {
            MaterialTheme.colorScheme.surfaceContainerHigh
        },
        border = if (selected) {
            BorderStroke(1.dp, MaterialTheme.colorScheme.primary)
        } else {
            null
        },
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(12.dp))
            .clickable(onClick = onClick)
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = if (selected) Icons.Default.CheckCircle else Icons.Default.Dns,
                contentDescription = null,
                tint = if (selected) {
                    MaterialTheme.colorScheme.primary
                } else {
                    MaterialTheme.colorScheme.onSurfaceVariant
                },
                modifier = Modifier.size(18.dp)
            )
            Spacer(modifier = Modifier.width(10.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = server.url,
                    style = MaterialTheme.typography.bodyMedium.copy(fontWeight = FontWeight.Medium),
                    color = MaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
                val badge = buildString {
                    if (server.isEmulatorHost) append("Emulator host • ")
                    append("${server.latencyMs} ms")
                }
                Text(
                    text = badge,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

@Composable
private fun UrlSchemeChip(
    scheme: String,
    selected: Boolean,
    onClick: () -> Unit
) {
    Surface(
        shape = CircleShape,
        color = if (selected) {
            MaterialTheme.colorScheme.secondaryContainer
        } else {
            MaterialTheme.colorScheme.surfaceContainerHighest
        },
        modifier = Modifier
            .height(28.dp)
            .clip(CircleShape)
            .clickable(onClick = onClick)
    ) {
        Box(
            modifier = Modifier.padding(horizontal = 10.dp),
            contentAlignment = Alignment.Center
        ) {
            Text(
                text = scheme,
                style = MaterialTheme.typography.labelSmall,
                color = if (selected) {
                    MaterialTheme.colorScheme.onSecondaryContainer
                } else {
                    MaterialTheme.colorScheme.onSurfaceVariant
                }
            )
        }
    }
}

