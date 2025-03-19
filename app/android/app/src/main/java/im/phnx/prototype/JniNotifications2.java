package im.phnx.prototype;

public class JniNotifications2 {
    static void showNotification(String identifier, String title, String message) {
        JniNotifications.showNotification(identifier, title, message);
    }
}