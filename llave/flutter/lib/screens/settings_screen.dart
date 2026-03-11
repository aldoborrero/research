import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../src/auth_provider.dart';
import '../src/llave_bridge.dart';
import '../src/theme_provider.dart';

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final themeMode = ref.watch(themeModeProvider);

    return Scaffold(
      appBar: AppBar(title: const Text('Settings')),
      body: ListView(
        children: [
          const SizedBox(height: 8),
          _SectionHeader('Appearance'),
          ListTile(
            leading: Icon(_themeModeIcon(themeMode)),
            title: const Text('Theme'),
            subtitle: Text(_themeModeLabel(themeMode)),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => _showThemePicker(context, ref),
          ),
          const Divider(),
          _SectionHeader('Security'),
          ListTile(
            leading: const Icon(Icons.pin_outlined),
            title: const Text('Change PIN'),
            subtitle: const Text('Update your unlock PIN'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => _showChangePinDialog(context, ref),
          ),
          const Divider(),
          _SectionHeader('Device'),
          ListTile(
            leading: Icon(Icons.logout, color: theme.colorScheme.error),
            title: Text('Deactivate Device', style: TextStyle(color: theme.colorScheme.error)),
            subtitle: const Text('Remove this device from Llave'),
            onTap: () {
              showDialog(
                context: context,
                builder: (ctx) => AlertDialog(
                  title: const Text('Deactivate Device?'),
                  content: const Text('This will remove your device registration with Llave. You will need to activate again.'),
                  actions: [
                    TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
                    FilledButton(
                      onPressed: () {
                        Navigator.pop(ctx);
                        ref.read(authProvider.notifier).deactivate();
                      },
                      child: const Text('Deactivate'),
                    ),
                  ],
                ),
              );
            },
          ),
          ListTile(
            leading: const Icon(Icons.delete_outline),
            title: const Text('Clear Session'),
            subtitle: const Text('Log out and clear saved credentials'),
            onTap: () async {
              await ref.read(authProvider.notifier).logout();
              if (context.mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('Session cleared')),
                );
              }
            },
          ),
          const Divider(),
          _SectionHeader('About'),
          const ListTile(
            leading: Icon(Icons.info_outline),
            title: Text('Version'),
            subtitle: Text('0.1.0'),
          ),
          const ListTile(
            leading: Icon(Icons.code),
            title: Text('Powered by'),
            subtitle: Text('Rust core (llave-core) via flutter_rust_bridge'),
          ),
          if (kDebugMode || kProfileMode) ...[
            const Divider(),
            _SectionHeader('Developer'),
            ListTile(
              leading: Icon(
                Icons.science,
                color: ref.watch(llaveBridgeProvider) is MockLlaveBridge
                    ? Colors.orange
                    : Colors.green,
              ),
              title: const Text('Developer Tools'),
              subtitle: Text(
                ref.watch(llaveBridgeProvider) is MockLlaveBridge
                    ? 'Mock bridge active'
                    : 'Real bridge active',
              ),
              trailing: const Icon(Icons.chevron_right),
              onTap: () => context.go('/dev'),
            ),
          ],
        ],
      ),
    );
  }

  static IconData _themeModeIcon(ThemeMode mode) => switch (mode) {
        ThemeMode.system => Icons.brightness_auto,
        ThemeMode.light => Icons.light_mode,
        ThemeMode.dark => Icons.dark_mode,
      };

  static String _themeModeLabel(ThemeMode mode) => switch (mode) {
        ThemeMode.system => 'System default',
        ThemeMode.light => 'Light',
        ThemeMode.dark => 'Dark',
      };

  void _showChangePinDialog(BuildContext context, WidgetRef ref) {
    final oldPinCtrl = TextEditingController();
    final newPinCtrl = TextEditingController();
    final confirmCtrl = TextEditingController();

    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Change PIN'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: oldPinCtrl,
              obscureText: true,
              keyboardType: TextInputType.number,
              decoration: const InputDecoration(
                labelText: 'Current PIN',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: newPinCtrl,
              obscureText: true,
              keyboardType: TextInputType.number,
              decoration: const InputDecoration(
                labelText: 'New PIN',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: confirmCtrl,
              obscureText: true,
              keyboardType: TextInputType.number,
              decoration: const InputDecoration(
                labelText: 'Confirm New PIN',
                border: OutlineInputBorder(),
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () async {
              final oldPin = oldPinCtrl.text;
              final newPin = newPinCtrl.text;
              final confirm = confirmCtrl.text;

              if (newPin.length < 4) {
                if (ctx.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('PIN must be at least 4 characters')),
                  );
                }
                return;
              }
              if (newPin != confirm) {
                if (ctx.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('New PINs do not match')),
                  );
                }
                return;
              }

              Navigator.pop(ctx);
              try {
                await ref.read(authProvider.notifier).changePin(oldPin, newPin);
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('PIN changed successfully')),
                  );
                }
              } catch (e) {
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(content: Text('Failed to change PIN: $e')),
                  );
                }
              }
            },
            child: const Text('Change'),
          ),
        ],
      ),
    );
  }

  void _showThemePicker(BuildContext context, WidgetRef ref) {
    final notifier = ref.read(themeModeProvider.notifier);
    final current = ref.read(themeModeProvider);

    showDialog(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: const Text('Choose theme'),
        children: [
          for (final mode in ThemeMode.values)
            RadioListTile<ThemeMode>(
              title: Text(_themeModeLabel(mode)),
              secondary: Icon(_themeModeIcon(mode)),
              value: mode,
              groupValue: current,
              onChanged: (v) {
                if (v != null) notifier.setMode(v);
                Navigator.pop(ctx);
              },
            ),
        ],
      ),
    );
  }
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
