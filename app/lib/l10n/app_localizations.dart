// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[Locale('en')];

  /// No description provided for @appTitle.
  ///
  /// In en, this message translates to:
  /// **'Prototype'**
  String get appTitle;

  /// No description provided for @userSettingsScreen_title.
  ///
  /// In en, this message translates to:
  /// **'User Settings'**
  String get userSettingsScreen_title;

  /// No description provided for @userSettingsScreen_idCopied.
  ///
  /// In en, this message translates to:
  /// **'User ID copied to clipboard'**
  String get userSettingsScreen_idCopied;

  /// No description provided for @userSettingsScreen_profileDescription.
  ///
  /// In en, this message translates to:
  /// **'Others will see your picture and name when you communicate with them.'**
  String get userSettingsScreen_profileDescription;

  /// No description provided for @userSettingsScreen_userNamesDescription.
  ///
  /// In en, this message translates to:
  /// **'Share usernames with others so they can connect with you. After the connection, usernames are not visible to others anymore. You can have up to 5 usernames.'**
  String get userSettingsScreen_userNamesDescription;

  /// No description provided for @removeUsernameDialog_title.
  ///
  /// In en, this message translates to:
  /// **'Remove Username'**
  String get removeUsernameDialog_title;

  /// No description provided for @removeUsernameDialog_content.
  ///
  /// In en, this message translates to:
  /// **'If you continue, your username will be removed and may be claimed by someone else. You’ll no longer be reachable through it.'**
  String get removeUsernameDialog_content;

  /// No description provided for @removeUsernameDialog_cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get removeUsernameDialog_cancel;

  /// No description provided for @removeUsernameDialog_remove.
  ///
  /// In en, this message translates to:
  /// **'Remove'**
  String get removeUsernameDialog_remove;

  /// No description provided for @userSettingsScreen_noUserHandles.
  ///
  /// In en, this message translates to:
  /// **'no user handles yet'**
  String get userSettingsScreen_noUserHandles;

  /// No description provided for @userSettingsScreen_userHandlePlaceholder.
  ///
  /// In en, this message translates to:
  /// **'Username'**
  String get userSettingsScreen_userHandlePlaceholder;

  /// No description provided for @addMembersScreen_title.
  ///
  /// In en, this message translates to:
  /// **'Add members'**
  String get addMembersScreen_title;

  /// No description provided for @addMembersScreen_addMembers.
  ///
  /// In en, this message translates to:
  /// **'Add member(s)'**
  String get addMembersScreen_addMembers;

  /// No description provided for @addMembersScreen_error_noActiveConversation.
  ///
  /// In en, this message translates to:
  /// **'an active conversation is obligatory'**
  String get addMembersScreen_error_noActiveConversation;

  /// No description provided for @conversationDetailsScreen_title.
  ///
  /// In en, this message translates to:
  /// **'Details'**
  String get conversationDetailsScreen_title;

  /// No description provided for @conversationDetailsScreen_unknownConversation.
  ///
  /// In en, this message translates to:
  /// **'Unknown conversation'**
  String get conversationDetailsScreen_unknownConversation;

  /// No description provided for @conversationScreen_emptyConversation.
  ///
  /// In en, this message translates to:
  /// **'Select a chat to start messaging'**
  String get conversationScreen_emptyConversation;

  /// No description provided for @memberDetailsScreen_title.
  ///
  /// In en, this message translates to:
  /// **'Member details'**
  String get memberDetailsScreen_title;

  /// No description provided for @memberDetailsScreen_error.
  ///
  /// In en, this message translates to:
  /// **'No member details for intro screen'**
  String get memberDetailsScreen_error;

  /// No description provided for @removeUserDialog_title.
  ///
  /// In en, this message translates to:
  /// **'Remove user'**
  String get removeUserDialog_title;

  /// No description provided for @removeUserDialog_content.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to remove this user from the group?'**
  String get removeUserDialog_content;

  /// No description provided for @removeUserDialog_cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get removeUserDialog_cancel;

  /// No description provided for @removeUserDialog_removeUser.
  ///
  /// In en, this message translates to:
  /// **'Remove user'**
  String get removeUserDialog_removeUser;

  /// No description provided for @removeUserButton_text.
  ///
  /// In en, this message translates to:
  /// **'Remove user'**
  String get removeUserButton_text;

  /// No description provided for @introScreen_developerSettings.
  ///
  /// In en, this message translates to:
  /// **'Developer Settings'**
  String get introScreen_developerSettings;

  /// No description provided for @introScreen_signUp.
  ///
  /// In en, this message translates to:
  /// **'Sign up'**
  String get introScreen_signUp;

  /// No description provided for @userHandleScreen_title.
  ///
  /// In en, this message translates to:
  /// **'Username'**
  String get userHandleScreen_title;

  /// No description provided for @userHandleScreen_inputHint.
  ///
  /// In en, this message translates to:
  /// **'Username'**
  String get userHandleScreen_inputHint;

  /// No description provided for @userHandleScreen_error_emptyHandle.
  ///
  /// In en, this message translates to:
  /// **'User handle cannot be empty'**
  String get userHandleScreen_error_emptyHandle;

  /// No description provided for @userHandleScreen_description.
  ///
  /// In en, this message translates to:
  /// **'Choose a username that others can use to connect with you.\n\nUse letters, numbers, or underscores. Minimum 5 characters.'**
  String get userHandleScreen_description;

  /// No description provided for @userHandleScreen_save.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get userHandleScreen_save;

  /// No description provided for @editDisplayNameScreen_title.
  ///
  /// In en, this message translates to:
  /// **'Display Name'**
  String get editDisplayNameScreen_title;

  /// No description provided for @editDisplayNameScreen_hintText.
  ///
  /// In en, this message translates to:
  /// **'Display name'**
  String get editDisplayNameScreen_hintText;

  /// No description provided for @editDisplayNameScreen_description.
  ///
  /// In en, this message translates to:
  /// **'Choose a name that others will see when you communicate with them.'**
  String get editDisplayNameScreen_description;

  /// No description provided for @editDisplayNameScreen_save.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get editDisplayNameScreen_save;

  /// No description provided for @systemMessage_userAddedUser_prefix.
  ///
  /// In en, this message translates to:
  /// **''**
  String get systemMessage_userAddedUser_prefix;

  /// No description provided for @systemMessage_userAddedUser_infix.
  ///
  /// In en, this message translates to:
  /// **' added '**
  String get systemMessage_userAddedUser_infix;

  /// No description provided for @systemMessage_userAddedUser_suffix.
  ///
  /// In en, this message translates to:
  /// **''**
  String get systemMessage_userAddedUser_suffix;

  /// No description provided for @systemMessage_userRemovedUser_prefix.
  ///
  /// In en, this message translates to:
  /// **''**
  String get systemMessage_userRemovedUser_prefix;

  /// No description provided for @systemMessage_userRemovedUser_infix.
  ///
  /// In en, this message translates to:
  /// **' removed '**
  String get systemMessage_userRemovedUser_infix;

  /// No description provided for @systemMessage_userRemovedUser_suffix.
  ///
  /// In en, this message translates to:
  /// **''**
  String get systemMessage_userRemovedUser_suffix;

  /// No description provided for @timestamp_now.
  ///
  /// In en, this message translates to:
  /// **'Now'**
  String get timestamp_now;

  /// No description provided for @timestamp_yesterday.
  ///
  /// In en, this message translates to:
  /// **'Yesterday'**
  String get timestamp_yesterday;

  /// No description provided for @conversationList_newConnection.
  ///
  /// In en, this message translates to:
  /// **'New connection'**
  String get conversationList_newConnection;

  /// No description provided for @conversationList_newConversation.
  ///
  /// In en, this message translates to:
  /// **'New conversation'**
  String get conversationList_newConversation;

  /// No description provided for @newConversationDialog_newConversationTitle.
  ///
  /// In en, this message translates to:
  /// **'New conversation'**
  String get newConversationDialog_newConversationTitle;

  /// No description provided for @newConversationDialog_newConversationDescription.
  ///
  /// In en, this message translates to:
  /// **'Choose a name for the new conversation'**
  String get newConversationDialog_newConversationDescription;

  /// No description provided for @newConversationDialog_conversationNamePlaceholder.
  ///
  /// In en, this message translates to:
  /// **'Conversation name'**
  String get newConversationDialog_conversationNamePlaceholder;

  /// No description provided for @newConversationDialog_actionButton.
  ///
  /// In en, this message translates to:
  /// **'Create conversation'**
  String get newConversationDialog_actionButton;

  /// No description provided for @newConversationDialog_error.
  ///
  /// In en, this message translates to:
  /// **'Failed to add conversation with name {conversationName}: {error}'**
  String newConversationDialog_error(Object conversationName, Object error);

  /// No description provided for @newConnectionDialog_newConnectionTitle.
  ///
  /// In en, this message translates to:
  /// **'New connection'**
  String get newConnectionDialog_newConnectionTitle;

  /// No description provided for @newConnectionDialog_newConnectionDescription.
  ///
  /// In en, this message translates to:
  /// **'Enter the Username of the user you want to connect to'**
  String get newConnectionDialog_newConnectionDescription;

  /// No description provided for @newConnectionDialog_usernamePlaceholder.
  ///
  /// In en, this message translates to:
  /// **'Username'**
  String get newConnectionDialog_usernamePlaceholder;

  /// No description provided for @newConnectionDialog_actionButton.
  ///
  /// In en, this message translates to:
  /// **'Connect'**
  String get newConnectionDialog_actionButton;

  /// No description provided for @newConnectionDialog_error.
  ///
  /// In en, this message translates to:
  /// **'Failed to add user with Username {username}: {error}'**
  String newConnectionDialog_error(Object error, Object username);
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
