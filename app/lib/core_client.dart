// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';
import 'dart:io';
import 'dart:typed_data';
import 'package:collection/collection.dart';
import 'package:flutter/widgets.dart';
import 'package:path_provider/path_provider.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core/api/user.dart';
import 'package:prototype/core/api/utils.dart';
import 'package:prototype/platform.dart';
import 'package:provider/provider.dart';
import 'package:uuid/uuid.dart';

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

  UiUserProfile? _ownProfile;

  UiUserProfile get ownProfile => _ownProfile!;

  final StreamController<User?> _userController = StreamController<User?>();

  // This event is dispatched whenever we switch to a new conversation

  StreamController<UiConversationDetails> conversationSwitch =
      StreamController<UiConversationDetails>.broadcast();

  Stream<UiConversationDetails> get onConversationSwitch =>
      conversationSwitch.stream;

  // This event is dispatched whenever there is a change to the conversation list

  late StreamController<ConversationId> conversationListUpdates =
      StreamController<ConversationId>.broadcast();

  Stream<ConversationId> get onConversationListUpdate =>
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

  Stream<User?> get userStream => _userController.stream;

  User get user => _user!;
  set user(User user) {
    _userController.add(user);
    _user = user;
  }

  String get username => ownProfile.userName;

  Future<String> dbPath() async {
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
    await deleteDatabases(clientDbPath: await dbPath());
    _userController.add(null);
    _user = null;
  }

  Future<bool> loadUser() async {
    try {
      user = await User.loadDefault(path: await dbPath());

      final ownProfile = await user.ownUserProfile();
      _ownProfile = ownProfile;
      print("Loaded user: ${ownProfile.userName}");

      stageUser(ownProfile.userName);

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
      path: await dbPath(),
      pushToken: pushToken,
    );

    _ownProfile = await user.ownUserProfile();

    print("User registered");

    stageUser(userName);
  }

  Future<void> stageUser(String userName) async {
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
        final count = await user.globalUnreadMessagesCount();
        await setBadgeCount(count);
      }
      conversationListUpdates.add(
        const ConversationId(
          uuid: UuidValue.fromNamespace(Namespace.nil),
        ),
      );
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

  Future<ConversationId> createConversation(String name) async {
    final conversationId = await user.createConversation(name: name);
    conversationListUpdates.add(conversationId);
    conversations();
    selectConversation(conversationId);
    return conversationId;
  }

  Future<void> sendMessage(
      ConversationId conversationId, String message) async {
    UiConversationMessage conversationMessage;
    try {
      conversationMessage = await user.sendMessage(
          conversationId: conversationId, message: message);
    } catch (e) {
      print("Error when sending message: $e");
      return;
    }

    messageUpdates.add(conversationMessage);
    conversationListUpdates.add(conversationId);
  }

  Future<void> createConnection(String userName) async {
    await user.createConnection(userName: userName);
    conversationListUpdates.add(
      const ConversationId(
        uuid: UuidValue.fromNamespace(Namespace.nil),
      ),
    );
  }

  Future<List<String>> getMembers(ConversationId conversationId) async {
    return await user.membersOfConversation(conversationId: conversationId);
  }

  Future<void> addUserToConversation(
      ConversationId conversationId, String userName) async {
    await user.addUsersToConversation(
        conversationId: conversationId, userNames: [userName]);
  }

  Future<void> removeUserFromConversation(
      ConversationId conversationId, String userName) async {
    await user.removeUsersFromConversation(
        conversationId: conversationId, userNames: [userName]);
  }

  Future<List<UiContact>> getContacts() async {
    return await user.getContacts();
  }

  void selectConversation(ConversationId conversationId) {
    _currentConversation = _conversations
        .where((conversation) => conversation.id == conversationId)
        .firstOrNull;
    if (_currentConversation != null) {
      conversationSwitch.add(_currentConversation!);
    }
  }

  Future<void> setOwnProfile(String displayName, Uint8List? picture) async {
    await user.setUserProfile(
        displayName: displayName, profilePictureOption: picture);
    final ownProfile = await user.ownUserProfile();
    _ownProfile = ownProfile;
    ownProfileUpdate.add(ownProfile);
  }
}

extension BuildContextExtension on BuildContext {
  CoreClient get coreClient => read<CoreClient>();
}
