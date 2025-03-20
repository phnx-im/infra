import Flutter
import UIKit

@main
@objc class AppDelegate: FlutterAppDelegate {
  private var deviceToken: String?
  private let notificationChannelName: String = "im.phnx.prototype/channel"

  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    GeneratedPluginRegistrant.register(with: self)

    if #available(iOS 10.0, *) {
      UNUserNotificationCenter.current().delegate = self
    }

    // Register for push notifications
    UIApplication.shared.registerForRemoteNotifications()

    // Set up the method channel to retrieve the token from Flutter
    let controller = window?.rootViewController as! FlutterViewController
    let methodChannel = FlutterMethodChannel(
      name: notificationChannelName,
      binaryMessenger: controller.binaryMessenger)

    // Set the handler function for the method channel
    methodChannel.setMethodCallHandler(handleMethodCall)

    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  override func application(
    _ application: UIApplication, didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data
  ) {
    NSLog("Device token available")
    let tokenParts = deviceToken.map { data in String(format: "%02.2hhx", data) }
    let token = tokenParts.joined()

    // Save the token in memory
    self.deviceToken = token
  }

  override func application(
    _ application: UIApplication, didFailToRegisterForRemoteNotificationsWithError error: Error
  ) {
    NSLog("Failed to register: \(error)")
  }

  // This method will be called when app received push notifications in foreground
  override func userNotificationCenter(
    _ center: UNUserNotificationCenter, willPresent notification: UNNotification,
    withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
  ) {
    NSLog("Foreground notification received")
    if let handle = NotificationHandle.init(notification: notification) {
      notifyFlutter(method: "receivedNotification", arguments: handle.toDict())
    }
    completionHandler([.alert, .sound])
  }

  // This method will be called when the user taps on the notification
  override func userNotificationCenter(
    _ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse,
    withCompletionHandler completionHandler: @escaping () -> Void
  ) {
    NSLog("User opened notification")
    if let handle = NotificationHandle.init(notification: response.notification) {
      notifyFlutter(method: "openedNotification", arguments: handle.toDict())
    }
    completionHandler()
  }

  // Call Flutter by passing a method and customData as payload
  private func notifyFlutter(method: String, arguments: [String: Any?]) {
    let controller = window?.rootViewController as! FlutterViewController
    let channel = FlutterMethodChannel(
      name: notificationChannelName, binaryMessenger: controller.binaryMessenger)
    channel.invokeMethod(method, arguments: arguments)
  }

  // Define the handler function
  private func handleMethodCall(call: FlutterMethodCall, result: @escaping FlutterResult) {
    if call.method == "getDeviceToken" {
      self.getDeviceToken(result: result)
    } else if call.method == "getDatabasesDirectory" {
      self.getSharedDocumentsDirectory(result: result)
    } else if call.method == "setBadgeCount" {
      if let args = call.arguments as? [String: Any], let count = args["count"] as? Int {
        self.setBadgeCount(count, result: result)
      } else {
        result(
          FlutterError(
            code: "INVALID_ARGUMENT", message: "Invalid or missing arguments", details: nil))
      }
    } else if call.method == "sendNotification" {
      if let args = call.arguments as? [String: Any?],
        let identifierStr = args["identifier"] as? String,
        let identifier = UUID(uuidString: identifierStr),
        let title = args["title"] as? String,
        let body = args["body"] as? String,
        let conversationIdStr = args["conversationId"] as? String?
      {
        sendNotification(
          identifier: identifier,
          title: title,
          body: body,
          conversationId: conversationIdStr.flatMap { UUID(uuidString: $0) })
        result(nil)
      } else {
        result(
          FlutterError(
            code: "DecodingError",
            message: "Failed to decode sendNotifications arguments",
            details: nil))
      }
    } else if call.method == "getActiveNotifications" {
      getActiveNotifications { handles in
        result(handles.map { $0.toDict() })
      }
    } else if call.method == "cancelNotifications" {
      if let args = call.arguments as? [String: Any?],
        let identifiers = args["identifiers"] as? [String]
      {
        let ids = identifiers.compactMap { UUID(uuidString: $0) }
        cancelNotifications(identifiers: ids)
        result(nil)
      } else {
        result(
          FlutterError(
            code: "DecodingError",
            message: "Failed to decode cancelNotifications arguments",
            details: nil))
      }
    } else {
      NSLog("Unknown method called: \(call.method)")
      result(FlutterMethodNotImplemented)
    }

  }

  // Get device token
  private func getDeviceToken(result: FlutterResult) {
    if let token = deviceToken {
      result(token)
    } else {
      result(FlutterError(code: "UNAVAILABLE", message: "Device token not available", details: nil))
    }
  }

  // Get the shared documents path
  private func getSharedDocumentsDirectory(result: FlutterResult) {
    if let containerURL = FileManager.default.containerURL(
      forSecurityApplicationGroupIdentifier: "group.im.phnx.prototype")
    {
      let documentsURL = containerURL.appendingPathComponent("Documents")
      // Create the "Documents" directory if it doesn't exist
      let fileManager = FileManager.default
      if !fileManager.fileExists(atPath: documentsURL.path) {
        do {
          try fileManager.createDirectory(
            at: documentsURL, withIntermediateDirectories: true, attributes: nil)
        } catch {
          print("Failed to create Documents directory: \(error)")
        }
      }
      result(documentsURL.path)
    } else {
      result(
        FlutterError(
          code: "UNAVAILABLE",
          message: "App group container not found",
          details: nil))
    }
  }

  // Set the badge count
  private func setBadgeCount(_ count: Int, result: FlutterResult) {
    UIApplication.shared.applicationIconBadgeNumber = count
    result(nil)
  }
}

func sendNotification(identifier: UUID, title: String, body: String, conversationId: UUID?) {
  let center = UNUserNotificationCenter.current()

  let content = UNMutableNotificationContent()
  content.title = title
  content.body = body
  content.sound = UNNotificationSound.default
  content.userInfo["conversationId"] = conversationId?.uuidString

  let request = UNNotificationRequest(
    identifier: identifier.uuidString,
    content: content,
    trigger: nil)

  center.add(request) { error in
    if let error = error {
      NSLog("NSE Error adding notification: \(error)")
    }
  }
}

struct NotificationHandle {
  let identifier: UUID
  let conversationId: UUID?

  init?(notification: UNNotification) {
    let identifierStr = notification.request.identifier
    guard let identifier = UUID(uuidString: identifierStr) else {
      return nil
    }
    self.identifier = identifier
    let conversationIdStr: String? =
      notification.request.content.userInfo["conversationId"] as? String? ?? nil
    self.conversationId = conversationIdStr.flatMap { UUID(uuidString: $0) }
  }

  func toDict() -> [String: Any?] {
    [
      "identifier": identifier.uuidString,
      "conversationId": conversationId?.uuidString,
    ]
  }
}

func getActiveNotifications(completionHandler: @escaping ([NotificationHandle]) -> Void) {
  let center = UNUserNotificationCenter.current()
  center.getDeliveredNotifications { notifications in
    completionHandler(
      notifications.compactMap {
        NotificationHandle(notification: $0)
      })
  }
}

func cancelNotifications(identifiers: [UUID]) {
  let center = UNUserNotificationCenter.current()
  center.removeDeliveredNotifications(
    withIdentifiers: identifiers.map {
      $0.uuidString
    })
}
