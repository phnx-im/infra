import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/api/types.dart';
import 'package:uuid/uuid.dart';

class ConversationState {
  const ConversationState();

  UiConversationDetails? get conversation => null;

  int get messagesCount => 0;

  UuidValue get messagesEpoch => UuidValue.fromNamespace(Namespace.nil);
}

class ConversationCubit extends Cubit<ConversationState> {
  ConversationCubit({
    required userCubit,
    required ConversationId conversationId,
  }) : super(ConversationState());

  UiConversationMessageId messageIdFromRevOffset(int index) {
    throw UnimplementedError();
  }
}
