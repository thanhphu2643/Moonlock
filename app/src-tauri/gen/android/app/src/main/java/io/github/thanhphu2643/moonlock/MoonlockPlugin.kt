package io.github.thanhphu2643.moonlock

// Android counterpart of swift/MoonlockKit: everything iOS gets from Swift, Android gets here.
//   syncCalendar  — CalendarContract (≈ EventKit): writes into a local calendar "Moonlock".
//   saveDownload  — MediaStore Downloads (≈ the iOS Files app), for the .ics export.
//   installTabBar / selectTab — Material bottom navigation (≈ the native UITabBar).
// Rust calls these via PluginHandle::run_mobile_plugin (see src/android.rs).

import android.Manifest
import android.app.Activity
import android.content.ContentUris
import android.content.ContentValues
import android.content.res.ColorStateList
import android.graphics.Canvas
import android.graphics.ColorFilter
import android.graphics.Matrix
import android.graphics.Paint
import android.graphics.Path
import android.graphics.PixelFormat
import android.graphics.drawable.Drawable
import android.graphics.drawable.StateListDrawable
import android.os.Build
import android.os.Environment
import android.provider.CalendarContract
import android.provider.MediaStore
import android.view.ContextThemeWrapper
import android.view.Gravity
import android.view.ViewGroup
import android.webkit.WebView
import android.widget.FrameLayout
import androidx.core.graphics.PathParser
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import app.tauri.PermissionState
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.google.android.material.bottomnavigation.BottomNavigationView
import com.google.android.material.navigation.NavigationBarView
import java.io.File
import java.util.Calendar
import java.util.TimeZone
import kotlin.concurrent.thread

@InvokeArg
class CalItem {
  lateinit var date: String
  lateinit var title: String
  var notes: String = ""
  var alarm_days: Int? = null
}

@InvokeArg
class SyncArgs {
  var items: Array<CalItem> = arrayOf()
}

@InvokeArg
class SaveArgs {
  lateinit var name: String
  lateinit var content: String
}

@InvokeArg
class Tab {
  lateinit var title: String
  var path: String = ""
}

@InvokeArg
class TabsArgs {
  var tabs: Array<Tab> = arrayOf()
}

@InvokeArg
class SelectArgs {
  var index: Int = 0
}

private const val CAL_NAME = "Moonlock"
private const val TINT = 0xFF5C99FF.toInt()        // same blue as the iOS tab bar tint
private const val MUTED = 0xFF8D97A8.toInt()
private const val BAR_BG = 0xFF1E2430.toInt()

@TauriPlugin(
  permissions = [
    Permission(strings = [Manifest.permission.READ_CALENDAR, Manifest.permission.WRITE_CALENDAR], alias = "calendar"),
  ],
)
class MoonlockPlugin(private val activity: Activity) : Plugin(activity) {
  private var webView: WebView? = null
  private var bar: BottomNavigationView? = null

  override fun load(webView: WebView) {
    this.webView = webView
  }

  // ───────────────────────────── Calendar ─────────────────────────────

  @Command
  fun syncCalendar(invoke: Invoke) {
    if (getPermissionState("calendar") == PermissionState.GRANTED) doSync(invoke)
    else requestPermissionForAlias("calendar", invoke, "calendarPermsCallback")
  }

  @PermissionCallback
  private fun calendarPermsCallback(invoke: Invoke) {
    if (getPermissionState("calendar") == PermissionState.GRANTED) doSync(invoke)
    else invoke.reject("Chưa cấp quyền Lịch. Mở Cài đặt → Ứng dụng → Moonlock → Quyền → Lịch.", "DENIED")
  }

