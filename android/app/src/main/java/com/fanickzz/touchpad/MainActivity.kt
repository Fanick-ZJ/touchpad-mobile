package com.fanickzz.touchpad

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.fanickzz.touchpad.ui.theme.TouchpadApp
import com.fanickzz.touchpad.ui.theme.TouchpadTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            TouchpadTheme {
                TouchpadApp()
            }
        }
    }
}
