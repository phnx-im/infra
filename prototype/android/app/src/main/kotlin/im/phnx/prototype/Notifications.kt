// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

package im.phnx.prototype

import android.util.Log
import kotlinx.serialization.*
import kotlinx.serialization.json.*

private const val LOGTAG = "NativeLib"

@Serializable
data class IncomingNotificationContent(
    val title: String,
    val body: String,
    val data: String,
    val path: String
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
    val badge_count: Int,
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
    }

    // Wrapper to process new messages. Handles JSON
    // serialization/deserialization and memory cleanup.
    public fun processNewMessages(input: IncomingNotificationContent): NotificationBatch? {
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
