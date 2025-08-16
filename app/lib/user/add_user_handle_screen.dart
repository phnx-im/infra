// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/l10n/l10n.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/ui/colors/themes.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

class AddUserHandleScreen extends StatefulWidget {
  const AddUserHandleScreen({super.key});

  @override
  State<AddUserHandleScreen> createState() => _AddUserHandleScreenState();
}

class _AddUserHandleScreenState extends State<AddUserHandleScreen> {
  final _formKey = GlobalKey<FormState>();
  final _controller = TextEditingController();
  bool _alreadyExists = false;

  @override
  void initState() {
    super.initState();
    // Clear already exists flag when the text field changes
    _controller.addListener(() {
      if (_alreadyExists) {
        setState(() {
          _alreadyExists = false;
        });
      }
    });
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(
        title: Text(loc.userHandleScreen_title),
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
            child: Form(
              key: _formKey,
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  const _UserHandleAvatar(),
                  const SizedBox(height: Spacings.m),
                  TextFormField(
                    autofocus: true,
                    controller: _controller,
                    decoration: InputDecoration(
                      hintText: loc.userHandleScreen_inputHint,
                    ),
                    validator: (value) {
                      if (_alreadyExists) {
                        return 'Username already exists';
                      }
                      if (value == null || value.trim().isEmpty) {
                        return loc.userHandleScreen_error_emptyHandle;
                      }
                      final handle = UiUserHandle(
                        plaintext: value.trim().toLowerCase(),
                      );
                      return handle.validationError();
                    },
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
                        loc.userHandleScreen_description,
                      ),
                    ),
                  ),
                  const Spacer(),
                  OutlinedButton(
                    onPressed: () => _submit(context),
                    style: buttonStyle(CustomColorScheme.of(context), true),
                    child: Text(loc.userHandleScreen_save),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  void _submit(BuildContext context) async {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    final handle = UiUserHandle(
      plaintext: _controller.text.trim().toLowerCase(),
    );
    final userCubit = context.read<UserCubit>();
    final navigationCubit = context.read<NavigationCubit>();
    if (!await userCubit.addUserHandle(handle)) {
      setState(() {
        _alreadyExists = true;
      });
      _formKey.currentState!.validate();
      return;
    }
    navigationCubit.pop();
  }
}

class _UserHandleAvatar extends StatelessWidget {
  const _UserHandleAvatar();

  static const _size = 100.0;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: _size,
      height: _size,
      child: CircleAvatar(
        radius: _size / 2,
        backgroundColor: CustomColorScheme.of(context).text.quaternary,
        child: const Icon(Icons.alternate_email, size: _size / 1.5),
      ),
    );
  }
}
