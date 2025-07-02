// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'Prototype';

  @override
  String get userSettingsScreen_title => 'User Settings';

  @override
  String get userSettingsScreen_idCopied => 'User ID copied to clipboard';

  @override
  String get userSettingsScreen_profileDescription =>
      'Others will see your picture and name when you communicate with them.';

  @override
  String get userSettingsScreen_userNamesDescription =>
      'Share usernames with others so they can connect with you. After the connection, usernames are not visible to others anymore. You can have up to 5 usernames.';

  @override
  String get userSettingsScreen_interfaceScale => 'Interface scale';

  @override
  String get removeUsernameDialog_title => 'Remove Username';

  @override
  String get removeUsernameDialog_content =>
      'If you continue, your username will be removed and may be claimed by someone else. You’ll no longer be reachable through it.';

  @override
  String get removeUsernameDialog_cancel => 'Cancel';

  @override
  String get removeUsernameDialog_remove => 'Remove';

  @override
  String get userSettingsScreen_noUserHandles => 'no user handles yet';

  @override
  String get userSettingsScreen_userHandlePlaceholder => 'Username';

  @override
  String get addMembersScreen_title => 'Add members';

  @override
  String get addMembersScreen_addMembers => 'Add member(s)';

  @override
  String get addMembersScreen_error_noActiveConversation =>
      'an active conversation is obligatory';

  @override
  String get conversationDetailsScreen_title => 'Details';

  @override
  String get conversationDetailsScreen_unknownConversation =>
      'Unknown conversation';

  @override
  String get conversationScreen_emptyConversation =>
      'Select a chat to start messaging';

  @override
  String get memberDetailsScreen_title => 'Member details';

  @override
  String get memberDetailsScreen_error => 'No member details for intro screen';

  @override
  String get removeUserDialog_title => 'Remove user';

  @override
  String get removeUserDialog_content =>
      'Are you sure you want to remove this user from the group?';

  @override
  String get removeUserDialog_cancel => 'Cancel';

  @override
  String get removeUserDialog_removeUser => 'Remove user';

  @override
  String get removeUserButton_text => 'Remove user';

  @override
  String get introScreen_signUp => 'Sign up';

  @override
  String get userHandleScreen_title => 'Username';

  @override
  String get userHandleScreen_inputHint => 'Username';

  @override
  String get userHandleScreen_error_emptyHandle => 'Username cannot be empty';

  @override
  String get userHandleScreen_description =>
      'Choose a username that others can use to connect with you.\n\nUse letters, numbers, or underscores. Minimum 5 characters.';

  @override
  String get userHandleScreen_save => 'Save';

  @override
  String get editDisplayNameScreen_title => 'Display Name';

  @override
  String get editDisplayNameScreen_hintText => 'Display name';

  @override
  String get editDisplayNameScreen_description =>
      'Choose a name that others will see when you communicate with them.';

  @override
  String get editDisplayNameScreen_save => 'Save';

  @override
  String get systemMessage_userAddedUser_prefix => '';

  @override
  String get systemMessage_userAddedUser_infix => ' added ';

  @override
  String get systemMessage_userAddedUser_suffix => '';

  @override
  String get systemMessage_userRemovedUser_prefix => '';

  @override
  String get systemMessage_userRemovedUser_infix => ' removed ';

  @override
  String get systemMessage_userRemovedUser_suffix => '';

  @override
  String get timestamp_now => 'Now';

  @override
  String get timestamp_yesterday => 'Yesterday';

  @override
  String get conversationList_newContact => 'New contact';

  @override
  String get conversationList_newGroup => 'New group';

  @override
  String get settings_profile => 'Profile';

  @override
  String get settings_developerSettings => 'Settings (developer)';

  @override
  String get newConversationDialog_newConversationTitle => 'New conversation';

  @override
  String get newConversationDialog_newConversationDescription =>
      'Choose a name for the new conversation';

  @override
  String get newConversationDialog_conversationNamePlaceholder =>
      'Conversation name';

  @override
  String get newConversationDialog_actionButton => 'Create conversation';

  @override
  String newConversationDialog_error(Object conversationName) {
    return 'Failed to add conversation with name $conversationName';
  }

  @override
  String get newConversationDialog_error_emptyGroupName =>
      'Conversation name cannot be empty';

  @override
  String get newConnectionDialog_newConnectionTitle => 'New connection';

  @override
  String get newConnectionDialog_newConnectionDescription =>
      'Enter the Username of the user you want to connect to';

  @override
  String get newConnectionDialog_usernamePlaceholder => 'Username';

  @override
  String get newConnectionDialog_actionButton => 'Connect';

  @override
  String newConnectionDialog_error(Object username) {
    return 'Failed to add user with Username $username. Please try again.';
  }

  @override
  String get newConnectionDialog_error_emptyHandle =>
      'Username cannot be empty';

  @override
  String newConnectionDialog_error_handleNotFound(Object username) {
    return 'Username $username does not exist';
  }

  @override
  String get composer_error_attachment =>
      'Failed to upload attachment. Please try again.';

  @override
  String attachmentSize(double size, Object byteUnit) {
    final intl.NumberFormat sizeNumberFormat = intl
        .NumberFormat.decimalPatternDigits(
      locale: localeName,
      decimalDigits: 2,
    );
    final String sizeString = sizeNumberFormat.format(size);

    return '$sizeString $byteUnit';
  }
}
