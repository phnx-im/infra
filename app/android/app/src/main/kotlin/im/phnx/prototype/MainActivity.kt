// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

package im.phnx.prototype

import android.content.ContentValues.TAG
import android.content.Intent
import android.os.Bundle
import com.google.android.gms.tasks.Task
import com.google.firebase.messaging.FirebaseMessaging
import io.flutter.Log
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel


class MainActivity : FlutterActivity() {
    companion object {
        private const val CHANNEL_NAME: String = "im.phnx.prototype/channel"

        const val SELECT_NOTIFICATION: String = "SELECT_NOTIFICATION"
        const val INTENT_EXTRA_NOTIFICATION_ID: String = "im.phnx.prototype/notification_id"

        var instance: MainActivity? = null
    }

    private var channel: MethodChannel? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        instance = this
        NativeLib.registerJavaVm(JniNotifications)
    }

    override fun onDestroy() {
        instance = null
        super.onDestroy()
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)

        if (intent.action == SELECT_NOTIFICATION) {
            val arguments = mapOf(
                "identifier" to intent.extras?.getString(
                    INTENT_EXTRA_NOTIFICATION_ID
                )
            )
            channel?.invokeMethod("openedNotification", arguments)
        }
    }


    // Configures the Method Channel to communicate with Flutter
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        channel = MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            CHANNEL_NAME
        )
        channel?.setMethodCallHandler { call, result ->
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

    override fun detachFromFlutterEngine() {
        super.detachFromFlutterEngine()

        channel?.setMethodCallHandler(null)
        channel = null
    }
}

