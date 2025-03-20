// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

package im.phnx.prototype

import android.content.ContentValues.TAG
import android.content.Intent
import com.google.android.gms.tasks.Task
import com.google.firebase.messaging.FirebaseMessaging
import io.flutter.Log
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    companion object {
        private const val CHANNEL_NAME: String = "im.phnx.prototype/channel"
    }

    private var channel: MethodChannel? = null

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)

        if (intent.action == Notifications.SELECT_NOTIFICATION) {
            val notificationId = intent.extras?.getString(Notifications.EXTRAS_NOTIFICATION_ID_KEY)
            val conversationId = intent.extras?.getString(Notifications.EXTRAS_CONVERSATION_ID_KEY)
            if (notificationId != null) {
                val arguments = mapOf(
                    "identifier" to notificationId, "conversationId" to conversationId
                )
                channel?.invokeMethod("openedNotification", arguments)
            }
        }
    }

    override fun detachFromFlutterEngine() {
        super.detachFromFlutterEngine()

        channel?.setMethodCallHandler(null)
        channel = null
    }

    // Configures the Method Channel to communicate with Flutter
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        channel = MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger, CHANNEL_NAME
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

                "sendNotification" -> {
                    val identifier: String? = call.argument("identifier")
                    val title: String? = call.argument("title")
                    val body: String? = call.argument("body")
                    val conversationId: String? = call.argument("conversationId")

                    if (identifier != null && title != null && body != null) {
                        val notification =
                            NotificationContent(identifier, title, body, conversationId)
                        Notifications.showNotification(this, notification)
                        result.success(null)
                    } else {
                        result.error(
                            "DeserializeError",
                            "Failed to decode notification arguments ${call.arguments}",
                            ""
                        )
                    }
                }

                "getActiveNotifications" -> {
                    val notifications = Notifications.getActiveNotifications(this)
                    val res: ArrayList<Map<String, Any?>> = ArrayList(notifications.map { handle ->
                        mapOf<String, Any?>(
                            "identifier" to handle.notificationId,
                            "conversationId" to handle.conversationId
                        )
                    })
                    result.success(res)
                }

                "cancelNotifications" -> {
                    val identifiers: ArrayList<String>? = call.argument("identifiers")
                    if (identifiers != null) {
                        Notifications.cancelNotifications(this, identifiers)
                    } else {
                        result.error(
                            "DeserializeError", "Failed to decode 'identifiers' arguments", ""
                        )
                    }
                }

                else -> {
                    result.notImplemented()
                }
            }
        }
    }

}

