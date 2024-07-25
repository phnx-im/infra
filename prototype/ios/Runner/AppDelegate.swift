import Flutter
import UIKit

@UIApplicationMain
@objc class AppDelegate: FlutterAppDelegate {
  private var deviceToken: String?
  
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
    let deviceTokenChannel = FlutterMethodChannel(name: "im.phnx.prototype/channel",
                                                  binaryMessenger: controller.binaryMessenger)
    deviceTokenChannel.setMethodCallHandler { [weak self] (call, result) in
      if call.method == "devicetoken" {
        self?.getDeviceToken(result: result)
      } else {
        result(FlutterMethodNotImplemented)
      }
    }

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
    print("Failed to register: \(error)")
  }
  
  private func getDeviceToken(result: FlutterResult) {
    if let token = deviceToken {
      result(token)
    } else {
      result(FlutterError(code: "UNAVAILABLE", message: "Device token not available", details: nil))
    }
  }
}
