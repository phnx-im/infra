// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/util/platform.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

class DeveloperSettingsScreen extends StatefulWidget {
  const DeveloperSettingsScreen({super.key});

  @override
  State<DeveloperSettingsScreen> createState() =>
      _DeveloperSettingsScreenState();
}

class _DeveloperSettingsScreenState extends State<DeveloperSettingsScreen> {
  String? deviceToken;

  @override
  void initState() {
    super.initState();
    _loadDeviceToken();
  }

  void _loadDeviceToken() async {
    final token = await getDeviceToken();
    setState(() {
      deviceToken = token;
    });
  }

  void _reRegisterPushToken(CoreClient coreClient) async {
    final newDeviceToken = await getDeviceToken();
    if (newDeviceToken != null) {
      if (Platform.isAndroid) {
        final pushToken = PlatformPushToken.google(newDeviceToken);
        print("Push token: $pushToken");
        coreClient.user.updatePushToken(pushToken);
        setState(() {
          deviceToken = pushToken.token;
        });
      } else if (Platform.isIOS) {
        final pushToken = PlatformPushToken.apple(newDeviceToken);
        coreClient.user.updatePushToken(pushToken);
        setState(() {
          deviceToken = pushToken.token;
        });
      } else {
        throw StateError("unsupported platform");
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final user = context.select((LoadableUserCubit cubit) => cubit.state.user);

    final isMobile = Platform.isAndroid || Platform.isIOS;

    return SafeArea(
      child: Scaffold(
        appBar: AppBar(
          title: const Text('Developer Settings'),
          leading: const AppBarBackButton(),
        ),
        body: Center(
          child: Container(
            padding: const EdgeInsets.all(Spacings.xs),
            constraints:
                isPointer() ? const BoxConstraints(maxWidth: 800) : null,
            child: ListView(
              children: [
                if (isMobile) ...[
                  _SectionHeader("Mobile Device"),
                  ListTile(
                    title: const Text('Push Token'),
                    subtitle: Text(
                      "Refresh the current device push token '${deviceToken ?? "N/A"}'.",
                    ),
                    onTap: () =>
                        _reRegisterPushToken(context.read<CoreClient>()),
                  ),
                  const Divider(),
                ],
                if (user != null) ...[
                  _SectionHeader("User"),
                  ListTile(
                    title: Text("Change User"),
                    subtitle: Text(
                      "Change the currently logged in user.",
                    ),
                    onTap: () => context
                        .read<NavigationCubit>()
                        .openDeveloperSettings(
                            screen: DeveloperSettingsScreenType.changeUser),
                  ),
                  ListTile(
                    title: Text("Log Out"),
                    subtitle: Text(
                      "Log out of the currently logged in user.",
                    ),
                    onTap: () => context.read<CoreClient>().logout(),
                  ),
                  const Divider(),
                ],
                _SectionHeader("App Data"),
                if (user != null)
                  ListTile(
                    title: Text('Erase User Database',
                        style: Theme.of(context)
                            .textTheme
                            .bodyLarge
                            ?.copyWith(color: Colors.red)),
                    subtitle: Text(
                      "Erase the database of the currently logged in user '${user.userName}', id: ${user.clientId}'.",
                    ),
                    onTap: () => _confirmDialog(
                      context: context,
                      onConfirm: () =>
                          context.read<CoreClient>().deleteUserDatabase(),
                      label: "Are you sure you want to erase the database?",
                      confirmLabel: "Erase",
                    ),
                  ),
                ListTile(
                  title: Text('Erase All Databases',
                      style: Theme.of(context)
                          .textTheme
                          .bodyLarge
                          ?.copyWith(color: Colors.red)),
                  subtitle: Text(
                    "Erase all databases of all users.",
                  ),
                  onTap: () => _confirmDialog(
                    context: context,
                    onConfirm: () {
                      context.read<CoreClient>().deleteDatabase();
                      context.read<NavigationCubit>().openIntro();
                    },
                    label: "Are you sure you want to erase all databases?",
                    confirmLabel: "Erase",
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

void _confirmDialog({
  required BuildContext context,
  required void Function() onConfirm,
  required String label,
  required String confirmLabel,
}) {
  showDialog(
    context: context,
    builder: (BuildContext context) {
      return AlertDialog(
        title: const Text('Confirmation'),
        content: Text(label),
        actions: [
          TextButton(
            child: const Text('Cancel'),
            onPressed: () {
              Navigator.of(context).pop();
            },
          ),
          TextButton(
            style: TextButton.styleFrom(
              backgroundColor: Colors.red,
              foregroundColor: Colors.white,
            ),
            onPressed: onConfirm,
            child: Text(confirmLabel),
          ),
        ],
      );
    },
  );
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader(this.label);

  final String label;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: Spacings.xxs),
      child: Text(
        label,
        style: Theme.of(context)
            .textTheme
            .labelMedium
            ?.copyWith(fontWeight: FontWeight.bold),
      ),
    );
  }
}
