import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logging/logging.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'llave_bridge.dart';

final _log = Logger('ProxyProvider');
const _prefKey = 'proxy_url';

/// Holds the current proxy URL (`null` = direct connection).
final proxyProvider =
    StateNotifierProvider<ProxyNotifier, String?>((ref) {
  return ProxyNotifier(ref);
});

class ProxyNotifier extends StateNotifier<String?> {
  final Ref _ref;

  ProxyNotifier(this._ref) : super(null) {
    _load();
  }

  Future<void> _load() async {
    final prefs = await SharedPreferences.getInstance();
    final stored = prefs.getString(_prefKey);
    if (stored != null && stored.isNotEmpty) {
      _log.info('loaded proxy: $stored');
      state = stored;
      _apply(stored);
    }
  }

  /// Set a new proxy URL, persist it, and apply it to the Rust core.
  /// Pass `null` or empty string to clear.
  Future<void> setProxy(String? url) async {
    final trimmed = url?.trim();
    final effective = (trimmed != null && trimmed.isNotEmpty) ? trimmed : null;

    state = effective;
    _apply(effective);

    final prefs = await SharedPreferences.getInstance();
    if (effective != null) {
      await prefs.setString(_prefKey, effective);
      _log.info('proxy set: $effective');
    } else {
      await prefs.remove(_prefKey);
      _log.info('proxy cleared');
    }
  }

  void _apply(String? url) {
    try {
      _ref.read(llaveBridgeProvider).setProxy(url);
    } catch (e) {
      _log.warning('failed to apply proxy: $e');
    }
  }
}
