// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:air/theme/theme.dart';
import 'package:air/ui/colors/themes.dart';

class CreateConversationView extends StatefulWidget {
  final String title;
  final String prompt;
  final String hint;
  final String action;
  final Future<String?> Function(String)? onAction;
  final FormFieldValidator<String> validator;

  @override
  const CreateConversationView(
    BuildContext context,
    this.title,
    this.prompt,
    this.hint,
    this.action, {
    this.onAction,
    required this.validator,
    super.key,
  });

  @override
  State<CreateConversationView> createState() => _CreateConversationViewState();
}

class _CreateConversationViewState extends State<CreateConversationView> {
  final _formKey = GlobalKey<FormState>();

  bool _isInputValid = false;
  String? _customValidationError;

  final TextEditingController _controller = TextEditingController();

  void _validateForm() {
    setState(() {
      _isInputValid = _formKey.currentState?.validate() ?? false;
    });
  }

  @override
  Widget build(context) {
    return AlertDialog(
      title: Text(widget.title),
      titlePadding: const EdgeInsets.all(20),
      titleTextStyle: Theme.of(context).textTheme.titleLarge?.copyWith(
        color: CustomColorScheme.of(context).text.secondary,
      ),
      actionsAlignment: MainAxisAlignment.spaceBetween,
      actionsPadding: const EdgeInsets.all(20),
      buttonPadding: const EdgeInsets.symmetric(horizontal: 20, vertical: 20),
      contentPadding: const EdgeInsets.symmetric(horizontal: 20, vertical: 10),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const SizedBox(height: 50),
              Text(
                widget.prompt,
                style: Theme.of(context).textTheme.bodyMedium,
              ),
              const SizedBox(height: 20),
              Form(
                key: _formKey,
                autovalidateMode: AutovalidateMode.onUserInteraction,
                child: ConstrainedBox(
                  constraints: BoxConstraints.tight(const Size(380, 80)),
                  child: TextFormField(
                    autofocus: true,
                    controller: _controller,
                    decoration: InputDecoration(hintText: widget.hint),
                    validator: _validator,
                    onChanged: (String value) => _validateForm(),
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
      actions: <Widget>[
        TextButton(
          style: dynamicTextButtonStyle(context, true, false),
          child: const Text('Cancel'),
          onPressed: () {
            Navigator.of(context).pop(null);
          },
        ),
        TextButton(
          style: dynamicTextButtonStyle(context, _isInputValid, true),
          onPressed: _isInputValid ? _onAction : null,
          child: Text(widget.action),
        ),
      ],
    );
  }

  void _onAction() async {
    if (widget.onAction != null) {
      final error = await widget.onAction!(_controller.text);
      if (error != null) {
        setState(() {
          _isInputValid = false;
          _customValidationError = error;
        });
      }
    } else {
      Navigator.of(context).pop(_controller.text);
    }
  }

  String? _validator(String? input) {
    if (_customValidationError != null) {
      final error = _customValidationError;
      _customValidationError = null;
      return error;
    } else {
      return widget.validator(input);
    }
  }
}
