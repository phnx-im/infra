// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/styles.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user_cubit.dart';

import 'conversation_list.dart';
import 'conversation_list_cubit.dart';
import 'footer.dart';
import 'top.dart';

class ConversationViewContainer extends StatelessWidget {
  const ConversationViewContainer({super.key});

  @override
  Widget build(BuildContext context) {
    final userCubit = context.read<UserCubit>();
    return BlocProvider(
      create: (context) {
        return ConversationListCubit(userCubit: userCubit);
      },
      child: const ConversationView(),
    );
  }
}

class ConversationView extends StatelessWidget {
  const ConversationView({super.key});

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
      child: const Scaffold(
        backgroundColor: convPaneBackgroundColor,
        body: Column(
          mainAxisAlignment: MainAxisAlignment.end,
          children: [
            ConversationListTop(),
            SizedBox(height: Spacings.s),
            Expanded(child: ConversationList()),
            ConversationListFooter(),
          ],
        ),
      ),
    );
  }
}
