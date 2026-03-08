import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../src/auth_provider.dart';
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
          _SectionHeader('Account'),
          ListTile(
            leading: const Icon(Icons.person_outline),
            title: const Text('My Data'),
            subtitle: const Text('View Cl@ve account information'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () {
              // TODO: Navigate to my data or call getMyData()
            },
          ),
          ListTile(
            leading: const Icon(Icons.qr_code),
            title: const Text('QR Authentication'),
            subtitle: const Text('Scan a QR code to authenticate'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () {
              // TODO: Navigate to QR screen
            },
          ),
          const Divider(),
          _SectionHeader('Appearance'),
          ListTile(
            leading: Icon(_themeModeIcon(themeMode)),
            title: const Text('Theme'),
            subtitle: Text(_themeModeLabel(themeMode)),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => _showThemePicker(context, ref),
          ),
          const Divider(),
          _SectionHeader('Device'),
          ListTile(
            leading: Icon(Icons.logout, color: theme.colorScheme.error),
            title: Text('Deactivate Device', style: TextStyle(color: theme.colorScheme.error)),
            subtitle: const Text('Remove this device from Cl@ve'),
            onTap: () {
              showDialog(
                context: context,
                builder: (ctx) => AlertDialog(
                  title: const Text('Deactivate Device?'),
                  content: const Text('This will remove your device registration with Cl@ve. You will need to activate again.'),
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
            subtitle: Text('Rust core (clave-core) via flutter_rust_bridge'),
          ),
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
