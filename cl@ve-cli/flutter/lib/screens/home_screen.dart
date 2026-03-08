import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Cl@ve AEAT'),
        centerTitle: true,
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // Status card
          Card(
            child: Padding(
              padding: const EdgeInsets.all(20),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.shield_outlined, color: theme.colorScheme.primary),
                      const SizedBox(width: 12),
                      Text('Session Status', style: theme.textTheme.titleMedium),
                    ],
                  ),
                  const SizedBox(height: 12),
                  // TODO: Replace with actual status from Rust bridge
                  Text(
                    'No active session',
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(height: 16),
                  FilledButton.icon(
                    onPressed: () => context.go('/activate'),
                    icon: const Icon(Icons.login),
                    label: const Text('Activate Device'),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),

          // Quick actions
          Text('Quick Actions', style: theme.textTheme.titleMedium),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              _ActionChip(
                icon: Icons.pin,
                label: 'Request PIN',
                onTap: () => context.go('/pin'),
              ),
              _ActionChip(
                icon: Icons.notifications,
                label: 'Pending',
                onTap: () => context.go('/pending'),
              ),
              _ActionChip(
                icon: Icons.qr_code,
                label: 'QR Auth',
                onTap: () {
                  // TODO: QR screen
                },
              ),
              _ActionChip(
                icon: Icons.person,
                label: 'My Data',
                onTap: () {
                  // TODO: My data screen
                },
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _ActionChip extends StatelessWidget {
  final IconData icon;
  final String label;
  final VoidCallback onTap;

  const _ActionChip({
    required this.icon,
    required this.label,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return ActionChip(
      avatar: Icon(icon, size: 18),
      label: Text(label),
      onPressed: onTap,
    );
  }
}
