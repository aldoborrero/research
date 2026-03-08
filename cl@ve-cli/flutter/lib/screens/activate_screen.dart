import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../src/auth_provider.dart';

class ActivateScreen extends ConsumerStatefulWidget {
  const ActivateScreen({super.key});

  @override
  ConsumerState<ActivateScreen> createState() => _ActivateScreenState();
}

class _ActivateScreenState extends ConsumerState<ActivateScreen> {
  final _formKey = GlobalKey<FormState>();
  final _nifController = TextEditingController();
  final _passwordController = TextEditingController();
  bool _obscurePassword = true;
  bool _loading = false;
  String? _error;

  @override
  void dispose() {
    _nifController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  Future<void> _activate() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() {
      _loading = true;
      _error = null;
    });

    final nif = _nifController.text.trim();
    final password = _passwordController.text.trim();

    await ref.read(authProvider.notifier).activate(
          nif,
          password.isEmpty ? null : password,
        );

    if (!mounted) return;

    final auth = ref.read(authProvider);
    if (auth is AuthAuthenticated) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Device activated for ${auth.session.nif}'),
          backgroundColor: Theme.of(context).colorScheme.primary,
        ),
      );
      context.go('/');
    } else if (auth is AuthError) {
      setState(() {
        _loading = false;
        _error = auth.message;
      });
    } else {
      setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: Form(
            key: _formKey,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const SizedBox(height: 48),
                // Header
                Icon(
                  Icons.shield,
                  size: 64,
                  color: theme.colorScheme.primary,
                ),
                const SizedBox(height: 16),
                Text(
                  'Activate Cl@ve',
                  style: theme.textTheme.headlineMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 8),
                Text(
                  'Register this device with your Cl@ve account to access AEAT services.',
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 40),

                // NIF field
                TextFormField(
                  controller: _nifController,
                  decoration: const InputDecoration(
                    labelText: 'NIF (DNI/NIE)',
                    hintText: '12345678A',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.badge),
                  ),
                  textCapitalization: TextCapitalization.characters,
                  enabled: !_loading,
                  validator: (v) {
                    if (v == null || v.trim().isEmpty) return 'NIF is required';
                    final trimmed = v.trim();
                    if (trimmed.length < 8) {
                      return 'NIF must be at least 8 characters';
                    }
                    if (!RegExp(r'^[0-9XYZ]').hasMatch(trimmed.toUpperCase())) {
                      return 'NIF must start with 0-9, X, Y, or Z';
                    }
                    return null;
                  },
                ),
                const SizedBox(height: 16),

                // Password field
                TextFormField(
                  controller: _passwordController,
                  decoration: InputDecoration(
                    labelText: 'Password (optional)',
                    hintText: 'Auto-generated if empty',
                    border: const OutlineInputBorder(),
                    prefixIcon: const Icon(Icons.lock_outline),
                    suffixIcon: IconButton(
                      icon: Icon(_obscurePassword
                          ? Icons.visibility_off
                          : Icons.visibility),
                      onPressed: () =>
                          setState(() => _obscurePassword = !_obscurePassword),
                    ),
                  ),
                  obscureText: _obscurePassword,
                  enabled: !_loading,
                ),
                const SizedBox(height: 8),
                Text(
                  'If left empty, a secure password will be generated automatically.',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),

                // Error message
                if (_error != null) ...[
                  const SizedBox(height: 16),
                  Card(
                    color: theme.colorScheme.errorContainer,
                    child: Padding(
                      padding: const EdgeInsets.all(12),
                      child: Row(
                        children: [
                          Icon(Icons.error_outline,
                              color: theme.colorScheme.error, size: 20),
                          const SizedBox(width: 8),
                          Expanded(
                            child: Text(
                              _error!,
                              style: TextStyle(
                                  color: theme.colorScheme.onErrorContainer),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ],

                const SizedBox(height: 24),

                // Activate button
                FilledButton.icon(
                  onPressed: _loading ? null : _activate,
                  icon: _loading
                      ? const SizedBox(
                          width: 18,
                          height: 18,
                          child: CircularProgressIndicator(
                              strokeWidth: 2, color: Colors.white),
                        )
                      : const Icon(Icons.shield),
                  label: Text(_loading ? 'Activating...' : 'Activate Device'),
                  style: FilledButton.styleFrom(
                    padding: const EdgeInsets.symmetric(vertical: 16),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
