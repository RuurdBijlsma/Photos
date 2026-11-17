package dev.ruurd.photos.core.network

import android.content.Context
import android.net.ConnectivityManager
import android.net.LinkProperties
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.net.Inet4Address
import java.util.concurrent.ConcurrentHashMap

class NetworkMonitor(context: Context) {
    private val connectivityManager =
        context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager

    private val lanAddresses = ConcurrentHashMap<Network, Inet4Address>()

    private val _isOnline = MutableStateFlow(checkCurrentConnectivity())
    val isOnline: StateFlow<Boolean> = _isOnline.asStateFlow()

    private val _lanIpv4Address = MutableStateFlow(seedCurrentLanIpv4())
    val lanIpv4Address: StateFlow<Inet4Address?> = _lanIpv4Address.asStateFlow()

    private val defaultCallback = object : ConnectivityManager.NetworkCallback() {
        override fun onAvailable(network: Network) {
            _isOnline.value = true
        }

        override fun onLost(network: Network) {
            _isOnline.value = checkCurrentConnectivity()
        }

        override fun onCapabilitiesChanged(
            network: Network,
            networkCapabilities: NetworkCapabilities
        ) {
            val hasInternet =
                networkCapabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) &&
                        networkCapabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED)
            _isOnline.value = hasInternet
        }
    }

    init {
        connectivityManager.registerDefaultNetworkCallback(defaultCallback)
        connectivityManager.registerNetworkCallback(
            NetworkRequest.Builder()
                .addTransportType(NetworkCapabilities.TRANSPORT_WIFI)
                .build(),
            createLanCallback()
        )
        connectivityManager.registerNetworkCallback(
            NetworkRequest.Builder()
                .addTransportType(NetworkCapabilities.TRANSPORT_ETHERNET)
                .build(),
            createLanCallback()
        )
    }

    private fun createLanCallback(): ConnectivityManager.NetworkCallback =
        object : ConnectivityManager.NetworkCallback() {
            override fun onLinkPropertiesChanged(network: Network, linkProperties: LinkProperties) {
                val ipv4 = extractSiteLocalIpv4(linkProperties)
                if (ipv4 != null) {
                    lanAddresses[network] = ipv4
                } else {
                    lanAddresses.remove(network)
                }
                _lanIpv4Address.value = lanAddresses.values.firstOrNull() ?: seedCurrentLanIpv4()
            }

            override fun onLost(network: Network) {
                lanAddresses.remove(network)
                _lanIpv4Address.value = lanAddresses.values.firstOrNull() ?: seedCurrentLanIpv4()
            }
        }

    private fun seedCurrentLanIpv4(): Inet4Address? {
        val activeNetwork = connectivityManager.activeNetwork ?: return null
        val caps = connectivityManager.getNetworkCapabilities(activeNetwork) ?: return null
        if (caps.hasTransport(NetworkCapabilities.TRANSPORT_VPN)) return null
        val isLan = caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) ||
                caps.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET)
        if (!isLan) return null
        return extractSiteLocalIpv4(connectivityManager.getLinkProperties(activeNetwork))
    }

    private fun extractSiteLocalIpv4(linkProperties: LinkProperties?): Inet4Address? =
        linkProperties?.linkAddresses
            ?.asSequence()
            ?.mapNotNull { it.address as? Inet4Address }
            ?.firstOrNull { !it.isLoopbackAddress && it.isSiteLocalAddress }

    private fun checkCurrentConnectivity(): Boolean {
        val activeNetwork = connectivityManager.activeNetwork ?: return false
        val capabilities = connectivityManager.getNetworkCapabilities(activeNetwork) ?: return false
        return capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
    }
}