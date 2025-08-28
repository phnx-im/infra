// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:air/l10n/l10n.dart';
import 'package:air/navigation/navigation.dart';
import 'package:air/theme/theme.dart';
import 'package:air/ui/colors/themes.dart';
import 'package:air/user/user.dart';
import 'package:air/widgets/widgets.dart';
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
    _controller.text = context.read<UsersCubit>().state.displayName();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final profilePicture = context.select(
      (UsersCubit cubit) => cubit.state.profilePicture(),
    );

    final loc = AppLocalizations.of(context);

    return Scaffold(
      appBar: AppBar(
        title: Text(loc.editDisplayNameScreen_title),
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
                  image: profilePicture,
                  size: 100,
                ),
                const SizedBox(height: Spacings.m),
                TextFormField(
                  autofocus: true,
                  controller: _controller,
                  decoration: InputDecoration(
                    hintText: loc.userHandleScreen_inputHint,
                  ),
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
                      loc.editDisplayNameScreen_description,
                    ),
                  ),
                ),
                const Spacer(),
                OutlinedButton(
                  onPressed: () => _submit(context),
                  style: buttonStyle(CustomColorScheme.of(context), true),
                  child: Text(loc.editDisplayNameScreen_save),
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
