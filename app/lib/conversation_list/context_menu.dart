// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:prototype/theme/spacings.dart';
import 'package:prototype/theme/styles.dart';

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

  //final ContextMenuCorner corner;
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
    WidgetsBinding.instance.addPersistentFrameCallback(_checkChildPosition);
  }

  void _checkChildPosition(Duration timeStamp) {
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

    WidgetsBinding.instance.scheduleFrameCallback(_checkChildPosition);
  }

  Offset _relativePosition() {
    final (position, size) = (_childPosition, _childSize);
    if (position == null || size == null) {
      return Offset.zero;
    }

    switch (widget.direction) {
      case ContextMenuDirection.left:
        return Offset(
          position.dx - widget.width - widget.offset.dx,
          position.dy + size.height + widget.offset.dy,
        );
      case ContextMenuDirection.right:
        return Offset(
          position.dx + size.width + widget.offset.dx,
          position.dy + size.height + widget.offset.dy,
        );
    }
  }

  @override
  Widget build(BuildContext context) {
    final relativePosition = _relativePosition();

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
                        for (final (i, item) in widget.menuItems.indexed) ...[
                          ContextMenuItem(
                            onPressed: () {
                              item.onPressed();
                              widget.controller.hide();
                            },
                            label: item.label,
                          ),
                          if (i < widget.menuItems.length - 1)
                            const Divider(
                              height: 0,
                              thickness: 1,
                              color: colorGreyLight,
                            ),
                        ],
                      ],
                    ),
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

class ContextMenuItem extends StatelessWidget {
  const ContextMenuItem({
    super.key,
    required this.onPressed,
    required this.label,
  });

  final VoidCallback onPressed;
  final String label;

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
