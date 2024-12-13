// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:ui';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:prototype/painting/painting.dart';
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
  final AsyncValueGetter<UiUserProfile?> profile;
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
  late final Future<UiUserProfile?> _profileFuture;

  @override
  void initState() {
    _profileFuture = widget.profile();
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<UiUserProfile?>(
      future: _profileFuture,
      builder: (context, snapshot) => UserAvatar(
        username: snapshot.data?.userName ?? " ",
        image: snapshot.data?.profilePictureOption,
        size: widget.size,
        onPressed: widget.onPressed,
      ),
    );
  }
}

class UserAvatar extends StatelessWidget {
  final String username;
  final double size;
  final Uint8List? image;
  final VoidCallback? onPressed;
  final String? cacheTag;

  const UserAvatar({
    super.key,
    required this.username,
    this.size = 24.0,
    this.image,
    this.onPressed,
    this.cacheTag,
  });

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onPressed,
      child: MouseRegion(
        cursor: onPressed != null
            ? SystemMouseCursors.click
            : SystemMouseCursors.basic,
        child: SizedBox(
          width: size,
          height: size,
          child: CircleAvatar(
            radius: size / 2,
            backgroundColor: colorDMBLight,
            foregroundImage: (image != null)
                ? CachedMemoryImage(cacheTag ?? "avatar:$username", image!)
                : null,
            child: Text(
              username.characters.firstOrNull?.toUpperCase() ?? "",
              style: TextStyle(
                color: Colors.white,
                fontSize: 10 * size / 24,
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
