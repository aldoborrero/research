import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../src/auth_provider.dart';
import '../src/llave_bridge.dart';

/// Developer tools screen, only accessible in debug/profile builds.
class DevScreen extends ConsumerStatefulWidget {
  const DevScreen({super.key});

  /// Whether the dev screen should be shown (debug or profile mode).
  static bool get isEnabled => kDebugMode || kProfileMode;

  @override
  ConsumerState<DevScreen> createState() => _DevScreenState();
}

class _DevScreenState extends ConsumerState<DevScreen> {
  String? _lastResult;
  bool _loading = false;

  @override
  Widget build(BuildContext context) {
    final bridge = ref.watch(llaveBridgeProvider);
    final auth = ref.watch(authProvider);
    final session = ref.watch(currentSessionProvider);
    final theme = Theme.of(context);
    final isMock = bridge is MockLlaveBridge;

    return Scaffold(
      appBar: AppBar(title: const Text('Developer Tools')),
      body: ListView(
        padding: const EdgeInsets.symmetric(vertical: 8),
        children: [
          // Bridge info
          _SectionHeader('Bridge'),
          ListTile(
            leading: Icon(
              isMock ? Icons.science : Icons.link,
              color: isMock ? Colors.orange : Colors.green,
            ),
            title: Text(isMock ? 'MockLlaveBridge' : 'RealLlaveBridge'),
            subtitle: Text(isMock
                ? 'Using mock data. Run flutter_rust_bridge_codegen to use real Rust core.'
                : 'Connected to Rust core via FFI'),
          ),

          const Divider(),

          // Auth state
          _SectionHeader('Auth State'),
          ListTile(
            leading: Icon(_authIcon(auth), color: _authColor(auth)),
            title: Text(_authLabel(auth)),
            subtitle: session != null
                ? Text('NIF: ${session.nif}\n'
                    'Device: ${session.deviceId}\n'
                    'Created: ${session.createdAt}\n'
                    'Firebase: ${session.hasFirebaseToken}')
                : null,
            isThreeLine: session != null,
          ),

          const Divider(),

          // Quick actions
          _SectionHeader('Quick Actions'),
          _ActionTile(
            icon: Icons.person_add,
            title: 'Activate with test NIF',
            subtitle: 'Activate device using 12345678Z',
            loading: _loading,
            onTap: () => _runAction(() async {
              await ref
                  .read(authProvider.notifier)
                  .activate('12345678Z', null);
              return 'Activation complete';
            }),
          ),
          _ActionTile(
            icon: Icons.pin,
            title: 'Request PIN',
            subtitle: 'Request a new PIN from the bridge',
            loading: _loading,
            onTap: session == null
                ? null
                : () => _runAction(() async {
                      final result = await bridge.requestPin();
                      return 'PIN: ${result.pin}\n'
                          'TTL: ${result.timeToLiveSeconds}s\n'
                          'NIF: ${result.nif}';
                    }),
          ),
          _ActionTile(
            icon: Icons.info_outline,
            title: 'Get Status',
            subtitle: 'Read current bridge status',
            loading: _loading,
            onTap: () => _runAction(() async {
              final s = bridge.getStatus();
              return 'Active: ${s.active}\n'
                  'NIF: ${s.nif}\n'
                  'Device: ${s.deviceId}\n'
                  'Created: ${s.createdAt}\n'
                  'Firebase: ${s.hasFirebaseToken}';
            }),
          ),
          _ActionTile(
            icon: Icons.search,
            title: 'Check NIF',
            subtitle: 'Check if 12345678Z is registered',
            loading: _loading,
            onTap: () => _runAction(() async {
              final result = await bridge.checkNif('12345678Z');
              return 'NIF: ${result.nif}\n'
                  'Status: ${result.status}\n'
                  'Response: ${result.responseJson}';
            }),
          ),
          _ActionTile(
            icon: Icons.account_circle,
            title: 'Get My Data',
            subtitle: 'Fetch account data from bridge',
            loading: _loading,
            onTap: session == null
                ? null
                : () => _runAction(() async {
                      final result = await bridge.getMyData();
                      return 'OK: ${result.ok}\n'
                          'Data: ${result.data}\n'
                          'Error: ${result.error}';
                    }),
          ),
          _ActionTile(
            icon: Icons.history,
            title: 'Get History',
            subtitle: 'Fetch operations history',
            loading: _loading,
            onTap: session == null
                ? null
                : () => _runAction(() async {
                      final result = await bridge.getHistory();
                      return 'OK: ${result.ok}\nData: ${result.data}';
                    }),
          ),
          _ActionTile(
            icon: Icons.pending_actions,
            title: 'Get Pending Requests',
            subtitle: 'Poll for pending auth requests',
            loading: _loading,
            onTap: session == null
                ? null
                : () => _runAction(() async {
                      final result = await bridge.getPendingRequests();
                      return 'OK: ${result.ok}\nData: ${result.data}';
                    }),
          ),
          _ActionTile(
            icon: Icons.validate,
            title: 'Validate NIF',
            subtitle: 'Test NIF validation with 12345678Z',
            loading: _loading,
            onTap: () => _runAction(() async {
              try {
                final result = bridge.validateNif('12345678Z');
                return 'Valid: $result';
              } catch (e) {
                return 'Error: $e';
              }
            }),
          ),

          const Divider(),

          // Destructive actions
          _SectionHeader('Reset'),
          _ActionTile(
            icon: Icons.logout,
            title: 'Logout',
            subtitle: 'Clear local session',
            loading: _loading,
            color: theme.colorScheme.error,
            onTap: session == null
                ? null
                : () => _runAction(() async {
                      await ref.read(authProvider.notifier).logout();
                      return 'Logged out';
                    }),
          ),

          const Divider(),

          // Result output
          if (_lastResult != null) ...[
            _SectionHeader('Last Result'),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Card(
                color: theme.colorScheme.surfaceContainerHighest,
                child: Stack(
                  children: [
                    Padding(
                      padding: const EdgeInsets.all(12),
                      child: SelectableText(
                        _lastResult!,
                        style: const TextStyle(
                          fontFamily: 'monospace',
                          fontSize: 13,
                        ),
                      ),
                    ),
                    Positioned(
                      top: 4,
                      right: 4,
                      child: IconButton(
                        icon: const Icon(Icons.copy, size: 18),
                        tooltip: 'Copy',
                        onPressed: () {
                          Clipboard.setData(
                              ClipboardData(text: _lastResult!));
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(
                                content: Text('Copied to clipboard')),
                          );
                        },
                      ),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 16),
          ],
        ],
      ),
    );
  }

  Future<void> _runAction(Future<String> Function() action) async {
    setState(() {
      _loading = true;
      _lastResult = null;
    });
    try {
      final result = await action();
      if (mounted) setState(() => _lastResult = result);
    } catch (e) {
      if (mounted) setState(() => _lastResult = 'Error: $e');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  IconData _authIcon(AuthState auth) => switch (auth) {
        AuthLoading() => Icons.hourglass_empty,
        AuthUnauthenticated() => Icons.lock_outline,
        AuthAuthenticated() => Icons.lock_open,
        AuthError() => Icons.error_outline,
      };

  Color _authColor(AuthState auth) => switch (auth) {
        AuthLoading() => Colors.grey,
        AuthUnauthenticated() => Colors.orange,
        AuthAuthenticated() => Colors.green,
        AuthError() => Colors.red,
      };

  String _authLabel(AuthState auth) => switch (auth) {
        AuthLoading() => 'Loading...',
        AuthUnauthenticated() => 'Unauthenticated',
        AuthAuthenticated() => 'Authenticated',
        AuthError(message: final m) => 'Error: $m',
      };
}

class _SectionHeader extends StatelessWidget {
  final String title;
  const _SectionHeader(this.title);

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 4),
      child: Text(
        title,
        style: Theme.of(context).textTheme.labelLarge?.copyWith(
              color: Theme.of(context).colorScheme.primary,
            ),
      ),
    );
  }
}

class _ActionTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final bool loading;
  final VoidCallback? onTap;
  final Color? color;

  const _ActionTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.loading,
    this.onTap,
    this.color,
  });

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: Icon(icon, color: color),
      title: Text(title, style: color != null ? TextStyle(color: color) : null),
      subtitle: Text(subtitle),
      trailing: loading
          ? const SizedBox(
              width: 20,
              height: 20,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          : const Icon(Icons.play_arrow),
      enabled: !loading && onTap != null,
      onTap: onTap,
    );
  }
}
