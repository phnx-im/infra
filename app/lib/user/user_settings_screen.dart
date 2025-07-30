// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:image_picker/image_picker.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/l10n/l10n.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/util/debouncer.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

const _listIconSize = 36.0;

class UserSettingsScreen extends StatelessWidget {
  const UserSettingsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final profile = context.select((UsersCubit cubit) => cubit.state.profile());

    final loc = AppLocalizations.of(context);

    final isMobilePlatform = Platform.isAndroid || Platform.isIOS;
    final isDesktopPlatform =
        Platform.isMacOS || Platform.isWindows || Platform.isLinux;

    return Scaffold(
      appBar: AppBar(
        title: Text(loc.userSettingsScreen_title),
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

                  if (isMobilePlatform) ...[
                    const SizedBox(height: Spacings.xs),
                    Divider(color: Theme.of(context).hintColor),
                    const SizedBox(height: Spacings.xs),

                    const _MobileSettings(),
                  ],

                  if (isDesktopPlatform) ...[
                    const SizedBox(height: Spacings.xs),
                    Divider(color: Theme.of(context).hintColor),
                    const SizedBox(height: Spacings.xs),

                    const _DesktopSettings(),
                  ],
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
      (UsersCubit cubit) => cubit.state.displayName(),
    );

    final loc = AppLocalizations.of(context);

    return ListView(
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
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
              snackBarMessage: loc.userSettingsScreen_idCopied,
            );
          },
        ),
        const SizedBox(height: Spacings.xs),
        ListTile(
          subtitle: Text(
            style: TextStyle(color: Theme.of(context).hintColor),
            loc.userSettingsScreen_profileDescription,
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

    final loc = AppLocalizations.of(context);

    return ListView(
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      children: [
        ...userHandles.expand(
          (handle) => [
            _UserHandle(handle: handle),
            const SizedBox(height: Spacings.xs),
          ],
        ),
        if (userHandles.isEmpty || userHandles.length < 5) ...[
          const _UserHandlePlaceholder(),
          const SizedBox(height: Spacings.xs),
        ],
        ListTile(
          subtitle: Text(
            style: TextStyle(color: Theme.of(context).hintColor),
            loc.userSettingsScreen_userNamesDescription,
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
    final loc = AppLocalizations.of(context);
    await showDialog(
      context: context,
      builder: (BuildContext context) {
        return AlertDialog(
          title: Text(loc.removeUsernameDialog_title),
          content: Text(loc.removeUsernameDialog_content),
          actions: [
            TextButton(
              onPressed: () {
                Navigator.of(context).pop(false);
              },
              style: textButtonStyle(context),
              child: Text(loc.removeUsernameDialog_cancel),
            ),
            TextButton(
              onPressed: () async {
                await context.read<UserCubit>().removeUserHandle(handle);
                if (context.mounted) {
                  Navigator.of(context).pop(true);
                }
              },
              style: textButtonStyle(context),
              child: Text(loc.removeUsernameDialog_remove),
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
    final loc = AppLocalizations.of(context);

    return ListTile(
      leading: const Icon(Icons.alternate_email, size: _listIconSize),
      title: Text(
        style: TextStyle(color: Theme.of(context).hintColor),
        loc.userSettingsScreen_userHandlePlaceholder,
      ),
      onTap:
          () => context.read<NavigationCubit>().openUserSettings(
            screen: UserSettingsScreenType.addUserHandle,
          ),
    );
  }
}

class _MobileSettings extends StatefulWidget {
  const _MobileSettings();

  @override
  State<_MobileSettings> createState() => _MobileSettingsState();
}

class _MobileSettingsState extends State<_MobileSettings> {
  final Debouncer _sendOnEnterDebouncer = Debouncer(
    delay: const Duration(milliseconds: 500),
  );
  bool _sendOnEnter = false;

  @override
  void initState() {
    super.initState();
    setState(() {
      _sendOnEnter = context.read<UserSettingsCubit>().state.sendOnEnter;
    });
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      children: [
        SwitchListTile(
          title: const Text("Send with Enter"),
          value: _sendOnEnter,
          onChanged: (value) {
            _sendOnEnterDebouncer.run(() {
              context.read<UserSettingsCubit>().setSendOnEnter(
                userCubit: context.read(),
                value: value,
              );
            });
            setState(() {
              _sendOnEnter = value;
            });
          },
        ),
      ],
    );
  }
}

class _DesktopSettings extends StatefulWidget {
  const _DesktopSettings();

  @override
  State<_DesktopSettings> createState() => _DesktopSettingsState();
}

class _DesktopSettingsState extends State<_DesktopSettings> {
  double _interfaceScaleSliderValue = 100.0;

  @override
  void initState() {
    super.initState();
    setState(() {
      _interfaceScaleSliderValue =
          context.read<UserSettingsCubit>().state.interfaceScale * 100;
    });
  }

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);

    return ListView(
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      children: [
        ListTile(
          leading: const Icon(Icons.visibility, size: _listIconSize),
          titleAlignment: ListTileTitleAlignment.top,
          title: Text(loc.userSettingsScreen_interfaceScale),
          subtitle: Slider(
            min: 50,
            max: 300,
            divisions: ((300 - 50) / 5).truncate(),
            value: _interfaceScaleSliderValue,
            label: _interfaceScaleSliderValue.truncate().toString(),
            activeColor: colorDMB,
            onChanged:
                (value) => setState(() => _interfaceScaleSliderValue = value),
            onChangeEnd: (value) {
              context.read<UserSettingsCubit>().setInterfaceScale(
                userCubit: context.read(),
                value: value / 100,
              );
            },
          ),
        ),
        const SizedBox(height: Spacings.s),
      ],
    );
  }
}

Color getColor(Set<WidgetState> states) {
  const Set<WidgetState> interactiveStates = <WidgetState>{
    WidgetState.pressed,
    WidgetState.hovered,
    WidgetState.focused,
  };
  if (states.any(interactiveStates.contains)) {
    return Colors.brown;
  }
  return Colors.transparent;
}
