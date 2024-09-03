// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/styles.dart';

IconButton appBarBackButton(BuildContext context) {
  return IconButton(
    icon: const Icon(Icons.arrow_back),
    color: Colors.black,
    hoverColor: Colors.transparent,
    splashColor: Colors.transparent,
    highlightColor: Colors.transparent,
    onPressed: () => Navigator.of(context).pop(),
  );
}

class FutureUserAvatar extends StatefulWidget {
  final Future<UiUserProfile?> profile;
  final VoidCallback? onPressed;
  final double size;

  const FutureUserAvatar({
    super.key,
    required this.profile,
    this.onPressed,
    this.size = 24.0,
  });

  @override
  State<FutureUserAvatar> createState() => _FutureUserAvatarState();
}

class _FutureUserAvatarState extends State<FutureUserAvatar> {
  @override
  Widget build(BuildContext context) {
    return FutureBuilder<UiUserProfile?>(
        future: widget.profile,
        builder: (context, snapshot) {
          if (snapshot.hasData) {
            return UserAvatar(
                username: snapshot.data!.userName,
                size: widget.size,
                image: snapshot.data!.profilePictureOption,
                onPressed: widget.onPressed);
          } else {
            return UserAvatar(
                username: " ",
                size: widget.size,
                image: null,
                onPressed: widget.onPressed);
          }
        });
  }
}

class UserAvatar extends StatefulWidget {
  final String username;
  final double size;
  final Uint8List? image;
  final VoidCallback? onPressed;

  const UserAvatar({
    super.key,
    required this.username,
    this.size = 24.0,
    this.image,
    this.onPressed,
  });

  @override
  State<UserAvatar> createState() => _UserAvatarState();
}

class _UserAvatarState extends State<UserAvatar> {
  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: widget.onPressed,
      child: MouseRegion(
        cursor: widget.onPressed != null
            ? SystemMouseCursors.click
            : SystemMouseCursors.basic,
        child: SizedBox(
          width: widget.size,
          height: widget.size,
          child: CircleAvatar(
            radius: widget.size / 2,
            backgroundColor: colorDMBLight,
            foregroundImage:
                (widget.image != null) ? MemoryImage(widget.image!) : null,
            child: Text(
              (widget.username.characters.firstOrNull ?? "").toUpperCase(),
              style: TextStyle(
                color: Colors.white,
                fontSize: 10 * widget.size / 24,
                fontWeight: FontWeight.bold,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class FrostedGlass extends StatelessWidget {
  final Color color;
  final double height;

  const FrostedGlass({
    super.key,
    required this.color,
    required this.height,
  });

  @override
  Widget build(BuildContext context) {
    return ClipRect(
      child: BackdropFilter(
        filter: ImageFilter.blur(
            sigmaX: 15, sigmaY: 15, tileMode: TileMode.repeated),
        child: Container(
          width: MediaQuery.of(context).size.width,
          height: height,
          color: color.withOpacity(0.4),
        ),
      ),
    );
  }
}
