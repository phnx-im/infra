// TODO: This can be implemented on the Rust side
import 'dart:async';

import 'package:logging/logging.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/state_provider.dart';

final _log = Logger('CurrentConversationCubit');

class CurrentConversationCubit
    extends StateStreamableSource<UiConversationDetails?> {
  CurrentConversationCubit({
    required CoreClient coreClient,
  }) : _stateStream = coreClient.onConversationSwitch {
    _stateSubscription = coreClient.onConversationSwitch.listen(_replaceState);
  }

  void _replaceState(UiConversationDetails? state) {
    _log.fine('Replacing state: $state');
    _state = state;
  }

  bool _isClosed = false;
  UiConversationDetails? _state;
  final Stream<UiConversationDetails> _stateStream;
  late final StreamSubscription<UiConversationDetails?> _stateSubscription;

  @override
  void close() {
    _stateSubscription.cancel();
    _isClosed = true;
  }

  @override
  bool get isClosed => _isClosed;

  @override
  UiConversationDetails? get state => _state;

  @override
  Stream<UiConversationDetails?> get stream => _stateStream;
}
