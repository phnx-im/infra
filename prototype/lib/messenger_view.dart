import 'package:flutter/material.dart';
import 'package:prototype/conversation_list_pane/conversation_view.dart';
import 'package:prototype/conversation_pane/conversation_pane.dart';
import 'package:prototype/styles.dart';

void main() {
  runApp(const MessengerView());
}

// Combined navigator
void pushToNavigator(BuildContext context, Widget widget) {
  Navigator.of(context).push(MaterialPageRoute(builder: (context) => widget));
}

class MessengerView extends StatelessWidget {
  const MessengerView({super.key});

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        if (isSmallScreen(context)) {
          return const ConversationView();
        } else {
          return Scaffold(
            body: Row(
              children: [
                const SizedBox(
                  width: 300,
                  child: ConversationView(),
                ),
                Expanded(
                  child: ConversationPane(navigatorKey),
                ),
              ],
            ),
          );
        }
      },
    );
  }
}
