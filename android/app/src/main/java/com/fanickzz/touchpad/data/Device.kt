package com.fanickzz.touchpad.data

import android.net.MacAddress
import java.net.InetAddress

data class Device(
    val ip: InetAddress,
    val name: String,
    val mac: MacAddress
)
