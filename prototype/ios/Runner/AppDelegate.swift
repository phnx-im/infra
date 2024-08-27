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
        let methodChannel = FlutterMethodChannel(name: notificationChannelName,
                                                 binaryMessenger: controller.binaryMessenger)
        
        // Set the handler function for the method channel
        methodChannel.setMethodCallHandler(handleMethodCall)
        
        return super.application(application, didFinishLaunchingWithOptions: launchOptions)
    }
    
    override func application(_ application: UIApplication, didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data) {
        NSLog("Device token available")
        let tokenParts = deviceToken.map { data in String(format: "%02.2hhx", data) }
        let token = tokenParts.joined()
        
        // Save the token in memory
        self.deviceToken = token
    }
    
    override func application(_ application: UIApplication, didFailToRegisterForRemoteNotificationsWithError error: Error) {
        NSLog("Failed to register: \(error)")
    }
    
    // This method will be called when app received push notifications in foreground
    override func userNotificationCenter(_ center: UNUserNotificationCenter, willPresent notification: UNNotification, withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void) {
        NSLog("Foreground notification received")
        let userInfo = notification.request.content.userInfo
        if let customData = userInfo["customData"] as? String {
            notifyFlutter(customData: customData, method: "receivedNotification")
        }
        completionHandler([.alert, .sound])
    }
    
    // This method will be called when the user taps on the notification
    override func userNotificationCenter(_ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse, withCompletionHandler completionHandler: @escaping () -> Void) {
        NSLog("User opened notification")
        let userInfo = response.notification.request.content.userInfo
        if let customData = userInfo["customData"] as? String {
            notifyFlutter(customData: customData, method: "openedNotification")
        }
        completionHandler()
    }
    
    // Call Flutter by passing a method and customData as payload
    private func notifyFlutter(customData: String, method: String) {
        let controller = window?.rootViewController as! FlutterViewController
        let channel = FlutterMethodChannel(name: notificationChannelName, binaryMessenger: controller.binaryMessenger)
        let arguments: [String: String] = ["customData": customData]
        channel.invokeMethod(method, arguments: arguments)
    }
    
    // Define the handler function
    private func handleMethodCall(call: FlutterMethodCall, result: @escaping FlutterResult) {
        if call.method == "getDeviceToken" {
            self.getDeviceToken(result: result)
        } else if call.method == "getSharedDocumentsDirectory" {
            self.getSharedDocumentsDirectory(result: result)
        }
        else if call.method == "setBadgeCount" {
            if let args = call.arguments as? [String: Any], let count = args["count"] as? Int {
                self.setBadgeCount(count, result: result)
            } else {
                result(FlutterError(code: "INVALID_ARGUMENT", message: "Invalid or missing arguments", details: nil))
            }
        }
        else {
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
        if let containerURL = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: "group.im.phnx.prototype") {
            let documentsURL = containerURL.appendingPathComponent("Documents")
            // Create the "Documents" directory if it doesn't exist
            let fileManager = FileManager.default
            if !fileManager.fileExists(atPath: documentsURL.path) {
                do {
                    try fileManager.createDirectory(at: documentsURL, withIntermediateDirectories: true, attributes: nil)
                } catch {
                    print("Failed to create Documents directory: \(error)")
                }
            }
            result(documentsURL.path)
        } else {
            result(FlutterError(code: "UNAVAILABLE",
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
