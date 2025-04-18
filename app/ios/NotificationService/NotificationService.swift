//
//  NotificationService.swift
//  NotificationService
//

import Foundation
import UserNotifications

struct IncomingNotificationContent: Codable {
  let title: String
  let body: String
  let data: String
  let path: String
  let logFilePath: String
}

struct NotificationBatch: Codable {
  let badgeCount: UInt32
  let removals: [String]
  let additions: [NotificationContent]
}

struct NotificationContent: Codable {
  let identifier: UUID
  let title: String
  let body: String
  let conversationId: ConversationId
}

struct ConversationId: Codable {
  let uuid: UUID
}

class NotificationService: UNNotificationServiceExtension {

  var contentHandler: ((UNNotificationContent) -> Void)?
  var bestAttemptContent: UNMutableNotificationContent?

  override func didReceive(
    _ request: UNNotificationRequest,
    withContentHandler contentHandler: @escaping (UNNotificationContent) -> Void
  ) {

    NSLog("NSE Received notification")
    self.contentHandler = contentHandler
    bestAttemptContent = (request.content.mutableCopy() as? UNMutableNotificationContent)

    guard let bestAttemptContent = bestAttemptContent else {
      contentHandler(request.content)
      return
    }

    // Extract the "data" field from the push notification payload
    let userInfo = request.content.userInfo
    guard let data = userInfo["data"] as? String else {
      NSLog("NSE Data field not set")
      contentHandler(request.content)
      return
    }

    // Find the documents directory path for the databases
    guard
      let containerURL = FileManager.default.containerURL(
        forSecurityApplicationGroupIdentifier: "group.im.phnx.prototype")
    else {
      NSLog("NSE Could not find documents directory")
      contentHandler(request.content)
      return
    }
    let path = containerURL.appendingPathComponent("Documents").path

    guard
      let cachesDirectory = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)
        .first
    else {
      NSLog("NSE Could not find cache directory")
      contentHandler(request.content)
      return
    }
    let logFilePath = cachesDirectory.appendingPathComponent("background.log").path

    // Create IncomingNotificationContent object
    let incomingContent = IncomingNotificationContent(
      title: bestAttemptContent.title,
      body: bestAttemptContent.body,
      data: data,
      path: path,
      logFilePath: logFilePath
    )

    if let jsonData = try? JSONEncoder().encode(incomingContent),
      let jsonString = String(data: jsonData, encoding: .utf8)
    {

      jsonString.withCString { cString in
        if let responsePointer = process_new_messages(cString) {
          let responseString = String(cString: responsePointer)
          free_string(responsePointer)

          if let responseData = responseString.data(using: .utf8),
            let notificationBatch = try? JSONDecoder().decode(
              NotificationBatch.self, from: responseData)
          {

            handleNotificationBatch(notificationBatch, contentHandler: contentHandler)
          } else {
            contentHandler(request.content)
          }
        } else {
          contentHandler(request.content)
        }
      }
    } else {
      contentHandler(request.content)
    }
  }

  override func serviceExtensionTimeWillExpire() {
    NSLog("NSE Expiration handler invoked")
    if let contentHandler = contentHandler, let bestAttemptContent = bestAttemptContent {
      bestAttemptContent.title = "Timer expired"
      bestAttemptContent.body = "Please report this issue"
      contentHandler(bestAttemptContent)
    }
  }

  func handleNotificationBatch(
    _ batch: NotificationBatch, contentHandler: @escaping (UNNotificationContent) -> Void
  ) {
    let center = UNUserNotificationCenter.current()
    let dispatchGroup = DispatchGroup()

    // Remove notifications
    center.removeDeliveredNotifications(withIdentifiers: batch.removals)

    // Add notifications
    var lastNotification: NotificationContent?
    for (index, notificationContent) in batch.additions.enumerated() {
      // This cannot underflow because there is at least one addition
      if index == batch.additions.count - 1 {
        lastNotification = notificationContent
      } else {
        dispatchGroup.enter()
        let newContent = UNMutableNotificationContent()
        newContent.title = notificationContent.title
        newContent.body = notificationContent.body
        newContent.sound = UNNotificationSound.default
        newContent.userInfo["conversationId"] = notificationContent.conversationId.uuid.uuidString
        let request = UNNotificationRequest(
          identifier: notificationContent.identifier.uuidString,
          content: newContent,
          trigger: nil)
        center.add(request) { error in
          if let error = error {
            NSLog("NSE Error adding notification: \(error)")
          }
          dispatchGroup.leave()
        }
      }
    }

    // Notify when all notifications are added
    dispatchGroup.notify(queue: DispatchQueue.main) {
      let content = UNMutableNotificationContent()
      if let lastNotification = lastNotification {
        content.title = lastNotification.title
        content.body = lastNotification.body
        content.sound = UNNotificationSound.default
        content.userInfo["conversationId"] = lastNotification.conversationId.uuid.uuidString
      }
      // Add the badge number
      content.badge = NSNumber(value: batch.badgeCount)
      // Delay the callback by 1 second so that the notifications can be removed
      DispatchQueue.main.asyncAfter(deadline: .now() + 1) {
        contentHandler(content)
      }
    }
  }
}
