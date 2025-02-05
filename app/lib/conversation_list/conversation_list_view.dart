// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';

import 'conversation_list_content.dart';
import 'conversation_list_cubit.dart';
import 'conversation_list_footer.dart';
import 'conversation_list_header.dart';

class ConversationListContainer extends StatelessWidget {
  const ConversationListContainer({super.key});

  @override
  Widget build(BuildContext context) {
    return BlocProvider(
      create: (context) => ConversationListCubit(
        userCubit: context.read<UserCubit>(),
      ),
      child: const ConversationListView(),
    );
  }
}

class ConversationListView extends StatelessWidget {
  const ConversationListView({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: const BoxDecoration(
        shape: BoxShape.rectangle,
        border: Border(
          right: BorderSide(
            width: 1,
            color: colorGreyLight,
          ),
        ),
      ),
      child: Scaffold(
        backgroundColor: convPaneBackgroundColor,
        body: Stack(
          children: [
            Padding(
              padding: EdgeInsets.only(top: kToolbarHeight, bottom: 120),
              child: ConversationListContent(),
            ),
            ConversationListHeader(),
            ConversationListFooter(),
          ],
        ),
      ),
    );
  }
}
