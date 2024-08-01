import Flutter
import UIKit

@UIApplicationMain
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
    
    private func notifyFlutter(customData: String, method: String) {
        let controller = window?.rootViewController as! FlutterViewController
        let channel = FlutterMethodChannel(name: notificationChannelName, binaryMessenger: controller.binaryMessenger)
        let arguments: [String: String] = ["customData": customData]
        channel.invokeMethod(method, arguments: arguments)
    }
    
    // Define the handler function
    private func handleMethodCall(call: FlutterMethodCall, result: @escaping FlutterResult) {
        if call.method == "devicetoken" {
            self.getDeviceToken(result: result)
        } else if call.method == "getSharedDocumentsDirectory" {
            self.getSharedDocumentsDirectory(result: result)
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
            let documentsPath = containerURL.appendingPathComponent("Documents").path
            result(documentsPath)
        } else {
            result(FlutterError(code: "UNAVAILABLE",
                                message: "App group container not found",
                                details: nil))
        }
    }
}
