package org.grt.bible

import android.graphics.Color
import android.os.Bundle
import android.print.PrintAttributes
import android.print.PrintManager
import android.view.View
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // The page stays clear of the status bar, the navigation bar and the keyboard.
    ViewCompat.setOnApplyWindowInsetsListener(findViewById<View>(android.R.id.content)) { view, insets ->
      val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
      val keyboard = insets.getInsets(WindowInsetsCompat.Type.ime())
      view.setPadding(bars.left, bars.top, bars.right, maxOf(bars.bottom, keyboard.bottom))
      WindowInsetsCompat.CONSUMED
    }
  }

  override fun onWebViewCreate(webView: WebView) {
    webView.addJavascriptInterface(SystemBars(), "grtSystemBars")
    webView.addJavascriptInterface(Printer(webView), "grtPrint")
  }

  /** Hands the page to the system's print service, which also saves it as a PDF. */
  inner class Printer(private val webView: WebView) {
    @JavascriptInterface
    fun print(title: String, paper: String) {
      val media = when (paper) {
        "a5" -> PrintAttributes.MediaSize.ISO_A5
        "letter" -> PrintAttributes.MediaSize.NA_LETTER
        else -> PrintAttributes.MediaSize.ISO_A4
      }
      runOnUiThread {
        val manager = getSystemService(PRINT_SERVICE) as PrintManager
        val name = title.take(80).ifBlank { "GRT Bible" }
        val attributes = PrintAttributes.Builder().setMediaSize(media).build()
        manager.print(name, webView.createPrintDocumentAdapter(name), attributes)
      }
    }
  }

  /** Lets the page paint the strips behind the system bars in the colours of its theme. */
  inner class SystemBars {
    @JavascriptInterface
    fun paint(color: String, dark: Boolean) {
      val parsed = runCatching { Color.parseColor(color) }.getOrNull() ?: return
      runOnUiThread {
        window.decorView.setBackgroundColor(parsed)
        WindowCompat.getInsetsController(window, window.decorView).apply {
          isAppearanceLightStatusBars = !dark
          isAppearanceLightNavigationBars = !dark
        }
      }
    }
  }
}
