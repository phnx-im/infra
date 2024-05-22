import 'dart:async';
import 'dart:typed_data';
import 'package:collection/collection.dart';
import 'dart:ffi';
import 'dart:io';
import 'package:applogic/applogic.dart';
import 'package:path_provider/path_provider.dart';

DynamicLibrary loadLibForFlutter(String path) =>
    Platform.isIOS || Platform.isMacOS
        ? DynamicLibrary.process()
        : DynamicLibrary.open(path);

const base = 'phnxapplogic';
final path = Platform.isWindows
    ? 'windows/$base.dll'
    : Platform.isAndroid
        ? 'lib$base.so'
        : 'linux/lib$base.so';
final dylib = loadLibForFlutter(path);
final bridge = RustBridgeImpl(dylib);

// Helper definitions
Function unOrdDeepEq = const DeepCollectionEquality.unordered().equals;

class CoreClient {
  static final CoreClient _coreClient = CoreClient._internal();

  factory CoreClient() {
    return _coreClient;
  }

  CoreClient._internal();

  List<UiConversation> _conversations = [];
  late RustUser user;
  Timer pollingTimer = Timer(Duration.zero, () => {});
  UiConversation? _currentConversation;

  late UiUserProfile ownProfile;

  // This event is dispatched whenever we switch to a new conversation

  StreamController<UiConversation> conversationSwitch =
      StreamController<UiConversation>.broadcast();

  Stream<UiConversation> get onConversationSwitch => conversationSwitch.stream;

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

  // Logging
  void setupLogging() async {
    /*  createLogStream().listen((event) {
      print(
          'log from rust: ${event.level} ${event.tag} ${event.msg} ${event.timeMillis}');
    }); */
  }

  String get username {
    return ownProfile.userName;
  }

  Future<String> dbPath() async {
    final directory = await getApplicationDocumentsDirectory();
    final path = directory.path;

    print("Document path: $path");
    return path;
  }

  Future<void> deleteDatabases() async {
    await bridge.deleteDatabases(clientDbPath: await dbPath());
  }

  Future<bool> loadUser() async {
    try {
      final userBuilder = await UserBuilder.newUserBuilder(bridge: bridge);
      final stream = userBuilder.getStream().asBroadcastStream();

      // We wait for the first element to be received
      await stream.firstWhere((UiNotificationType event) {
        return true;
      }, orElse: () => throw Exception("No first_notification received"));

      user = await userBuilder.loadDefault(path: await dbPath());

      ownProfile = await user.ownUserProfile();

      print("Loaded user: ${ownProfile.userName}");

      stageUser(ownProfile.userName, stream);

      return true;
    } catch (e) {
      print("Error when loading user: $e");
      return false;
    }
  }

  Future<void> createUser(
      String userName, String password, String address) async {
    final fqun = userName;
    final userBuilder = await UserBuilder.newUserBuilder(bridge: bridge);
    final stream = userBuilder.getStream().asBroadcastStream();

    await stream.firstWhere((UiNotificationType event) {
      return true;
    }, orElse: () => throw Exception("No first_notification received"));

    user = await userBuilder.createUser(
        userName: fqun,
        password: password,
        address: address,
        path: await dbPath());

    print("User registered");

    stageUser(userName, stream);
  }

  Future<void> stageUser(
      String userName, Stream<UiNotificationType> stream) async {
    // Load existing conversations
    await conversations();

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
        case WsNotification.Connected:
          print("Connected to the websocket");
          startPolling();
          break;
        case WsNotification.Disconnected:
          print("Disconnected from the websocket");
          cancelPolling();
          break;
        case WsNotification.QueueUpdate:
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
    } catch (e) {
      print("Error when fetching messages: $e");
    }
  }

  Future<List<UiConversation>> conversations() async {
    _conversations = await user.getConversations();
    return _conversations;
  }

  UiConversation? get currentConversation {
    return _currentConversation;
  }

  List<UiConversation> get conversationsList {
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
