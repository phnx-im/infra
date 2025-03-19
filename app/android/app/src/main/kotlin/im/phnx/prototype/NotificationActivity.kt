package im.phnx.prototype

import android.app.Activity
import android.os.Bundle
import android.util.Log

private const val LOGTAG = "NotificationActivity"

class NotificationActivity: Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        if (intent != null) {
            Log.i(LOGTAG, "handling intent")
        }
    }
}