// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';
import 'package:flutter/material.dart';
import 'package:prototype/theme/spacings.dart';
import 'package:prototype/ui/colors/themes.dart';
import 'package:prototype/ui/typography/font_size.dart';

class ContextMenuItem extends StatelessWidget {
  const ContextMenuItem({
    super.key,
    required this.onPressed,
    required this.label,
    this.leadingIcon,
    this.trailingIcon,
  });

  final VoidCallback onPressed;
  final String label;
  final IconData? leadingIcon;
  final IconData? trailingIcon;

  @override
  Widget build(BuildContext context) {
    return TextButton(
      onPressed: onPressed,
      style: TextButton.styleFrom(
        shape: const RoundedRectangleBorder(borderRadius: BorderRadius.zero),
        foregroundColor: customColors(context).text.primary,
        padding: const EdgeInsets.symmetric(vertical: Spacings.s),
        alignment: Alignment.centerLeft,
        splashFactory: !Platform.isAndroid ? NoSplash.splashFactory : null,
        overlayColor: Colors.transparent,
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Row(
            spacing: Spacings.xxs,
            children: [
              Icon(leadingIcon, size: 24),
              Text(label, style: TextStyle(fontSize: LabelFontSize.base.size)),
            ],
          ),
          Icon(trailingIcon),
        ],
      ),
    );
  }

  ContextMenuItem copyWith({required Null Function() onPressed}) {
    return ContextMenuItem(
      onPressed: onPressed,
      label: label,
      leadingIcon: leadingIcon,
      trailingIcon: trailingIcon,
    );
  }
}
