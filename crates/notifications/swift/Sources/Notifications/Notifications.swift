import Foundation
import UserNotifications
import os

@available(macOS 11, iOS 13.0, *)
let log = Logger(subsystem: "Notifications", category: "ffi")

@_cdecl("notifications_send")
func sendNotification(
  identifierPtr: UnsafePointer<UInt8>,
  identifierLen: UInt32,
  titlePtr: UnsafePointer<UInt8>,
  titleLen: UInt32,
  bodyPtr: UnsafePointer<UInt8>,
  bodyLen: UInt32
) {
  let identifier = String.copyBuffer(identifierPtr, identifierLen)
  let title = String.copyBuffer(titlePtr, titleLen)
  let body = String.copyBuffer(bodyPtr, bodyLen)

  print("Sending notification: \(identifier), \(title), \(body)")

  let content = UNMutableNotificationContent()
  content.title = title
  content.body = body

  let center = UNUserNotificationCenter.current()
  center.add(
    UNNotificationRequest(
      identifier: identifier,
      content: content,
      trigger: nil
    ))
}

@_cdecl("notifications_remove")
func removeNotification(
  identifiersPtr: UnsafePointer<UInt8>,
  identifiersLen: UInt32
) {
  let identifiers = String.copyBuffer(identifiersPtr, identifiersLen)
  let identifiersArray = identifiers.split(separator: "\0").map { String($0) }

  let center = UNUserNotificationCenter.current()
  center.removeDeliveredNotifications(withIdentifiers: identifiersArray)
}

extension StringProtocol {
  static func copyBuffer(_ buffer: UnsafePointer<UInt8>, _ len: UInt32) -> String {
    String(data: Data(bytes: buffer, count: Int(len)), encoding: .utf8)!
  }
}

@_cdecl("notifications_get_delivered")
func deliverNotifications(
  ctx: UnsafeMutableRawPointer?,
  handler: @convention(c) (UnsafeMutableRawPointer?, UnsafePointer<UInt8>, UInt32) -> Void,
  finish: @convention(c) (UnsafeMutableRawPointer?) -> Void
) {
  let center = UNUserNotificationCenter.current()
  center.getDeliveredNotifications { notifications in
    for notification in notifications {
      let identifier = notification.request.identifier
      if #available(macOS 11, iOS 13.0, *) {
        log.debug("Received pending notification: \(identifier)")
      }
      guard let identifierData = identifier.data(using: .utf8) else {
        continue
      }
      identifierData.withUnsafeBytes {
        let buffer = $0.baseAddress!.assumingMemoryBound(to: UInt8.self)
        handler(ctx, buffer, UInt32(identifierData.count))
      }
    }
    finish(ctx);
  }
}
