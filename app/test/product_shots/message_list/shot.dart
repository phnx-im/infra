import 'package:air/conversation_list/conversation_list_cubit.dart';
import 'package:air/conversation_list/conversation_list_view.dart';
import 'package:air/core/api/conversation_details_cubit.dart';
import 'package:air/core/api/conversation_list_cubit.dart';
import 'package:air/core/api/navigation_cubit.dart';
import 'package:air/core/api/types.dart';
import 'package:air/l10n/app_localizations.dart';
import 'package:air/navigation/navigation_cubit.dart';
import 'package:air/theme/theme_data.dart';
import 'package:air/ui/colors/themes.dart';
import 'package:air/user/user_cubit.dart';
import 'package:air/user/users_cubit.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import 'content.dart';
import '../../helpers.dart';
import '../../mocks.dart';
import '../product_shot.dart';

/// Large store headline used in product shots.
class ShotTitle extends StatelessWidget {
  const ShotTitle({super.key, required this.text});

  final String text;

  static const _style = TextStyle(
    fontSize: 64,
    fontWeight: FontWeight.w800,
    color: Color.fromARGB(255, 59, 61, 65), // dark grey title
    height: 1.5,
    letterSpacing: -0.5,
  );

  @override
  Widget build(BuildContext context) {
    return DefaultTextStyle.merge(
      style: _style,
      child: Text(
        text,
        maxLines: 2,
        textAlign: TextAlign.center,
        overflow: TextOverflow.ellipsis,
      ),
    );
  }
}

void main() {
  late MockNavigationCubit navigationCubit;
  late MockConversationListCubit conversationListCubit;
  late MockUserCubit userCubit;
  late MockUsersCubit contactsCubit;
  late MockConversationDetailsCubit conversationDetailsCubit;

  setUp(() async {
    navigationCubit = MockNavigationCubit();
    userCubit = MockUserCubit();
    conversationListCubit = MockConversationListCubit();
    contactsCubit = MockUsersCubit();
    conversationDetailsCubit = MockConversationDetailsCubit();

    when(() => navigationCubit.state).thenReturn(const NavigationState.home());
    when(() => userCubit.state).thenReturn(MockUiUser(id: 1));
    when(() => contactsCubit.state).thenReturn(
      MockUsersState(
        profiles: [UiUserProfile(userId: 1.userId(), displayName: "alice")],
      ),
    );
    when(() => conversationDetailsCubit.state).thenReturn(
      ConversationDetailsState(
        conversation: conversations[1],
        members: [1.userId()],
      ),
    );
  });

  testWidgets('product shot fixed size with label and 80% frame', (
    tester,
  ) async {
    when(() => navigationCubit.state).thenReturn(
      NavigationState.home(
        home: HomeNavigationState(
          conversationOpen: true,
          conversationId: conversations[1].id,
        ),
      ),
    );
    when(() => conversationListCubit.state).thenReturn(
      ConversationListState(
        conversations: List.generate(
          20,
          (index) => conversations[index % conversations.length],
        ),
      ),
    );

    // Set surface size to exactly match ProductShot dimensions
    const shotWidth = 1242;
    const shotHeight = 2000; //2688;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    tester.view.physicalSize = Size(
      shotWidth.toDouble(),
      shotHeight.toDouble(),
    );
    tester.view.devicePixelRatio = 1.0;

    const shot = ProductShot(
      widthPx: shotWidth,
      heightPx: shotHeight,
      backgroundColor: Color.fromARGB(
        255,
        221,
        227,
        234,
      ), // light metallic grey
      label: 'Private messaging.\nFor everybody.',
      child: ConversationListView(),
    );

    Widget buildSubject() => MultiBlocProvider(
      providers: [
        BlocProvider<NavigationCubit>.value(value: navigationCubit),
        BlocProvider<UserCubit>.value(value: userCubit),
        BlocProvider<UsersCubit>.value(value: contactsCubit),
        BlocProvider<ConversationListCubit>.value(value: conversationListCubit),
      ],
      child: Builder(
        builder: (context) {
          return MaterialApp(
            debugShowCheckedModeBanner: false,
            theme: themeData(
              MediaQuery.platformBrightnessOf(context),
              CustomColorScheme.of(context),
            ),
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            home: const Scaffold(body: shot),
          );
        },
      ),
    );

    await tester.pumpWidget(buildSubject());
    await tester.pump(const Duration(milliseconds: 100));

    await expectLater(
      find.byType(ProductShot),
      matchesGoldenFile('../shots/message_list.png'),
    );
  });
}
