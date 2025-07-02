// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/theme/theme.dart';

/// Left panel which can be resized by dragging the handle
class ResizablePanel extends StatefulWidget {
  const ResizablePanel({
    required this.initialWidth,
    this.minWidth = 180,
    this.maxWidth = 500,
    this.resizeHandleWidth = 10,
    this.panelColor = convPaneBackgroundColor,
    this.backgroundColor = Colors.white,
    this.separatorColor = colorGreyLight,
    this.onResizeEnd,
    required this.child,
    super.key,
  });

  final double initialWidth;
  final double minWidth;
  final double maxWidth;
  final double resizeHandleWidth;

  final Color panelColor;
  final Color backgroundColor;
  final Color separatorColor;

  final Widget child;

  final Function(double)? onResizeEnd;

  @override
  State<ResizablePanel> createState() => _ResizablePanelState();
}

class _ResizablePanelState extends State<ResizablePanel> {
  late double _panelWidth;

  @override
  void initState() {
    super.initState();
    setState(() {
      _panelWidth = widget.initialWidth;
    });
  }

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: _panelWidth,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          SizedBox(
            width: _panelWidth - widget.resizeHandleWidth,
            child: widget.child,
          ),

          // Resizable Handle
          MouseRegion(
            cursor: SystemMouseCursors.resizeColumn,
            child: GestureDetector(
              onHorizontalDragUpdate: onResize,
              onHorizontalDragEnd: (details) {
                if (widget.onResizeEnd case final onResizeEnd?) {
                  onResizeEnd(_panelWidth);
                }
              },
              child: SizedBox(
                width: widget.resizeHandleWidth,
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: widget.resizeHandleWidth / 2,
                      color: widget.panelColor,
                      alignment: Alignment.topRight,
                    ),
                    Container(
                      width: widget.resizeHandleWidth / 2,
                      decoration: BoxDecoration(
                        color: widget.backgroundColor,
                        shape: BoxShape.rectangle,
                        border: Border(
                          left: BorderSide(
                            width: 1,
                            color: widget.separatorColor,
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  void onResize(DragUpdateDetails details) {
    setState(() {
      _panelWidth = (_panelWidth + details.delta.dx).clamp(
        widget.minWidth,
        widget.maxWidth,
      );
    });
  }
}
