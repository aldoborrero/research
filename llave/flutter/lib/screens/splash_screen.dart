import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../src/auth_provider.dart';

/// Splash screen shown at startup.
///
/// Initialises the Rust bridge, checks session state, then navigates to
/// the home screen (active session) or the activate screen (no session).
class SplashScreen extends ConsumerStatefulWidget {
  const SplashScreen({super.key});

  @override
  ConsumerState<SplashScreen> createState() => _SplashScreenState();
}

class _SplashScreenState extends ConsumerState<SplashScreen>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _fadeIn;
  String _statusText = 'Loading...';

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 800),
    );
    _fadeIn = CurvedAnimation(parent: _controller, curve: Curves.easeIn);
    _controller.forward();
    _init();
  }

  Future<void> _init() async {
    try {
      // TODO: Initialise flutter_rust_bridge runtime:
      // await RustLib.init();

      setState(() => _statusText = 'Checking session...');

      // Initialise the Rust core and restore any persisted session from
      // platform secure storage (Android Keystore / iOS Keychain).
      await ref.read(authProvider.notifier).init();

      // Brief delay so splash is visible even on fast devices.
      await Future<void>.delayed(const Duration(milliseconds: 600));

      if (!mounted) return;

      final auth = ref.read(authProvider);
      if (auth is AuthAuthenticated) {
        context.go('/');
      } else {
        context.go('/activate');
      }
    } catch (e) {
      if (!mounted) return;
      setState(() => _statusText = 'Error: $e');
      await Future<void>.delayed(const Duration(seconds: 2));
      if (mounted) context.go('/activate');
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      body: Center(
        child: FadeTransition(
          opacity: _fadeIn,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                Icons.shield,
                size: 80,
                color: theme.colorScheme.primary,
              ),
              const SizedBox(height: 24),
              Text(
                'Llave',
                style: theme.textTheme.headlineLarge?.copyWith(
                  fontWeight: FontWeight.bold,
                  color: theme.colorScheme.primary,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                'AEAT',
                style: theme.textTheme.titleMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                  letterSpacing: 4,
                ),
              ),
              const SizedBox(height: 48),
              const SizedBox(
                width: 24,
                height: 24,
                child: CircularProgressIndicator(strokeWidth: 2),
              ),
              const SizedBox(height: 16),
              Text(
                _statusText,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
