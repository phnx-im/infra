// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:prototype/theme/spacings.dart';
import 'package:prototype/theme/styles.dart';
import 'package:prototype/user/user.dart';
import 'package:provider/provider.dart';

enum ContextMenuDirection { left, right }

class ContextMenuAnchor extends StatefulWidget {
  const ContextMenuAnchor({
    super.key,
    this.direction = ContextMenuDirection.left,
    required this.menuItems,
    required this.child,
  });

  final ContextMenuDirection direction;
  final List<ContextMenuItem> menuItems;
  final Widget child;

  @override
  State<ContextMenuAnchor> createState() => _ContextMenuAnchorState();
}

class _ContextMenuAnchorState extends State<ContextMenuAnchor> {
  final _controller = ContextMenuController();

  @override
  void initState() {
    super.initState();
    _controller.attach(context);
  }

  @override
  void dispose() {
    _controller.detach();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTapDown: (details) {
        final tapPosition = details.globalPosition;
        _controller.showMenu(
          direction: widget.direction,
          menuItems: widget.menuItems,
          position: tapPosition,
        );
      },
      child: widget.child,
    );
  }
}

class ContextMenuController extends ChangeNotifier {
  BuildContext? _context;

  OverlayEntry? _overlayEntry;

  void attach(BuildContext context) {
    _context = context;
    ServicesBinding.instance.keyboard.addHandler(_onKeyEvent);
  }

  void detach() {
    _context = null;
    ServicesBinding.instance.keyboard.removeHandler(_onKeyEvent);
  }

  bool _onKeyEvent(KeyEvent event) {
    if (event.logicalKey == LogicalKeyboardKey.escape) {
      hideMenu();
      return true;
    }
    return false;
  }

  /// Renders the menu in two passes:
  ///
  /// 1. Render the menu off screen to calculate the size of the menu.
  /// 2. Render the menu at the calculated position.
  ///
  /// The menu anchor is the tap position attached to the top-left resp. top-right corner depending
  /// on the direction.
  void showMenu({
    required Offset position,
    required List<ContextMenuItem> menuItems,
    ContextMenuDirection direction = ContextMenuDirection.left,
  }) {
    final menuKey = GlobalKey();

    hideMenu();

    assert(_context != null, "Context is not attached");
    final context = _context!;

    // Render the menu off screen to calculate the size of the menu.
    _overlayEntry = _createOverlayEntry(
      context,
      const Offset(-10000, -10000),
      menuKey,
      menuItems,
    );
    Overlay.of(_context!).insert(_overlayEntry!);

    // Schedule a post-frame callback to reposition the menu after the render pass.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final RenderBox? box =
          menuKey.currentContext?.findRenderObject() as RenderBox?;
      final Size widgetSize = box?.size ?? Size.zero;

      final screenSize = MediaQuery.of(context).size;

      // Based on the direction, take the left top or the right top corner as menu position.
      var (dx, dy) = switch (direction) {
        ContextMenuDirection.left => (position.dx, position.dy),
        ContextMenuDirection.right => (
          position.dx - widgetSize.width,
          position.dy,
        ),
      };

      // Make sure the menu does not overlap the screen edges.
      if (dx + widgetSize.width > screenSize.width) {
        dx = (screenSize.width - widgetSize.width).clamp(0.0, screenSize.width);
      }
      if (dy + widgetSize.height > screenSize.height) {
        dy = (screenSize.height - widgetSize.height).clamp(
          0.0,
          screenSize.height,
        );
      }

      // Take the scale into account.
      final scale = context.read<UserSettingsCubit>().state.interfaceScale;
      dx /= scale;
      dy /= scale;

      // Re-render the menu at the calculated position.
      _overlayEntry?.remove();
      _overlayEntry = _createOverlayEntry(
        context,
        Offset(dx, dy),
        menuKey,
        menuItems,
      );
      Overlay.of(context).insert(_overlayEntry!);
    });
  }

  void hideMenu() {
    _overlayEntry?.remove();
    _overlayEntry = null;
  }

  OverlayEntry _createOverlayEntry(
    BuildContext context,
    Offset offset,
    GlobalKey menuKey,
    List<ContextMenuItem> menuItems,
  ) {
    return OverlayEntry(
      builder:
          (context) => Stack(
            children: [
              // Fullscreen hit test area
              Positioned.fill(
                child: GestureDetector(
                  behavior: HitTestBehavior.translucent,
                  onTap: hideMenu,
                ),
              ),

              // Menu
              Positioned(
                left: offset.dx,
                top: offset.dy,
                child: ContextMenu(menuItems: menuItems, hideMenu: hideMenu),
              ),
            ],
          ),
    );
  }
}

class ContextMenu extends StatelessWidget {
  const ContextMenu({
    super.key,
    required this.menuItems,
    required this.hideMenu,
  });

  final List<ContextMenuItem> menuItems;
  final VoidCallback hideMenu;

  @override
  Widget build(BuildContext context) {
    return IntrinsicWidth(
      child: Container(
        clipBehavior: Clip.hardEdge,
        decoration: BoxDecoration(
          color: convPaneBackgroundColor,
          boxShadow: const [
            BoxShadow(
              color: Colors.black54,
              blurRadius: 64,
              offset: Offset(0, 4),
            ),
          ],
          borderRadius: BorderRadius.circular(16),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (final (i, item) in menuItems.indexed) ...[
              ContextMenuItem(
                onPressed: () {
                  item.onPressed();
                  hideMenu();
                },
                label: item.label,
              ),
              if (i < menuItems.length - 1)
                const Divider(height: 0, thickness: 1, color: colorGreyLight),
            ],
          ],
        ),
      ),
    );
  }
}

class ContextMenuItem extends StatelessWidget {
  const ContextMenuItem({
    super.key,
    required this.label,
    required this.onPressed,
  });

  final String label;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return TextButton(
      onPressed: onPressed,
      style: TextButton.styleFrom(
        shape: const RoundedRectangleBorder(borderRadius: BorderRadius.zero),
        foregroundColor: Colors.black87,
        padding: const EdgeInsets.symmetric(
          horizontal: Spacings.sm,
          vertical: Spacings.s,
        ),
        alignment: Alignment.centerLeft,
        splashFactory: !Platform.isAndroid ? NoSplash.splashFactory : null,
      ),
      child: Text(label),
    );
  }
}
