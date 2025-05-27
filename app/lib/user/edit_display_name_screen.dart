// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

class EditDisplayNameScreen extends StatefulWidget {
  const EditDisplayNameScreen({super.key});

  @override
  State<EditDisplayNameScreen> createState() => _EditDisplayNameScreenState();
}

class _EditDisplayNameScreenState extends State<EditDisplayNameScreen> {
  final _controller = TextEditingController();

  @override
  initState() {
    super.initState();
    final userCubit = context.read<UserCubit>();
    _controller.text = userCubit.state.displayName;
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final profilePicture = context.select(
      (UserCubit cubit) => cubit.state.profilePicture,
    );

    return Scaffold(
      appBar: AppBar(
        title: const Text('Display Name'),
        toolbarHeight: isPointer() ? 100 : null,
        leading: const AppBarBackButton(),
      ),
      body: SafeArea(
        child: Align(
          alignment: Alignment.topCenter,
          child: Container(
            constraints:
                isPointer() ? const BoxConstraints(maxWidth: 800) : null,
            padding: const EdgeInsets.all(Spacings.s),
            child: Column(
              children: [
                UserAvatar(
                  displayName: _controller.text.trim(),
                  size: 100,
                  image: profilePicture,
                ),
                const SizedBox(height: Spacings.m),
                TextFormField(
                  autofocus: true,
                  controller: _controller,
                  decoration: const InputDecoration(hintText: "Display name"),
                ),
                const SizedBox(height: Spacings.s),
                Align(
                  alignment: Alignment.centerLeft,
                  child: Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: Spacings.xxs,
                    ),
                    child: Text(
                      style: TextStyle(color: Theme.of(context).hintColor),
                      "Display Name is shared with Users and Groups in your "
                      "conversations.",
                    ),
                  ),
                ),
                const Spacer(),
                OutlinedButton(
                  onPressed: () => _submit(context),
                  style: buttonStyle(context, true),
                  child: const Text('Save'),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  void _submit(BuildContext context) async {
    final userCubit = context.read<UserCubit>();
    final navigationCubit = context.read<NavigationCubit>();
    await userCubit.setProfile(displayName: _controller.text.trim());
    navigationCubit.pop();
  }
}
