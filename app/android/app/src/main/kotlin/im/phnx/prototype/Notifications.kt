// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

package im.phnx.prototype

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.util.Log
import androidx.core.app.ActivityCompat
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.app.TaskStackBuilder
import kotlinx.serialization.*
import kotlinx.serialization.json.*

private const val LOGTAG = "NativeLib"

@Serializable
data class IncomingNotificationContent(
    val title: String,
    val body: String,
    val data: String,
    val path: String,
    val logFilePath: String,
)

@Serializable
data class NotificationContent(
    val identifier: String,
    val title: String,
    val body: String,
    val data: String
)

@Serializable
data class NotificationBatch(
    val badgeCount: Int,
    val removals: List<String>,
    val additions: List<NotificationContent>
)

class NativeLib {
    companion object {
        // Load the shared library
        init {
            System.loadLibrary("phnxapplogic")
        }

        // Declare the native method
        @JvmStatic
        external fun process_new_messages(content: String): String

        @JvmStatic
        external fun registerJavaVm(jniNotificationClass: Any)
    }

    // Wrapper to process new messages. Handles JSON
    // serialization/deserialization and memory cleanup.
    fun processNewMessages(input: IncomingNotificationContent): NotificationBatch? {
        Log.d(LOGTAG, "handleDataMessage")
        // Serialize input data to JSON
        val jsonInput = Json.encodeToString(IncomingNotificationContent.serializer(), input)

        // Call the Rust function
        val rawOutput: String
        try {
            rawOutput = process_new_messages(jsonInput)
        } catch (e: Exception) {
            Log.e(LOGTAG, "Error calling native function: ${e.message}")
            return null
        }

        // Deserialize the output JSON back into NotificationBatch
        val result: NotificationBatch = try {
            Json.decodeFromString(NotificationBatch.serializer(), rawOutput)
        } catch (e: Exception) {
            Log.e(LOGTAG, "Error decoding response JSON: ${e.message}")
            return null
        }

        return result
    }
}

class JniNotifications {
    companion object JniNotifications {
        private const val CHANNEL_ID = "Conversations"
        private const val NOTIFICATION_ID = 0;

        @JvmStatic
        fun showNotification(identifier: String, title: String, message: String) {
            try {
                val activity = MainActivity.instance ?: return
                val notificationManager =
                    activity.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

                // Create notification channel (needed for Android 8+)
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    val channel = NotificationChannel(
                        CHANNEL_ID, "Conversations", NotificationManager.IMPORTANCE_HIGH
                    )
                    notificationManager.createNotificationChannel(channel)
                }

                if (ActivityCompat.checkSelfPermission(
                        activity, Manifest.permission.POST_NOTIFICATIONS
                    ) != PackageManager.PERMISSION_GRANTED
                ) {
                    return
                }


                val intent = Intent(activity, MainActivity::class.java)
                intent.action = MainActivity.SELECT_NOTIFICATION
                intent.putExtra(MainActivity.INTENT_EXTRA_NOTIFICATION_ID, identifier)

                val pendingIntent = PendingIntent.getActivity(
                    activity,
                    1,
                    intent,
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
                )

                val notification =
                    NotificationCompat.Builder(activity, CHANNEL_ID)
                        .setContentTitle(title)
                        .setContentText(message)
                        .setSmallIcon(android.R.drawable.ic_notification_overlay)
                        .setContentIntent(pendingIntent)
                        .setDefaults(Notification.DEFAULT_ALL)
                        .setPriority(NotificationManagerCompat.IMPORTANCE_HIGH)
                        .build()
                NotificationManagerCompat.from(activity)
                    .notify(identifier, NOTIFICATION_ID, notification)
            } catch (exception: Exception) {
                Log.e(LOGTAG, "failed to show notification: $exception")
            }
        }

        @JvmStatic
        fun getActiveNotifications(): ArrayList<String> {
            try {
                val activity = MainActivity.instance ?: return ArrayList()
                val notificationManager =
                    activity.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
                val activeNotifications =
                    NotificationManagerCompat.from(activity).activeNotifications
                return activeNotifications.mapNotNull { notification -> notification.tag }
                    .toCollection(ArrayList())
            } catch (exception: Exception) {
                Log.e(LOGTAG, "failed to get active notifications: $exception")
            }
            return ArrayList()
        }

        @JvmStatic
        fun cancelNotifications(identifiers: ArrayList<String>) {
            try {
                val activity = MainActivity.instance ?: return
                val notificationManager =
                    activity.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
                for (identifier in identifiers) {
                    notificationManager.cancel(identifier, NOTIFICATION_ID)
                }
            } catch (exception: Exception) {
                Log.e(LOGTAG, "failed to cancel notifications: $exception")
            }
        }
    }
}