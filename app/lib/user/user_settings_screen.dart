// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:image_picker/image_picker.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

const _listIconSize = 36.0;

class UserSettingsScreen extends StatelessWidget {
  const UserSettingsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final profile = context.select(
      (ContactsCubit cubit) => cubit.state.profile(),
    );

    return Scaffold(
      appBar: AppBar(
        title: const Text('User Settings'),
        toolbarHeight: isPointer() ? 100 : null,
        leading: const AppBarBackButton(),
      ),
      body: SafeArea(
        child: Align(
          alignment: Alignment.topCenter,
          child: Container(
            constraints:
                isPointer() ? const BoxConstraints(maxWidth: 800) : null,
            child: SingleChildScrollView(
              child: Column(
                children: [
                  UserAvatar(
                    displayName: profile.displayName,
                    size: 100,
                    image: profile.profilePicture,
                    onPressed: () => _pickAvatar(context),
                  ),
                  const SizedBox(height: Spacings.xs),

                  const _UserProfileData(),

                  const SizedBox(height: Spacings.xs),
                  Divider(color: Theme.of(context).hintColor),
                  const SizedBox(height: Spacings.xs),

                  const _UserHandles(),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  void _pickAvatar(BuildContext context) async {
    final user = context.read<UserCubit>();

    final ImagePicker picker = ImagePicker();
    final XFile? image = await picker.pickImage(source: ImageSource.gallery);
    final bytes = await image?.readAsBytes();

    if (bytes != null) {
      await user.setProfile(profilePicture: bytes);
    }
  }
}

class _UserProfileData extends StatelessWidget {
  const _UserProfileData();

  @override
  Widget build(BuildContext context) {
    final userId = context.select((UserCubit cubit) => cubit.state.userId);
    final displayName = context.select(
      (ContactsCubit cubit) => cubit.state.displayName(),
    );

    return ListView(
      shrinkWrap: true,
      children: [
        ListTile(
          leading: const Icon(Icons.person_outline, size: _listIconSize),
          title: Text(displayName),
          onTap:
              () => context.read<NavigationCubit>().openUserSettings(
                screen: UserSettingsScreenType.editDisplayName,
              ),
        ),
        const SizedBox(height: Spacings.s),
        ListTile(
          leading: const Icon(Icons.numbers, size: _listIconSize),
          title: Text(userId.uuid.toString()),
          onTap: () {
            _copyTextToClipboard(
              context,
              userId.uuid.toString(),
              snackBarMessage: "User ID copied to clipboard",
            );
          },
        ),
        const SizedBox(height: Spacings.xs),
        ListTile(
          subtitle: Text(
            style: TextStyle(color: Theme.of(context).hintColor),
            "Others will see your picture and name when you communicate with them.",
          ),
        ),
      ],
    );
  }

  void _copyTextToClipboard(
    BuildContext context,
    String textToCopy, {
    required String snackBarMessage,
  }) async {
    final messenger = ScaffoldMessenger.of(context);
    await Clipboard.setData(ClipboardData(text: textToCopy));
    messenger.showSnackBar(SnackBar(content: Text(snackBarMessage)));
  }
}

class _UserHandles extends StatelessWidget {
  const _UserHandles();

  @override
  Widget build(BuildContext context) {
    final userHandles = context.select(
      (UserCubit cubit) => cubit.state.userHandles,
    );

    return ListView(
      shrinkWrap: true,
      children:
          userHandles.isEmpty
              // no user handles yet
              ? [
                const _UserHandlePlaceholder(),
                const SizedBox(height: Spacings.xs),
                ListTile(
                  subtitle: Text(
                    style: TextStyle(color: Theme.of(context).hintColor),
                    "Share usernames with others so they can connect with you.\nAfter the connection, usernames are not visible to others anymore.\nYou can have up to 5 usernames.",
                  ),
                ),
              ]
              // user handles
              : [
                ...userHandles.expand(
                  (handle) => [
                    _UserHandle(handle: handle),
                    const SizedBox(height: Spacings.xs),
                  ],
                ),
                if (userHandles.length < 5) ...[
                  const _UserHandlePlaceholder(),
                  const SizedBox(height: Spacings.xs),
                ],
                ListTile(
                  subtitle: Text(
                    style: TextStyle(color: Theme.of(context).hintColor),
                    "Share usernames with others so they can connect with you. After the connection, "
                    "usernames are not visible to others anymore. "
                    "You can have up to 5 usernames.",
                  ),
                ),
              ],
    );
  }
}

class _UserHandle extends StatelessWidget {
  const _UserHandle({required this.handle});

  final UiUserHandle handle;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: const Icon(Icons.alternate_email, size: _listIconSize),
      title: Text(handle.plaintext),
      onTap: () => _removeHandle(context),
    );
  }

  void _removeHandle(BuildContext context) async {
    await showDialog(
      context: context,
      builder: (BuildContext context) {
        return AlertDialog(
          title: const Text("Remove Username"),
          content: const Text(
            "If you continue, your username will be removed and may be claimed by someone else. "
            "You’ll no longer be reachable through it.",
          ),
          actions: [
            TextButton(
              onPressed: () {
                Navigator.of(context).pop(false);
              },
              style: textButtonStyle(context),
              child: const Text("Cancel"),
            ),
            TextButton(
              onPressed: () async {
                await context.read<UserCubit>().removeUserHandle(handle);
                if (context.mounted) {
                  Navigator.of(context).pop(true);
                }
              },
              style: textButtonStyle(context),
              child: const Text("Remove"),
            ),
          ],
        );
      },
    );
  }
}

class _UserHandlePlaceholder extends StatelessWidget {
  const _UserHandlePlaceholder();

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: const Icon(Icons.alternate_email, size: _listIconSize),
      title: Text(
        style: TextStyle(color: Theme.of(context).hintColor),
        "Username",
      ),
      onTap:
          () => context.read<NavigationCubit>().openUserSettings(
            screen: UserSettingsScreenType.addUserHandle,
          ),
    );
  }
}
