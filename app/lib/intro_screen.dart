// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/widgets/widgets.dart';

import 'core/core.dart';
import 'navigation/navigation.dart';

class IntroScreen extends StatelessWidget {
  const IntroScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final isUserLoading =
        context.select((LoadableUserCubit cubit) => cubit.state is LoadingUser);

    return Scaffold(
      body: Center(
        child: Container(
          height: MediaQuery.of(context).size.height,
          padding: const EdgeInsets.fromLTRB(20, 100, 20, 50),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.center,
            mainAxisSize: MainAxisSize.max,
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Image(
                image: const AssetImage('assets/images/logo.png'),
                height: 100,
                filterQuality: FilterQuality.high,
                color: Colors.grey[350],
              ),
              const _GradientText(
                "Prototype.",
                gradient: LinearGradient(
                  colors: [
                    Color.fromARGB(255, 34, 163, 255),
                    Color.fromARGB(255, 72, 23, 250)
                  ],
                  transform: GradientRotation(1.1),
                ),
                style: TextStyle(
                  fontSize: 36,
                  fontVariations: variationMedium,
                  letterSpacing: -0.9,
                ),
              ),
              const _ClientRecords(),
              // Text button that opens the developer settings screen
              TextButton(
                onPressed: () =>
                    context.read<NavigationCubit>().openDeveloperSettings(),
                style: textButtonStyle(context),
                child: const Text('Developer Settings'),
              ),
              if (!isUserLoading)
                Column(
                  crossAxisAlignment: isSmallScreen(context)
                      ? CrossAxisAlignment.stretch
                      : CrossAxisAlignment.center,
                  children: [
                    OutlinedButton(
                      onPressed: () =>
                          context.read<NavigationCubit>().openServerChoice(),
                      style: buttonStyle(context, true),
                      child: const Text('Sign up'),
                    )
                  ],
                )
            ],
          ),
        ),
      ),
    );
  }
}

class _ClientRecords extends StatefulWidget {
  const _ClientRecords();

  @override
  State<_ClientRecords> createState() => _ClientRecordsState();
}

class _ClientRecordsState extends State<_ClientRecords> {
  Future<List<UiClientRecord>>? _clientRecords;

  @override
  void initState() {
    super.initState();
    loadClientRecords();
  }

  void loadClientRecords() async {
    final clientRecords = User.loadClientRecords(dbPath: await dbPath());
    setState(() {
      _clientRecords = clientRecords;
    });
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<List<UiClientRecord>>(
      future: _clientRecords,
      builder: (context, snapshot) {
        if (snapshot.hasData) {
          return _ClientRecordsList(snapshot.data!);
        } else if (snapshot.hasError) {
          return Text(
            'Error loading contacts',
          );
        }
        return const CircularProgressIndicator();
      },
    );
  }
}

class _ClientRecordsList extends StatelessWidget {
  const _ClientRecordsList(this.clientRecords);

  final List<UiClientRecord> clientRecords;

  @override
  Widget build(BuildContext context) {
    const itemExtent = 72.0;

    return Center(
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxHeight: 3.5 * itemExtent, // 3 items without scrolling
          maxWidth: MediaQuery.of(context).size.width.clamp(0, 400),
        ),
        child: ListView(
          itemExtent: itemExtent,
          children: clientRecords
              .map(
                (record) => ListTile(
                  leading: UserAvatar(
                    username: record.userName.userName,
                    image: record.userProfile?.profilePicture,
                    size: Spacings.xl,
                  ),
                  title: Text(
                    record.userName
                        .displayName(record.userProfile?.displayName),
                  ),
                  subtitle: Text(
                    "@${record.userName.domain}",
                  ),
                  onTap: () => context.read<CoreClient>().loadUser(
                      userName: record.userName, clientId: record.clientId),
                ),
              )
              .toList(),
        ),
      ),
    );
  }
}

class _GradientText extends StatelessWidget {
  const _GradientText(
    this.text, {
    required this.gradient,
    this.style,
  });

  final String text;
  final TextStyle? style;
  final Gradient gradient;

  @override
  Widget build(BuildContext context) {
    return ShaderMask(
      blendMode: BlendMode.srcIn,
      shaderCallback: (bounds) => gradient.createShader(
        Rect.fromLTWH(0, 0, bounds.width, bounds.height),
      ),
      child: Text(text, style: style),
    );
  }
}
