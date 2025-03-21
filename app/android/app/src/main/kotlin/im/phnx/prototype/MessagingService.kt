// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

package im.phnx.prototype

import android.util.Log
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
        notificationBatch?.additions?.forEach { content ->
            Notifications.showNotification(this, content)
        }

        // Remove the notifications
        if (notificationBatch?.removals != null) {
            Notifications.cancelNotifications(this, ArrayList(notificationBatch.removals))
        }
    }

    override fun onNewToken(token: String) {
        // Handle token refresh
        Log.w(LOGTAG, "Device token was updated")
        // TODO: The new token needs to be provisioned on the server
    }
}
