import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/api/types.dart';

class MessageState {
  const MessageState();

  UiConversationMessage? get message => null;
}

class MessageCubit extends Cubit<MessageState> {
  MessageCubit({
    required userCubit,
    required UiConversationMessageId messageId,
  }) : super(MessageState());
}
