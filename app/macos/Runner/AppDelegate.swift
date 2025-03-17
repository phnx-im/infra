import Cocoa
import FlutterMacOS
import UserNotifications

@main
class AppDelegate: FlutterAppDelegate, UNUserNotificationCenterDelegate {
  private let notificationChannelName: String = "im.phnx.prototype/channel"

  override func applicationDidFinishLaunching(_ notification: Notification) {
    let center = UNUserNotificationCenter.current()
    center.delegate = self
    super.applicationDidFinishLaunching(notification)
  }

  override func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
    return true
  }

  override func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool {
    return true
  }

  // This method will be called when the user taps on the notification
  func userNotificationCenter(
    _ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse,
    withCompletionHandler completionHandler: @escaping () -> Void
  ) {
    NSLog("User opened notification")

    NSApp.activate(ignoringOtherApps: true)

    let identifier = response.notification.request.identifier
    let userInfo = response.notification.request.content.userInfo
    let customData = userInfo["customData"] as? String

    notifyFlutter(method: "openedNotification", identifier: identifier, customData: customData)

    completionHandler()
  }

  // Call Flutter by passing a method and customData as payload
  private func notifyFlutter(method: String, identifier: String, customData: String?) {
    let window = NSApplication.shared.windows.first as! MainFlutterWindow
    let controller = window.contentViewController as! FlutterViewController
    let channel = FlutterMethodChannel(
      name: notificationChannelName, binaryMessenger: controller.engine.binaryMessenger)
    let arguments: [String: String] = ["identifier": identifier, "customData": customData ?? ""]
    channel.invokeMethod(method, arguments: arguments)
  }
}
