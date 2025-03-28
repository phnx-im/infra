// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/util/cached_memory_image.dart';

class UserAvatar extends StatelessWidget {
  const UserAvatar({
    super.key,
    required this.username,
    this.size = 24.0,
    this.image,
    this.onPressed,
  });

  final String username;
  final double size;
  final ImageData? image;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onPressed,
      child: MouseRegion(
        cursor:
            onPressed != null
                ? SystemMouseCursors.click
                : SystemMouseCursors.basic,
        child: SizedBox(
          width: size,
          height: size,
          child: CircleAvatar(
            radius: size / 2,
            backgroundColor: colorDMBLight,
            foregroundImage:
                image != null ? CachedMemoryImage.fromImageData(image!) : null,
            child: Text(
              username.characters.firstOrNull?.toUpperCase() ?? "",
              style: TextStyle(
                color: Colors.white,
                fontSize: 10 * size / 24,
              ).merge(VariableFontWeight.bold),
            ),
          ),
        ),
      ),
    );
  }
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
      builder:
          (context, snapshot) => UserAvatar(
            username: snapshot.data?.userName ?? " ",
            image: snapshot.data?.profilePicture,
            size: widget.size,
            onPressed: widget.onPressed,
          ),
    );
  }
}
