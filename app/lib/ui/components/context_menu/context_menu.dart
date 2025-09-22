// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:air/theme/spacings.dart';
import 'package:air/ui/components/context_menu/context_menu_item_ui.dart';
import 'package:air/ui/components/context_menu/context_menu_ui.dart';

enum ContextMenuDirection { left, right }

class ContextMenu extends StatefulWidget {
  const ContextMenu({
    super.key,
    required this.direction,
    this.offset = Offset.zero,
    required this.width,
    required this.controller,
    required this.menuItems,
    this.child,
  });

  final ContextMenuDirection direction;
  final Offset offset;
  final double width;
  final OverlayPortalController controller;
  final List<ContextMenuItem> menuItems;
  final Widget? child;

  @override
  State<ContextMenu> createState() => _ContextMenuState();
}

class _ContextMenuState extends State<ContextMenu> {
  final GlobalKey _childKey = GlobalKey();
  Offset? _childPosition;
  Size? _childSize;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.scheduleFrameCallback(_checkChildPosition);
  }

  void _checkChildPosition(Duration timeStamp) {
    if (!mounted) return;

    final context = _childKey.currentContext;
    final box = context?.findRenderObject() as RenderBox?;

    if (box != null && box.hasSize) {
      final newSize = box.size;
      final newPosition = box.localToGlobal(Offset.zero);

      if (newSize != _childSize || newPosition != _childPosition) {
        setState(() {
          _childSize = newSize;
          _childPosition = newPosition;
        });
      }
    }
  }

  Offset _relativePosition() {
    final (position, size) = (_childPosition, _childSize);
    if (position == null || size == null) {
      return Offset.zero;
    }

    switch (widget.direction) {
      case ContextMenuDirection.left:
        return Offset(
          position.dx - widget.width + size.width - widget.offset.dx,
          position.dy + size.height + widget.offset.dy + Spacings.xs,
        );
      case ContextMenuDirection.right:
        return Offset(
          position.dx + widget.offset.dx,
          position.dy + size.height + widget.offset.dy + Spacings.xxs,
        );
    }
  }

  @override
  Widget build(BuildContext context) {
    final relativePosition = _relativePosition();

    // Add hide to menu items and store it menu items

    final updatedMenuItems = <ContextMenuItem>[];
    for (final item in widget.menuItems) {
      updatedMenuItems.add(
        item.copyWith(
          onPressed: () {
            widget.controller.hide();
            item.onPressed();
          },
        ),
      );
    }

    return OverlayPortal(
      controller: widget.controller,
      child: KeyedSubtree(
        key: _childKey,
        child: widget.child ?? const SizedBox.shrink(),
      ),

      overlayChildBuilder: (BuildContext context) {
        return Focus(
          autofocus: true,
          onKeyEvent: (node, event) {
            if (event.logicalKey == LogicalKeyboardKey.escape &&
                event is KeyDownEvent) {
              widget.controller.hide();
              return KeyEventResult.handled;
            }
            return KeyEventResult.ignored;
          },
          child: Stack(
            children: [
              Positioned.fill(
                child: GestureDetector(
                  behavior: HitTestBehavior.translucent,
                  onTap: () => widget.controller.hide(),
                ),
              ),
              Positioned(
                left: relativePosition.dx,
                top: relativePosition.dy,
                child: SizedBox(
                  width: widget.width,
                  child: ContextMenuUi(
                    menuItems: updatedMenuItems,
                    onHide: widget.controller.hide,
                  ),
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}
