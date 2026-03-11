import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../src/auth_provider.dart';
import '../src/rust/api/api.dart' as native_ffi;

class HomeScreen extends ConsumerWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(currentSessionProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Llave'),
        centerTitle: true,
        actions: [
          if (session != null)
            IconButton(
              icon: const Icon(Icons.logout),
              tooltip: 'Log out',
              onPressed: () => _confirmLogout(context, ref),
            ),
        ],
      ),
      body: session == null
          ? _UnauthenticatedBody()
          : _AuthenticatedBody(session: session),
    );
  }

  void _confirmLogout(BuildContext context, WidgetRef ref) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Log out?'),
        content:
            const Text('This will clear your session. You can re-activate later.'),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
          FilledButton(
            onPressed: () {
              Navigator.pop(ctx);
              ref.read(authProvider.notifier).logout();
            },
            child: const Text('Log out'),
          ),
        ],
      ),
    );
  }
}

class _UnauthenticatedBody extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.shield_outlined, size: 64,
                color: theme.colorScheme.onSurfaceVariant),
            const SizedBox(height: 16),
            Text(
              'No active session',
              style: theme.textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            Text(
              'Activate your device to get started.',
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 24),
            FilledButton.icon(
              onPressed: () => GoRouter.of(context).go('/activate'),
              icon: const Icon(Icons.login),
              label: const Text('Activate Device'),
            ),
          ],
        ),
      ),
    );
  }
}

class _AuthenticatedBody extends StatefulWidget {
  final dynamic session;
  const _AuthenticatedBody({required this.session});

  @override
  State<_AuthenticatedBody> createState() => _AuthenticatedBodyState();
}

class _AuthenticatedBodyState extends State<_AuthenticatedBody> {
  bool _loading = false;
  Map<String, dynamic>? _accountData;
  String? _error;

  @override
  void initState() {
    super.initState();
    _loadAccountData();
  }

  Future<void> _loadAccountData() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final result = await native_ffi.getMyData();
      if (!result.ok) {
        setState(() => _error = result.error ?? 'Could not load account data');
        return;
      }
      final parsed = jsonDecode(result.data);
      setState(() =>
          _accountData = parsed is Map<String, dynamic> ? parsed : null);
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // Account card
        _AccountCard(
          session: widget.session,
          accountData: _accountData,
          loading: _loading,
          error: _error,
          onRetry: _loadAccountData,
        ),
        const SizedBox(height: 20),

        // QR Auth — primary CTA
        SizedBox(
          height: 56,
          child: FilledButton.icon(
            onPressed: () => context.go('/qr'),
            icon: const Icon(Icons.qr_code_2, size: 24),
            label: Text(
              'QR Authentication',
              style: theme.textTheme.titleMedium?.copyWith(
                color: theme.colorScheme.onPrimary,
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class _AccountCard extends StatelessWidget {
  final dynamic session;
  final Map<String, dynamic>? accountData;
  final bool loading;
  final String? error;
  final VoidCallback onRetry;

  const _AccountCard({
    required this.session,
    required this.accountData,
    required this.loading,
    required this.error,
    required this.onRetry,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.person, color: theme.colorScheme.primary),
                const SizedBox(width: 12),
                Text('Account', style: theme.textTheme.titleMedium),
                const Spacer(),
                if (loading)
                  const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                else
                  InkWell(
                    borderRadius: BorderRadius.circular(16),
                    onTap: onRetry,
                    child: Icon(
                      Icons.refresh,
                      size: 20,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
              ],
            ),
            const SizedBox(height: 16),
            // Always show NIF from session
            _AccountRow(label: 'NIF', value: session.nif),
            // Show account data if loaded
            if (accountData != null) ...[
              if (accountData!['email'] != null)
                _AccountRow(label: 'Email', value: accountData!['email'].toString()),
              if (accountData!['numTelefono'] != null)
                _AccountRow(label: 'Phone', value: accountData!['numTelefono'].toString()),
              if (accountData!['nivelRegistro'] != null)
                _AccountRow(
                    label: 'Level',
                    value: _registrationLevel(accountData!['nivelRegistro'].toString())),
            ],
            if (error != null && accountData == null)
              Padding(
                padding: const EdgeInsets.only(top: 4),
                child: Text(
                  error!,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.error,
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }

  static String _registrationLevel(String level) => switch (level) {
        '1' => 'Basic',
        '2' => 'Advanced',
        '3' => 'Superior',
        _ => level,
      };
}

class _AccountRow extends StatelessWidget {
  final String label;
  final String value;
  const _AccountRow({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 64,
            child: Text(
              label,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(
            child: Text(value, style: theme.textTheme.bodyMedium),
          ),
        ],
      ),
    );
  }
}
