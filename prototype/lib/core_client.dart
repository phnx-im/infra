// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';
import 'dart:io';
import 'dart:typed_data';
import 'package:collection/collection.dart';
import 'package:path_provider/path_provider.dart';
import 'package:prototype/core/api/mobile_logging.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core/api/user.dart';
import 'package:prototype/core/api/utils.dart';
import 'package:prototype/core/frb_generated.dart';
import 'package:prototype/core/lib.dart';
import 'package:prototype/platform.dart';

// Helper definitions
Function unOrdDeepEq = const DeepCollectionEquality.unordered().equals;

class CoreClient {
  static final CoreClient _coreClient = CoreClient._internal();

  factory CoreClient() {
    return _coreClient;
  }

  CoreClient._internal();

  List<UiConversationDetails> _conversations = [];
  User? _user;
  Timer pollingTimer = Timer(Duration.zero, () => {});
  UiConversationDetails? _currentConversation;

  late UiUserProfile ownProfile;

  // This event is dispatched whenever we switch to a new conversation

  StreamController<UiConversationDetails> conversationSwitch =
      StreamController<UiConversationDetails>.broadcast();

  Stream<UiConversationDetails> get onConversationSwitch =>
      conversationSwitch.stream;

  // This event is dispatched whenever there is a change to the conversation list

  late StreamController<ConversationIdBytes> conversationListUpdates =
      StreamController<ConversationIdBytes>.broadcast();

  Stream<ConversationIdBytes> get onConversationListUpdate =>
      conversationListUpdates.stream;

  // This event is dispatched whenever a new message is received from the corelib

  late StreamController<UiConversationMessage> messageUpdates =
      StreamController<UiConversationMessage>.broadcast();

  Stream<UiConversationMessage> get onMessageUpdate => messageUpdates.stream;

  // This event is dispatched whenever the user's profile is updated

  late StreamController<UiUserProfile> ownProfileUpdate =
      StreamController<UiUserProfile>.broadcast();

  Stream<UiUserProfile> get onOwnProfileUpdate => ownProfileUpdate.stream;

  User? get maybeUser => _user;

  User get user => _user!;
  set user(User user) {
    _user = user;
  }

  Future<void> init() async {
    // FRB
    await RustLib.init();
    // Logging
    createLogStream().listen((event) {
      print(
          'Rust: ${event.level} ${event.tag} ${event.msg} ${event.timeMillis}');
    });
  }

  String get username {
    return ownProfile.userName;
  }

  Future<String> _dbPath() async {
    final String path;

    if (Platform.isAndroid || Platform.isIOS) {
      path = await getDatabaseDirectoryMobile();
    } else {
      final directory = await getApplicationDocumentsDirectory();
      path = directory.path;
    }

    print("Database path: $path");
    return path;
  }

  Future<void> deleteDatabase() async {
    await deleteDatabases(clientDbPath: await _dbPath());
  }

  Future<bool> loadUser() async {
    try {
      user = await User.loadDefault(path: await _dbPath());

      ownProfile = await user.ownUserProfile();

      print("Loaded user: ${ownProfile.userName}");

      _stageUser(ownProfile.userName);

      return true;
    } catch (e) {
      print("Error when loading user: $e");
      return false;
    }
  }

  Future<void> createUser(
      String userName, String password, String address) async {
    PlatformPushToken? pushToken;

    if (Platform.isAndroid) {
      final String? deviceToken = await getDeviceToken();

      if (deviceToken != null) {
        pushToken = PlatformPushToken.google(deviceToken);
      }
    } else if (Platform.isIOS) {
      final String? deviceToken = await getDeviceToken();

      if (deviceToken != null) {
        pushToken = PlatformPushToken.apple(deviceToken);
      }
    }

    user = await User.newInstance(
      userName: userName,
      password: password,
      address: address,
      path: await _dbPath(),
      pushToken: pushToken,
    );

    print("User registered");

    _stageUser(userName);
  }

