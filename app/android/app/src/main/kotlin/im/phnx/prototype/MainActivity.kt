// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

package im.phnx.prototype

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.ContentValues.TAG
import android.content.Context
import android.os.Build
import androidx.core.app.NotificationCompat
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import com.google.android.gms.tasks.Task
import com.google.firebase.messaging.FirebaseMessaging
import io.flutter.Log
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    private val channel: String = "im.phnx.prototype/channel"

    // Configures the Method Channel to communicate with Flutter
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            channel
        ).setMethodCallHandler { call, result ->
            when (call.method) {
                "getDeviceToken" -> {
                    FirebaseMessaging.getInstance().token.addOnCompleteListener { task: Task<String> ->
                        if (task.isSuccessful) {
                            val token = task.result
                            result.success(token)
                        } else {
                            Log.w(TAG, "Fetching FCM registration token failed" + task.exception)
                            result.error("NoDeviceToken", "Device token not available", "")
                        }
                    }
                }
                "getDatabasesDirectory" -> {
                    val databasePath = filesDir.absolutePath
                    Log.d(TAG, "Application database path: $databasePath")
                    result.success(databasePath)
                }
                else -> {
                    result.notImplemented()
                }
            }
        }
    }
}

