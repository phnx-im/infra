// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:air/theme/spacings.dart';
import 'package:air/ui/colors/themes.dart';
import 'package:air/ui/components/context_menu/context_menu_item_ui.dart';
import 'package:air/ui/effects/elevation.dart';

class ContextMenuUi extends StatelessWidget {
  const ContextMenuUi({
    super.key,
    required this.menuItems,
    required this.onHide,
  });

  final List<ContextMenuItem> menuItems;
  final VoidCallback onHide;

  @override
  Widget build(BuildContext context) {
    return Container(
      clipBehavior: Clip.hardEdge,
      decoration: BoxDecoration(
        color: CustomColorScheme.of(context).backgroundElevated.primary,
        boxShadow: elevationBoxShadows(context),
        borderRadius: BorderRadius.circular(16),
      ),
      padding: const EdgeInsets.symmetric(
        horizontal: Spacings.s,
        vertical: Spacings.xs,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          for (final (i, item) in menuItems.indexed) ...[
            item,
            if (i < menuItems.length - 1)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: Spacings.xxs),
                child: Divider(
                  height: 0,
                  thickness: 1,
                  color: CustomColorScheme.of(context).separator.primary,
                ),
              ),
          ],
        ],
      ),
    );
  }
}