  private fun doSync(invoke: Invoke) {
    val items = invoke.parseArgs(SyncArgs::class.java).items
    thread {
      try {
        val cal = ownCalendar() ?: return@thread invoke.reject("Không tạo được lịch \"Moonlock\".", "NO_CALENDAR")
        var created = 0
        for (item in items) {
          // All-day events must be stored at UTC midnight with the UTC timezone.
          val (y, m, d) = item.date.split("-").map { it.toInt() }
          val start = Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply { clear(); set(y, m - 1, d) }.timeInMillis
          if (exists(cal, start, item.title)) continue // idempotent, like the Swift side
          val v = ContentValues().apply {
            put(CalendarContract.Events.CALENDAR_ID, cal)
            put(CalendarContract.Events.TITLE, item.title)
            put(CalendarContract.Events.DESCRIPTION, item.notes)
            put(CalendarContract.Events.DTSTART, start)
            put(CalendarContract.Events.DTEND, start + 86_400_000L)
            put(CalendarContract.Events.ALL_DAY, 1)
            put(CalendarContract.Events.EVENT_TIMEZONE, "UTC")
            put(
              CalendarContract.Events.AVAILABILITY,
              if (item.alarm_days == null) CalendarContract.Events.AVAILABILITY_BUSY else CalendarContract.Events.AVAILABILITY_FREE,
            )
          }
          val uri = activity.contentResolver.insert(CalendarContract.Events.CONTENT_URI, v) ?: continue
          val days = item.alarm_days
          if (days != null && days > 0) {
            // 09:00 the day(s) before, same as the EKAlarm on iOS.
            activity.contentResolver.insert(CalendarContract.Reminders.CONTENT_URI, ContentValues().apply {
              put(CalendarContract.Reminders.EVENT_ID, ContentUris.parseId(uri))
              put(CalendarContract.Reminders.MINUTES, days * 1440 - 9 * 60)
              put(CalendarContract.Reminders.METHOD, CalendarContract.Reminders.METHOD_ALERT)
            })
          }
          created++
        }
        invoke.resolve(JSObject().put("count", created))
      } catch (e: Exception) {
        invoke.reject("Lỗi ghi lịch: ${e.message}", "FAILED", e)
      }
    }
  }

  private fun ownCalendar(): Long? {
    val cr = activity.contentResolver
    cr.query(
      CalendarContract.Calendars.CONTENT_URI,
      arrayOf(CalendarContract.Calendars._ID),
      "${CalendarContract.Calendars.ACCOUNT_TYPE} = ? AND ${CalendarContract.Calendars.NAME} = ?",
      arrayOf(CalendarContract.ACCOUNT_TYPE_LOCAL, CAL_NAME),
      null,
    )?.use { if (it.moveToFirst()) return it.getLong(0) }

    // Only a sync adapter may create calendars; a LOCAL account needs no real adapter.
    val uri = CalendarContract.Calendars.CONTENT_URI.buildUpon()
      .appendQueryParameter(CalendarContract.CALLER_IS_SYNCADAPTER, "true")
      .appendQueryParameter(CalendarContract.Calendars.ACCOUNT_NAME, CAL_NAME)
      .appendQueryParameter(CalendarContract.Calendars.ACCOUNT_TYPE, CalendarContract.ACCOUNT_TYPE_LOCAL)
      .build()
    val v = ContentValues().apply {
      put(CalendarContract.Calendars.ACCOUNT_NAME, CAL_NAME)
      put(CalendarContract.Calendars.ACCOUNT_TYPE, CalendarContract.ACCOUNT_TYPE_LOCAL)
      put(CalendarContract.Calendars.NAME, CAL_NAME)
      put(CalendarContract.Calendars.CALENDAR_DISPLAY_NAME, CAL_NAME)
      put(CalendarContract.Calendars.CALENDAR_COLOR, 0xFFDB2E38.toInt())
      put(CalendarContract.Calendars.CALENDAR_ACCESS_LEVEL, CalendarContract.Calendars.CAL_ACCESS_OWNER)
      put(CalendarContract.Calendars.OWNER_ACCOUNT, CAL_NAME)
      put(CalendarContract.Calendars.VISIBLE, 1)
      put(CalendarContract.Calendars.SYNC_EVENTS, 1)
      put(CalendarContract.Calendars.CALENDAR_TIME_ZONE, TimeZone.getDefault().id)
    }
    return cr.insert(uri, v)?.let { ContentUris.parseId(it) }
  }

  private fun exists(cal: Long, start: Long, title: String): Boolean =
    activity.contentResolver.query(
      CalendarContract.Events.CONTENT_URI,
      arrayOf(CalendarContract.Events._ID),
      "${CalendarContract.Events.CALENDAR_ID} = ? AND ${CalendarContract.Events.DTSTART} = ? AND " +
        "${CalendarContract.Events.TITLE} = ? AND ${CalendarContract.Events.DELETED} = 0",
      arrayOf(cal.toString(), start.toString(), title),
      null,
    )?.use { it.count > 0 } ?: false

  // ───────────────────────────── Downloads ─────────────────────────────

