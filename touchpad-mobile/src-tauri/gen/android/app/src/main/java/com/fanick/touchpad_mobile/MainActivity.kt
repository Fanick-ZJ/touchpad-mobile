package com.fanick.touchpad_mobile

import android.os.Bundle
import android.view.View
import android.view.WindowManager
import android.content.res.Configuration
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  companion object {
    private var instance: MainActivity? = null

    fun getInstance(): MainActivity? = instance
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    enableEdgeToEdge()
    instance = this
  }

  override fun onDestroy() {
    super.onDestroy()
    instance = null
  }

  /**
   * 设置全屏模式
   * @param enabled true=启用全屏, false=退出全屏
   */
  fun setFullScreen(enabled: Boolean) {
    if (enabled) {
      // 启用全屏
      if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
        window.setDecorFitsSystemWindows(false)
        window.insetsController?.let { controller ->
          controller.hide(android.view.WindowInsets.Type.statusBars() or android.view.WindowInsets.Type.navigationBars())
          controller.systemBarsBehavior = android.view.WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }
      } else {
        @Suppress("DEPRECATION")
        window.decorView.systemUiVisibility = (
            View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
            or View.SYSTEM_UI_FLAG_LAYOUT_STABLE
            or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
            or View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
            or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
            or View.SYSTEM_UI_FLAG_FULLSCREEN
        )
      }
    } else {
      // 退出全屏
      if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
        window.setDecorFitsSystemWindows(true)
        window.insetsController?.show(android.view.WindowInsets.Type.statusBars() or android.view.WindowInsets.Type.navigationBars())
      } else {
        @Suppress("DEPRECATION")
        window.decorView.systemUiVisibility = View.SYSTEM_UI_FLAG_LAYOUT_STABLE
      }
    }
  }

  /**
   * 设置屏幕方向
   * @param orientation 方向: "landscape", "portrait", "auto"
   */
  fun setOrientation(orientation: String) {
    val orientationEnum = when (orientation.lowercase()) {
      "landscape" -> android.content.pm.ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE
      "portrait" -> android.content.pm.ActivityInfo.SCREEN_ORIENTATION_PORTRAIT
      "reverse_landscape" -> android.content.pm.ActivityInfo.SCREEN_ORIENTATION_REVERSE_LANDSCAPE
      "reverse_portrait" -> android.content.pm.ActivityInfo.SCREEN_ORIENTATION_REVERSE_PORTRAIT
      else -> android.content.pm.ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED
    }
    requestedOrientation = orientationEnum
  }

  /**
   * 设置保持屏幕常亮
   * @param keepOn true=保持常亮, false=允许关闭
   */
  fun setKeepScreenOn(keepOn: Boolean) {
    if (keepOn) {
      window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    } else {
      window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    }
  }
}
