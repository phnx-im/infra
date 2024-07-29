//
//  NotificationService.swift
//  NotificationService
//
//  Created by Raphael Robert on 17.07.2024.
//

import UserNotifications
import Foundation
struct IncomingNotificationContent: Codable {
    let title: String
    let body: String
    let data: String
}

struct NotificationBatch: Codable {
    let removals: [String]
    let additions: [NotificationContent]
}

struct NotificationContent: Codable {
    let identifier: String
    let title: String
    let body: String
}

class NotificationService: UNNotificationServiceExtension {

    var contentHandler: ((UNNotificationContent) -> Void)?
    var bestAttemptContent: UNMutableNotificationContent?

    override func didReceive(_ request: UNNotificationRequest, withContentHandler contentHandler: @escaping (UNNotificationContent) -> Void) {
        
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

        // Create IncomingNotificationContent object
        let incomingContent = IncomingNotificationContent(title: bestAttemptContent.title, body: bestAttemptContent.body, data: data)

       if let jsonData = try? JSONEncoder().encode(incomingContent),
           let jsonString = String(data: jsonData, encoding: .utf8) {
            
            jsonString.withCString { cString in
                if let responsePointer = process_new_messages(cString) {
                    let responseString = String(cString: responsePointer)
                    free_string(responsePointer)

                    if let responseData = responseString.data(using: .utf8),
                       let notificationBatch = try? JSONDecoder().decode(NotificationBatch.self, from: responseData) {
                        
                        handleNotificationBatch(notificationBatch, contentHandler: contentHandler, bestAttemptContent: bestAttemptContent)
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

    func handleNotificationBatch(_ batch: NotificationBatch, contentHandler: @escaping (UNNotificationContent) -> Void, bestAttemptContent: UNMutableNotificationContent) {
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
                let request = UNNotificationRequest(identifier: notificationContent.identifier, content: newContent, trigger: nil)
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
            if let lastNotification = lastNotification {
                bestAttemptContent.title = lastNotification.title
                bestAttemptContent.body = lastNotification.body
                contentHandler(bestAttemptContent)
            } else {
                // Delay the callback by 1 second so that the notifications can be removed
                DispatchQueue.main.asyncAfter(deadline: .now() + 1) {
                    contentHandler(UNNotificationContent())
                }
            }
        }
    }
}
