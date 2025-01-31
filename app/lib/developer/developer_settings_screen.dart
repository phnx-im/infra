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

final _titleFontWeight = VariableFontWeight.medium;

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

  @override
  build(BuildContext context) {
    return DeveloperSettingsScreenView(
      deviceToken: deviceToken,
      isMobile: Platform.isAndroid || Platform.isIOS,
      onRefreshPushToken: () =>
          _reRegisterPushToken(context.read<CoreClient>()),
    );
  }

  void _reRegisterPushToken(CoreClient coreClient) async {
    final newDeviceToken = await getDeviceToken();
    if (newDeviceToken != null) {
      if (Platform.isAndroid) {
        final pushToken = PlatformPushToken.google(newDeviceToken);
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
}

class DeveloperSettingsScreenView extends StatelessWidget {
  const DeveloperSettingsScreenView({
    required this.deviceToken,
    required this.onRefreshPushToken,
    required this.isMobile,
    super.key,
  });

  final String? deviceToken;
  final bool isMobile;
  final VoidCallback onRefreshPushToken;

  @override
  Widget build(BuildContext context) {
    final user = context.select((LoadableUserCubit cubit) => cubit.state.user);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Developer Settings'),
        leading: const AppBarBackButton(),
      ),
      body: SafeArea(
        child: Center(
          child: Container(
            constraints:
                isPointer() ? const BoxConstraints(maxWidth: 800) : null,
            child: ListTileTheme(
              data: Theme.of(context).listTileTheme.copyWith(
                    titleAlignment: ListTileTitleAlignment.titleHeight,
                    titleTextStyle: Theme.of(context)
                        .textTheme
                        .bodyLarge!
                        .merge(_titleFontWeight),
                  ),
              child: ListView(
                children: [
                  if (isMobile) ...[
                    _SectionHeader("Mobile Device"),
                    ListTile(
                      title: const Text('Push Token'),
                      subtitle: Text(deviceToken ?? "N/A"),
                      trailing: const Icon(Icons.refresh),
                      onTap: onRefreshPushToken,
                    ),
                  ],
                  if (user != null) ...[
                    _SectionHeader("User"),
                    ListTile(
                      title: Text("Change User"),
                      trailing: const Icon(Icons.change_circle),
                      onTap: () => context
                          .read<NavigationCubit>()
                          .openDeveloperSettings(
                              screen: DeveloperSettingsScreenType.changeUser),
                    ),
                    ListTile(
                      title: Text("Log Out"),
                      trailing: const Icon(Icons.logout),
                      onTap: () => context.read<CoreClient>().logout(),
                    ),
                  ],
                  _SectionHeader("App Data"),
                  if (user != null)
                    ListTile(
                      title: Text(
                        user.userName,
                        style: Theme.of(context)
                            .textTheme
                            .bodyLarge
                            ?.copyWith(
                              color: Colors.red,
                            )
                            .merge(_titleFontWeight),
                      ),
                      subtitle: Text("id: ${user.clientId}"),
                      trailing: const Icon(Icons.delete),
                      onTap: () => _confirmDialog(
                        context: context,
                        onConfirm: () =>
                            context.read<CoreClient>().deleteUserDatabase(),
                        label: "Are you sure you want to erase the database?",
                        confirmLabel: "Erase",
                      ),
                    ),
                  ListTile(
                    title: Text(
                      'Erase All Databases',
                      style: Theme.of(context)
                          .textTheme
                          .bodyLarge
                          ?.copyWith(
                            color: Colors.red,
                          )
                          .merge(_titleFontWeight),
                    ),
                    trailing: const Icon(Icons.delete),
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
      padding: const EdgeInsets.symmetric(
        vertical: Spacings.xxs,
        horizontal: Spacings.xs,
      ),
      child: Text(
        label,
        style: Theme.of(context)
            .textTheme
            .labelMedium
            ?.merge(VariableFontWeight.bold),
      ),
    );
  }
}
