// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

package im.phnx.prototype

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage

private const val LOGTAG = "MessagingService"

class BackgroundFirebaseMessagingService : FirebaseMessagingService() {
    // Handle incoming messages from the OS
    override fun onMessageReceived(remoteMessage: RemoteMessage) {
        Log.d(LOGTAG, "onMessageReceived")
        // Check if the message contains data payload
        if (remoteMessage.data.isNotEmpty()) {
            handleDataMessage(remoteMessage.data)
        }
    }

    // Handle incoming data messages
    private fun handleDataMessage(data: Map<String, String>) {
        Log.d(LOGTAG, "handleDataMessage")
        val notificationManager =
            getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

        // Create a notification channel for Android 8.0+
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Log.d(LOGTAG, "Creating notification channel")
            val channel = NotificationChannel(
                "default_channel",
                "Default Channel",
                NotificationManager.IMPORTANCE_DEFAULT
            )
            notificationManager.createNotificationChannel(channel)
        }

        val logFilePath = cacheDir.resolve("background.log").absolutePath
        Log.d(LOGTAG, "Logging file path: $logFilePath")

        val notificationContent = IncomingNotificationContent(
            title = "",
            body = "",
            data = data["data"] ?: "",
            path = filesDir.absolutePath,
            logFilePath = cacheDir.resolve("background.log").absolutePath,
        )


        Log.d(LOGTAG, "Starting to process messages in Rust")
        val notificationBatch = NativeLib().processNewMessages(notificationContent)
        Log.d(LOGTAG, "Finished to process messages in Rust")

        // Show the notifications
        notificationBatch?.additions?.forEach {
            showNotification(notificationManager, it.identifier, it.title, it.body)
        }

        // Remove the notifications
        notificationBatch?.removals?.forEach {
            notificationManager.cancel(it, 0)
        }
    }

    private fun showNotification(notificationManager: NotificationManager, identifier: String?, title: String?, body: String?) {
        val notificationBuilder = NotificationCompat.Builder(this, "default_channel")
            .setContentTitle(title)
            .setContentText(body)
            .setSmallIcon(android.R.drawable.ic_notification_overlay) // Use your app's icon
            .setAutoCancel(true)

        notificationManager.notify(identifier, 0, notificationBuilder.build())
    }

    override fun onNewToken(token: String) {
        // Handle token refresh
        Log.w(LOGTAG, "Device token was updated")
        // TODO: The new token needs to be provisioned on the server
    }
}

