// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:air/theme/theme.dart';
import 'package:air/ui/colors/themes.dart';
import 'package:air/user/user.dart';

import 'chat_list_content.dart';
import 'chat_list_cubit.dart';
import 'chat_list_header.dart';

class ChatListContainer extends StatelessWidget {
  const ChatListContainer({required this.isStandalone, super.key});

  final bool isStandalone;

  @override
  Widget build(BuildContext context) {
    return BlocProvider(
      create: (context) => ChatListCubit(userCubit: context.read<UserCubit>()),
      child: ChatListView(scaffold: isStandalone),
    );
  }
}

class ChatListView extends StatelessWidget {
  const ChatListView({super.key, this.scaffold = false});

  final bool scaffold;

  double _topPadding() {
    return isPointer() ? Spacings.l : kToolbarHeight;
  }

  @override
  Widget build(BuildContext context) {
    final widget = Container(
      color: CustomColorScheme.of(context).backgroundBase.primary,
      padding: EdgeInsets.only(top: _topPadding()),
      child: const Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [ChatListHeader(), Expanded(child: ChatListContent())],
      ),
    );
    return scaffold
        ? Scaffold(
          backgroundColor: CustomColorScheme.of(context).backgroundBase.primary,
          body: widget,
        )
        : widget;
  }
}
