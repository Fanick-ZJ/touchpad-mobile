package com.fanickzz.touchpad.data

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.util.Log
import androidx.lifecycle.ViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import java.net.InetSocketAddress
import java.net.Socket

class NetworkServiceDiscoveryViewModel : ViewModel() {

    private val _discoveredServices = MutableStateFlow<List<NsdServiceInfo>>(emptyList())
    val discoveredServices: StateFlow<List<NsdServiceInfo>> = _discoveredServices

    private val _isSearching = MutableStateFlow(false)
    val isSearching: StateFlow<Boolean> = _isSearching

    private var nsdManager: NsdManager? = null
    private var discoveryListener: NsdManager.DiscoveryListener? = null
    private val TAG = "NsdViewModel"

    private val resolvingServices = mutableSetOf<String>()

    fun startServiceDiscovery(context: Context, serviceType: String) {
        if (isSearching.value) {
            Log.d(TAG, "Discovery is already active.")
            return
        }

        nsdManager = context.getSystemService(Context.NSD_SERVICE) as NsdManager
        _discoveredServices.value = emptyList()
        _isSearching.value = true

        discoveryListener = object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(regType: String) {
                Log.d(TAG, "Service discovery started")
            }

            @Suppress("DEPRECATION")
            override fun onServiceFound(service: NsdServiceInfo) {
                if (service.serviceName !in resolvingServices) {
                    resolvingServices.add(service.serviceName)
                    nsdManager?.resolveService(service, object : NsdManager.ResolveListener {
                        override fun onResolveFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                            Log.e(TAG, "Resolve failed for ${serviceInfo.serviceName}: error $errorCode")
                            resolvingServices.remove(serviceInfo.serviceName)
                        }

                        override fun onServiceResolved(serviceInfo: NsdServiceInfo) {
                            Log.i(TAG, "Service resolved: $serviceInfo")
                            resolvingServices.remove(serviceInfo.serviceName)
                            if (discoveredServices.value.none { it.serviceName == serviceInfo.serviceName }) {
                                _discoveredServices.value = _discoveredServices.value + serviceInfo
                            }
                        }
                    })
                }
            }

            override fun onServiceLost(service: NsdServiceInfo) {
                Log.d(TAG, "Service lost: ${service.serviceName}")
                _discoveredServices.value = discoveredServices.value.filter { it.serviceName != service.serviceName }
                resolvingServices.remove(service.serviceName)
            }

            override fun onDiscoveryStopped(serviceType: String) {
                Log.i(TAG, "Discovery stopped: $serviceType")
                _isSearching.value = false
            }

            override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                Log.e(TAG, "Start discovery failed: error $errorCode")
                _isSearching.value = false
            }

            override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {
                Log.e(TAG, "Stop discovery failed: error $errorCode")
            }
        }

        nsdManager?.discoverServices(serviceType, NsdManager.PROTOCOL_DNS_SD, discoveryListener)
    }

    fun stopServiceDiscovery() {
        discoveryListener?.let {
            try {
                nsdManager?.stopServiceDiscovery(it)
                resolvingServices.clear()
            } catch (e: IllegalArgumentException) {
                // Listener not registered
                Log.w(TAG, "Discovery listener not registered.")
            } finally {
                _isSearching.value = false
            }
        }
    }

    fun sendConnectMessage(service: NsdServiceInfo) {
        Thread {
            val address = service.host.hostAddress ?: return@Thread
            val port    = service.port
            Log.d(TAG, "Sending connect message to $address:$port")

            var socket: Socket? = null
            try {
                // 1. 先建一个无连接 Socket
                socket = Socket()
                // 2. 设置 3 秒超时
                socket.soTimeout = 3000
                socket.tcpNoDelay = true
                // 3. 连接（现在最多阻塞 3 s）
                socket.connect(InetSocketAddress(address, port), 3000)
                Log.d(TAG, "Connected to $address:$port")

                socket.getOutputStream().use { out ->
                    out.write("CONNECT".toByteArray())
                }
            } catch (e: Exception) {
                Log.e(TAG, "连接/发送失败: ${e.message}", e)
            } finally {
                socket?.close()
            }
        }.start()
    }

    override fun onCleared() {
        super.onCleared()
        stopServiceDiscovery()
    }
}