// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:logging/logging.dart';
import 'package:prototype/conversation_list/context_menu.dart';
import 'package:prototype/conversation_list/conversation_list_cubit.dart';
import 'package:prototype/conversation_list/create_conversation_view.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/l10n/l10n.dart';
import 'package:prototype/main.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

final _log = Logger("ConversationListHeader");

class ConversationListHeader extends StatelessWidget {
  const ConversationListHeader({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.only(left: Spacings.xxs, right: Spacings.s),
      child: const Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          _Avatar(),
          Expanded(child: _DisplayNameSpace()),
          _SettingsButton(),
        ],
      ),
    );
  }
}

class _Avatar extends StatefulWidget {
  const _Avatar();

  @override
  State<_Avatar> createState() => _AvatarState();
}

class _AvatarState extends State<_Avatar> {
  final contextMenuController = OverlayPortalController();

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);
    final profile = context.select((UsersCubit cubit) => cubit.state.profile());

    return Padding(
      padding: const EdgeInsets.only(left: Spacings.sm),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          ContextMenu(
            direction: ContextMenuDirection.right,
            width: 200,
            controller: contextMenuController,
            menuItems: [
              ContextMenuItem(
                label: loc.settings_profile,
                onPressed: () {
                  context.read<NavigationCubit>().openUserSettings();
                },
              ),
              ContextMenuItem(
                label: loc.settings_developerSettings,
                onPressed: () {
                  context.read<NavigationCubit>().openDeveloperSettings();
                },
              ),
            ],
            child: UserAvatar(
              displayName: profile.displayName,
              image: profile.profilePicture,
              size: Spacings.l,
              onPressed: () {
                contextMenuController.show();
              },
            ),
          ),
        ],
      ),
    );
  }
}

class _DisplayNameSpace extends StatelessWidget {
  const _DisplayNameSpace();

  @override
  Widget build(BuildContext context) {
    final displayName = context.select(
      (UsersCubit cubit) => cubit.state.displayName(),
    );

    return Text(
      displayName,
      style: const TextStyle(
        color: colorDMB,
        fontSize: 13,
      ).merge(VariableFontWeight.bold),
      overflow: TextOverflow.ellipsis,
      textAlign: TextAlign.center,
    );
  }
}

class _SettingsButton extends StatefulWidget {
  const _SettingsButton();

  @override
  State<_SettingsButton> createState() => _SettingsButtonState();
}

class _SettingsButtonState extends State<_SettingsButton> {
  final contextMenuController = OverlayPortalController();

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);

    return ContextMenu(
      direction: ContextMenuDirection.left,
      width: 200,
      controller: contextMenuController,
      menuItems: [
        ContextMenuItem(
          label: loc.conversationList_newContact,
          onPressed: () {
            _newContact(context);
          },
        ),
        ContextMenuItem(
          label: loc.conversationList_newGroup,
          onPressed: () {
            _newGroup(context);
          },
        ),
      ],
      child: IconButton(
        onPressed: () {
          contextMenuController.show();
        },
        hoverColor: Colors.transparent,
        focusColor: Colors.transparent,
        splashColor: Colors.transparent,
        highlightColor: Colors.transparent,
        icon: const Icon(Icons.add_circle_rounded, size: 24, color: colorDMB),
      ),
    );
  }

  void _newContact(BuildContext context) {
    final conversationListCubit = context.read<ConversationListCubit>();
    final loc = AppLocalizations.of(context);

    String? customError;

    String? validator(String? value) {
      final plaintext = value?.trim().toLowerCase();
      if (plaintext == null || plaintext.isEmpty) {
        return loc.newConnectionDialog_error_emptyHandle;
      }
      if (customError != null) {
        final error = customError;
        customError = null;
        return error;
      }
      UiUserHandle handle = UiUserHandle(plaintext: plaintext);
      return handle.validationError();
    }

    Future<String?> onAction(String input) async {
      final handle = UiUserHandle(plaintext: input.trim().toLowerCase());
      try {
        final conversationId = await conversationListCubit.createConnection(
          handle: handle,
        );
        if (context.mounted) {
          if (conversationId == null) {
            return loc.newConnectionDialog_error_handleNotFound(
              handle.plaintext,
            );
          }
          _log.info(
            "A new 1:1 connection with user '${handle.plaintext}' was created: "
            "conversationId = $conversationId",
          );
          Navigator.of(context).pop();
        }
      } catch (e) {
        _log.severe("Failed to create connection: $e");
        if (context.mounted) {
          showErrorBanner(
            ScaffoldMessenger.of(context),
            loc.newConnectionDialog_error(handle.plaintext),
          );
        }
      }
      return null;
    }

    showDialog(
      context: context,
      builder:
          (BuildContext context) => CreateConversationView(
            context,
            loc.newConnectionDialog_newConnectionTitle,
            loc.newConnectionDialog_newConnectionDescription,
            loc.newConnectionDialog_usernamePlaceholder,
            loc.newConnectionDialog_actionButton,
            validator: validator,
            onAction: onAction,
          ),
    );
  }

  void _newGroup(BuildContext context) async {
    final conversationListCubit = context.read<ConversationListCubit>();
    final loc = AppLocalizations.of(context);

    validator(String? value) {
      final plaintext = value?.trim().toLowerCase();
      if (plaintext == null || plaintext.isEmpty) {
        return loc.newConversationDialog_error_emptyGroupName;
      }
      return null;
    }

    String? input = await showDialog(
      context: context,
      builder:
          (BuildContext context) => CreateConversationView(
            context,
            loc.newConversationDialog_newConversationTitle,
            loc.newConversationDialog_newConversationDescription,
            loc.newConversationDialog_conversationNamePlaceholder,
            loc.newConversationDialog_actionButton,
            validator: validator,
          ),
    );
    String groupName = input?.trim() ?? "";
    if (groupName.isNotEmpty) {
      try {
        final conversationId = await conversationListCubit.createConversation(
          groupName: groupName,
        );
        _log.info(
          "A new group '$groupName' was created: "
          "conversationId = $conversationId",
        );
      } catch (e) {
        if (context.mounted) {
          _log.severe("Failed to create conversation: $e");
          showErrorBanner(
            ScaffoldMessenger.of(context),
            loc.newConversationDialog_error(groupName),
          );
        }
      }
    }
  }
}
