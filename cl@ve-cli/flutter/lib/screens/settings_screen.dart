import 'package:flutter/material.dart';

class SettingsScreen extends StatelessWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

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
                        // TODO: Call deactivate() then logout()
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
            onTap: () {
              // TODO: Call logout()
              ScaffoldMessenger.of(context).showSnackBar(
                const SnackBar(content: Text('Session cleared')),
              );
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
