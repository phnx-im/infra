import Cocoa
import FlutterMacOS
import UserNotifications

@main
class AppDelegate: FlutterAppDelegate, UNUserNotificationCenterDelegate {
  public static let notificationChannelName: String = "im.phnx.prototype/channel"

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

    if let identifier = UUID(uuidString: response.notification.request.identifier) {
      let userInfo = response.notification.request.content.userInfo
      let conversationId = (userInfo["conversationId"] as? String).flatMap { UUID(uuidString: $0) }
      let arguments = [
        "identifier": identifier.uuidString,
        "conversationId": conversationId?.uuidString,
      ]
      notifyFlutter(method: "openedNotification", arguments: arguments)
    }

    completionHandler()
  }

  // Call Flutter by passing a method and customData as payload
  private func notifyFlutter(method: String, arguments: [String: Any?]) {
    let window = NSApplication.shared.windows.first as! MainFlutterWindow
    let controller = window.contentViewController as! FlutterViewController
    let channel = FlutterMethodChannel(
      name: Self.notificationChannelName, binaryMessenger: controller.engine.binaryMessenger)
    channel.invokeMethod(method, arguments: arguments)
  }
}