  @Command
  fun saveDownload(invoke: Invoke) {
    val args = invoke.parseArgs(SaveArgs::class.java)
    try {
      val bytes = args.content.toByteArray()
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
        val cr = activity.contentResolver
        val col = MediaStore.Downloads.EXTERNAL_CONTENT_URI
        // Replace our previous export instead of piling up "moonlock (3).ics".
        runCatching { cr.delete(col, "${MediaStore.Downloads.DISPLAY_NAME} = ?", arrayOf(args.name)) }
        val uri = cr.insert(col, ContentValues().apply {
          put(MediaStore.Downloads.DISPLAY_NAME, args.name)
          put(MediaStore.Downloads.MIME_TYPE, "text/calendar")
        }) ?: return invoke.reject("Không tạo được file trong Downloads.")
        cr.openOutputStream(uri)?.use { it.write(bytes) }
        invoke.resolve(JSObject().put("path", "${Environment.DIRECTORY_DOWNLOADS}/${args.name}"))
      } else {
        @Suppress("DEPRECATION")
        val dir = Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS)
        val file = runCatching { File(dir, args.name).apply { writeBytes(bytes) } }
          .getOrElse { File(activity.getExternalFilesDir(null), args.name).apply { writeBytes(bytes) } }
        invoke.resolve(JSObject().put("path", file.absolutePath))
      }
    } catch (e: Exception) {
      invoke.reject("Không lưu được file: ${e.message}", e)
    }
  }

  // ───────────────────────────── Tab bar ─────────────────────────────

  @Command
  fun installTabBar(invoke: Invoke) {
    val tabs = invoke.parseArgs(TabsArgs::class.java).tabs
    activity.runOnUiThread {
      if (bar == null) {
        val wv = webView ?: return@runOnUiThread invoke.resolve(JSObject().put("ok", false))
        val ctx = ContextThemeWrapper(activity, com.google.android.material.R.style.Theme_Material3_Dark_NoActionBar)
        val nav = BottomNavigationView(ctx).apply {
          setBackgroundColor(BAR_BG)
          labelVisibilityMode = NavigationBarView.LABEL_VISIBILITY_LABELED
          itemIconTintList = null // icons carry their own checked/unchecked colours
          itemTextColor = ColorStateList(arrayOf(intArrayOf(android.R.attr.state_checked), intArrayOf()), intArrayOf(TINT, MUTED))
          itemActiveIndicatorColor = ColorStateList.valueOf(0x335C99FF)
          tabs.forEachIndexed { i, t -> menu.add(0, i, i, t.title).icon = tabIcon(t.path) }
          setOnItemSelectedListener { item ->
            wv.evaluateJavascript("window.__nativeTab && window.__nativeTab(${item.itemId})", null)
            true
          }
        }
        // MainActivity already pads the content for the gesture bar, so no extra insets here.
        ViewCompat.setOnApplyWindowInsetsListener(nav) { _, _ -> WindowInsetsCompat.CONSUMED }
        val content = activity.findViewById<FrameLayout>(android.R.id.content)
        content.addView(nav, FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT, Gravity.BOTTOM))
        // Shrink the webview so page content never hides behind the bar.
        nav.post {
          (wv.layoutParams as? ViewGroup.MarginLayoutParams)?.let {
            it.bottomMargin = nav.height
            wv.layoutParams = it
          }
        }
        bar = nav
      }
      invoke.resolve(JSObject().put("ok", true))
    }
  }

  @Command
  fun selectTab(invoke: Invoke) {
    val index = invoke.parseArgs(SelectArgs::class.java).index
    activity.runOnUiThread {
      bar?.menu?.findItem(index)?.let { if (!it.isChecked) it.isChecked = true }
      invoke.resolve()
    }
  }

  private fun tabIcon(path: String): Drawable = StateListDrawable().apply {
    addState(intArrayOf(android.R.attr.state_checked), StrokeIcon(path, TINT))
    addState(intArrayOf(), StrokeIcon(path, MUTED))
  }
}

/** A Lucide icon (24×24 viewBox, 2px round strokes) drawn from its SVG path data. */
private class StrokeIcon(pathData: String, color: Int) : Drawable() {
  private val src = Path().apply {
    pathData.split("|").forEach { d -> runCatching { addPath(PathParser.createPathFromPathData(d)) } }
  }
  private val path = Path()
  private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    style = Paint.Style.STROKE
    strokeCap = Paint.Cap.ROUND
    strokeJoin = Paint.Join.ROUND
    this.color = color
  }
  private val size = (24 * android.content.res.Resources.getSystem().displayMetrics.density).toInt()

  override fun getIntrinsicWidth() = size
  override fun getIntrinsicHeight() = size

  override fun draw(canvas: Canvas) {
    val s = bounds.width() / 24f
    path.reset()
    src.transform(Matrix().apply { setScale(s, s); postTranslate(bounds.left.toFloat(), bounds.top.toFloat()) }, path)
    paint.strokeWidth = 2f * s
    canvas.drawPath(path, paint)
  }

  override fun setAlpha(alpha: Int) { paint.alpha = alpha }
  override fun setColorFilter(colorFilter: ColorFilter?) { paint.colorFilter = colorFilter }
  @Deprecated("Deprecated in Java")
  override fun getOpacity() = PixelFormat.TRANSLUCENT
}
