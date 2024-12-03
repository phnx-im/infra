import 'package:flutter/widgets.dart';
import 'package:provider/provider.dart';
import 'package:provider/single_child_widget.dart';

// Based on <https://github.com/felangel/bloc/blob/ba21a05dfb3247acb33b269394e81d57ee45d1db/packages/bloc/lib/src/bloc_base.dart>
// MIT License

/// A source of streamable state which can be watched from a build context.
abstract class StateStreamableSource<T> {
  const StateStreamableSource();

  bool get isClosed;

  void close();

  T get state;

  Stream<T> get stream;
}

/// A provider of state via a [StateStreamableSource]
///
/// Can be accesseed via [context.watch], [context.select], or [context.read] from a child's build context.
class StateProvider<T extends StateStreamableSource>
    extends SingleChildStatelessWidget {
  const StateProvider({
    super.key,
    required Create<T> create,
    super.child,
    this.lazy = true,
  })  : _create = create,
        super();

  final bool lazy;

  final Create<T> _create;

  @override
  Widget buildWithChild(BuildContext context, Widget? child) {
    return InheritedProvider<T>(
      create: _create,
      dispose: (context, value) => value.close(),
      startListening: _startListening,
      lazy: lazy,
      child: child,
    );
  }

  static VoidCallback _startListening(
    InheritedContext<StateStreamableSource<dynamic>?> element,
    StateStreamableSource<dynamic> value,
  ) {
    final subscription =
        value.stream.listen((dynamic _) => element.markNeedsNotifyDependents());
    return subscription.cancel;
  }
}