  Future<void> _stageUser(String userName) async {
    // Load existing conversations
    await conversations();

    final stream = user.notificationStream().asBroadcastStream();

    stream.listen((UiNotificationType event) {
      print("Event: $event");
      event.when(
          conversationChange: (uuid) async {
            conversationListUpdates.add(uuid);
            await conversations();
          },
          message: (message) => {messageUpdates.add(message)});
    });

    print("User created, connecting to websocket");
    var websocket = user.websocket(timeout: 30, retryInterval: 10);

    websocket.listen((WsNotification event) async {
      print("Event: $event");
      switch (event) {
        case WsNotification.connected:
          print("Connected to the websocket");
          startPolling();
          break;
        case WsNotification.disconnected:
          print("Disconnected from the websocket");
          cancelPolling();
          break;
        case WsNotification.queueUpdate:
          print("Queue update");
          await fetchMessages();
          break;
        default:
          break;
      }
    });

    startPolling();
  }

  void startPolling() {
    if (pollingTimer.isActive) {
      cancelPolling();
    }

    pollingTimer = Timer.periodic(
      const Duration(seconds: 10),
      (timer) async {
        await fetchMessages();
      },
    );
  }

  void cancelPolling() {
    pollingTimer.cancel();
  }

  Future<void> fetchMessages() async {
    try {
      await user.fetchMessages();
      // iOS only
      if (Platform.isIOS) {
        final count = await coreClient.user.globalUnreadMessagesCount();
        await setBadgeCount(count);
      }
      conversationListUpdates.add(ConversationIdBytes(bytes: U8Array16.init()));
    } catch (e) {
      print("Error when fetching messages: $e");
    }
  }

  Future<List<UiConversationDetails>> conversations() async {
    _conversations = await user.getConversationDetails();
    return _conversations;
  }

  UiConversationDetails? get currentConversation {
    return _currentConversation;
  }

  List<UiConversationDetails> get conversationsList {
    return _conversations;
  }

  Future<ConversationIdBytes> createConversation(String name) async {
    final conversationId = await user.createConversation(name: name);
    conversationListUpdates.add(conversationId);
    conversations();
    selectConversation(conversationId);
    return conversationId;
  }

  Future<void> sendMessage(
      ConversationIdBytes conversationId, String message) async {
    UiConversationMessage conversationMessage;
    try {
      conversationMessage = await user.sendMessage(
          conversationId: conversationId, message: message);
    } catch (e) {
      print("Error when sending message: $e");
      return;
    }

    messageUpdates.add(conversationMessage);
    conversationListUpdates
        .add(ConversationIdBytes(bytes: conversationId.bytes));
  }

  Future<void> createConnection(String userName) async {
    await user.createConnection(userName: userName);
    conversationListUpdates.add(ConversationIdBytes(bytes: U8Array16.init()));
  }

  Future<List<String>> getMembers(ConversationIdBytes conversationId) async {
    return await user.membersOfConversation(conversationId: conversationId);
  }

  Future<void> addUserToConversation(
      ConversationIdBytes conversationId, String userName) async {
    await user.addUsersToConversation(
        conversationId: conversationId, userNames: [userName]);
  }

  Future<void> removeUserFromConversation(
      ConversationIdBytes conversationId, String userName) async {
    await user.removeUsersFromConversation(
        conversationId: conversationId, userNames: [userName]);
  }

  Future<List<UiContact>> getContacts() async {
    return await user.getContacts();
  }

  void selectConversation(ConversationIdBytes conversationId) {
    _currentConversation = _conversations
        .where((conversation) =>
            conversation.id.bytes.equals(conversationId.bytes))
        .firstOrNull;
    if (_currentConversation != null) {
      conversationSwitch.add(_currentConversation!);
    }
  }

  Future<void> setOwnProfile(String displayName, Uint8List? picture) async {
    await user.setUserProfile(
        displayName: displayName, profilePictureOption: picture);
    ownProfile = await user.ownUserProfile();
    ownProfileUpdate.add(ownProfile);
  }
}

final coreClient = CoreClient();
