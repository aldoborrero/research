import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../src/auth_provider.dart';
import '../src/biometric_service.dart';

/// PIN entry screen shown when an encrypted session exists on disk.
class UnlockScreen extends ConsumerStatefulWidget {
  const UnlockScreen({super.key});

  @override
  ConsumerState<UnlockScreen> createState() => _UnlockScreenState();
}

class _UnlockScreenState extends ConsumerState<UnlockScreen> {
  final _pinController = TextEditingController();
  String? _error;
  bool _loading = false;
  bool _obscure = true;
  bool _biometricAvailable = false;

  @override
  void initState() {
    super.initState();
    _initBiometrics();
  }

  @override
  void dispose() {
    _pinController.dispose();
    super.dispose();
  }

  Future<void> _initBiometrics() async {
    final available = await BiometricService.isAvailable();
    final enabled = available && await BiometricService.isEnabled();
    if (mounted) {
      setState(() => _biometricAvailable = enabled);
      if (enabled) {
        _unlockWithBiometrics();
      }
    }
  }

  Future<void> _unlockWithBiometrics() async {
    setState(() {
      _error = null;
      _loading = true;
    });

    try {
      final pin = await BiometricService.authenticate();
      if (pin == null) {
        if (mounted) setState(() => _loading = false);
        return;
      }
      await ref.read(authProvider.notifier).unlock(pin);
      if (mounted) context.go('/');
    } catch (_) {
      if (mounted) {
        setState(() {
          _error = 'Biometric unlock failed. Enter your PIN.';
          _loading = false;
        });
      }
    }
  }

  Future<void> _unlock() async {
    final pin = _pinController.text;
    if (pin.isEmpty) return;

    setState(() {
      _error = null;
      _loading = true;
    });

    try {
      await ref.read(authProvider.notifier).unlock(pin);
      if (mounted) context.go('/');
    } catch (_) {
      setState(() => _error = 'Wrong PIN. Try again.');
      _pinController.clear();
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _forgotPin() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Forgot PIN?'),
        content: const Text(
          'This will erase your saved session. '
          'You will need to activate your device again.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Erase & Reset'),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      await BiometricService.disable();
      await ref.read(authProvider.notifier).logout();
      if (mounted) context.go('/activate');
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(32),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 360),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  Icons.lock_outline,
                  size: 64,
                  color: theme.colorScheme.primary,
                ),
                const SizedBox(height: 24),
                Text(
                  'Llave',
                  style: theme.textTheme.headlineMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                    color: theme.colorScheme.primary,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  'Enter your PIN to unlock',
                  style: theme.textTheme.bodyLarge?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 32),
                TextField(
                  controller: _pinController,
                  autofocus: !_biometricAvailable,
                  obscureText: _obscure,
                  keyboardType: TextInputType.number,
                  textInputAction: TextInputAction.done,
                  onSubmitted: (_) => _unlock(),
                  decoration: InputDecoration(
                    labelText: 'PIN',
                    border: const OutlineInputBorder(),
                    errorText: _error,
                    suffixIcon: IconButton(
                      icon: Icon(_obscure
                          ? Icons.visibility_outlined
                          : Icons.visibility_off_outlined),
                      onPressed: () => setState(() => _obscure = !_obscure),
                    ),
                  ),
                ),
                const SizedBox(height: 24),
                SizedBox(
                  width: double.infinity,
                  child: FilledButton(
                    onPressed: _loading ? null : _unlock,
                    child: _loading
                        ? const SizedBox(
                            width: 20,
                            height: 20,
                            child: CircularProgressIndicator(
                                strokeWidth: 2, color: Colors.white),
                          )
                        : const Text('Unlock'),
                  ),
                ),
                if (_biometricAvailable) ...[
                  const SizedBox(height: 12),
                  SizedBox(
                    width: double.infinity,
                    child: OutlinedButton.icon(
                      onPressed: _loading ? null : _unlockWithBiometrics,
                      icon: const Icon(Icons.fingerprint),
                      label: const Text('Unlock with biometrics'),
                    ),
                  ),
                ],
                const SizedBox(height: 16),
                TextButton(
                  onPressed: _forgotPin,
                  child: const Text('Forgot PIN?'),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
