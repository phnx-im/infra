// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/widgets/widgets.dart';

class LogsScreen extends StatefulWidget {
  const LogsScreen({super.key});

  @override
  State<LogsScreen> createState() => _LogsScreenState();
}

class _LogsScreenState extends State<LogsScreen> {
  late Future<String> _appLogs;
  late Future<String> _backgroundLogs;

  @override
  void initState() {
    super.initState();
    _loadLogs();
  }

  void _loadLogs() async {
    final appLogs = readAppLogs();
    final backgroundLogs = getApplicationCacheDirectory()
        .then((cacheDir) => readBackgroundLogs(cacheDir: cacheDir.path));

    setState(() {
      _appLogs = appLogs;
      _backgroundLogs = backgroundLogs;
    });
  }

  void _clearLogs() async {
    await clearAppLogs();
    final cacheDir = await getApplicationCacheDirectory();
    await clearBackgroundLogs(cacheDir: cacheDir.path);
    setState(() {
      _appLogs = Future.value("");
      _backgroundLogs = Future.value("");
    });
  }

  @override
  Widget build(BuildContext context) {
    return LogsScreenView(
      appLogs: _appLogs,
      backgroundLogs: _backgroundLogs,
      reloadLogs: _loadLogs,
      clearLogs: _clearLogs,
    );
  }
}

class LogsScreenView extends StatelessWidget {
  const LogsScreenView({
    required this.appLogs,
    required this.backgroundLogs,
    required this.reloadLogs,
    required this.clearLogs,
    super.key,
  });

  final Future<String> appLogs;
  final Future<String> backgroundLogs;
  final VoidCallback reloadLogs;
  final VoidCallback clearLogs;

  @override
  Widget build(BuildContext context) {
    return DefaultTabController(
      length: 2,
      child: Scaffold(
        appBar: AppBar(
          title: const Text('Logs'),
          toolbarHeight: isPointer() ? 100 : null,
          leading: const AppBarBackButton(),
          actions: [
            PopupMenuButton(
              itemBuilder: (context) => [
                PopupMenuItem(
                  onTap: reloadLogs,
                  child: const Text('Reload Logs'),
                ),
                PopupMenuItem(
                  onTap: clearLogs,
                  child: const Text('Clear Logs'),
                ),
              ],
            ),
          ],
        ),
        bottomNavigationBar: const SafeArea(
          left: false,
          right: false,
          top: false,
          bottom: true,
          child: TabBar(
            tabs: [
              Tab(text: 'App'),
              Tab(text: 'Background'),
            ],
          ),
        ),
        body: SafeArea(
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: Spacings.xxs),
            child: TabBarView(
              children: [
                _LogsView(logs: appLogs),
                _LogsView(logs: backgroundLogs),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _LogsView extends StatefulWidget {
  const _LogsView({required this.logs});

  final Future<String>? logs;

  @override
  State<_LogsView> createState() => _LogsViewState();
}

class _LogsViewState extends State<_LogsView>
    with AutomaticKeepAliveClientMixin {
  @override
  Widget build(BuildContext context) {
    super.build(context);
    return FutureBuilder(
      future: widget.logs,
      builder: (context, snapshot) {
        if (snapshot.hasData) {
          final data = snapshot.data!;
          return SelectableText(data);
        } else if (snapshot.hasError) {
          return const Center(child: Text('Error loading logs'));
        }
        return const Center(
          child: SizedBox(
            child: CircularProgressIndicator(),
          ),
        );
      },
    );
  }

  @override
  bool get wantKeepAlive => true;
}
