import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'screens/home_screen.dart';
import 'screens/activate_screen.dart';
import 'screens/pin_screen.dart';
import 'screens/pending_screen.dart';
import 'screens/history_screen.dart';
import 'screens/settings_screen.dart';
import 'screens/splash_screen.dart';
import 'screens/dev_screen.dart';
import 'src/auth_provider.dart';
import 'src/theme_provider.dart';

void main() {
  // TODO: Initialize flutter_rust_bridge runtime here after codegen:
  // await RustLib.init();
  runApp(const ProviderScope(child: LlaveApp()));
}

/// Routes that require an active session.
const _protectedRoutes = {'/', '/pin', '/pending', '/history', '/settings'};

/// Routes that should not be accessible when already authenticated.
const _guestOnlyRoutes = {'/activate'};

GoRouter _buildRouter(WidgetRef ref) {
  return GoRouter(
    initialLocation: '/splash',
    redirect: (context, state) {
      final auth = ref.read(authProvider);
      final location = state.uri.path;

      // Never redirect away from splash — it handles its own navigation.
      if (location == '/splash') return null;

      final isAuthenticated = auth is AuthAuthenticated;

      // Redirect unauthenticated users away from protected routes.
      if (!isAuthenticated && _protectedRoutes.contains(location)) {
        return '/activate';
      }

      // Redirect authenticated users away from guest-only routes.
      if (isAuthenticated && _guestOnlyRoutes.contains(location)) {
        return '/';
      }

      return null;
    },
    routes: [
      GoRoute(path: '/splash', builder: (_, __) => const SplashScreen()),
      GoRoute(path: '/activate', builder: (_, __) => const ActivateScreen()),
      ShellRoute(
        builder: (context, state, child) => AppShell(child: child),
        routes: [
          GoRoute(path: '/', builder: (_, __) => const HomeScreen()),
          GoRoute(path: '/pin', builder: (_, __) => const PinScreen()),
          GoRoute(path: '/pending', builder: (_, __) => const PendingScreen()),
          GoRoute(path: '/history', builder: (_, __) => const HistoryScreen()),
          GoRoute(
              path: '/settings', builder: (_, __) => const SettingsScreen()),
          if (DevScreen.isEnabled)
            GoRoute(path: '/dev', builder: (_, __) => const DevScreen()),
        ],
      ),
    ],
  );
}

class LlaveApp extends ConsumerWidget {
  const LlaveApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeModeProvider);

    // Rebuild router when auth state changes so redirects re-evaluate.
    ref.watch(authProvider);
    final router = _buildRouter(ref);

    return MaterialApp.router(
      title: 'Llave AEAT',
      debugShowCheckedModeBanner: false,
      themeMode: themeMode,
      theme: ThemeData(
        colorSchemeSeed: const Color(0xFF2563EB),
        useMaterial3: true,
        brightness: Brightness.light,
      ),
      darkTheme: ThemeData(
        colorSchemeSeed: const Color(0xFF2563EB),
        useMaterial3: true,
        brightness: Brightness.dark,
      ),
      routerConfig: router,
    );
  }
}

/// Shell with bottom navigation bar for mobile, navigation rail for desktop.
class AppShell extends StatelessWidget {
  final Widget child;
  const AppShell({super.key, required this.child});

  static const _railDestinations = [
    NavigationRailDestination(
        icon: Icon(Icons.home_outlined),
        selectedIcon: Icon(Icons.home),
        label: Text('Home')),
    NavigationRailDestination(
        icon: Icon(Icons.notifications_outlined),
        selectedIcon: Icon(Icons.notifications),
        label: Text('Pending')),
    NavigationRailDestination(
        icon: Icon(Icons.pin_outlined),
        selectedIcon: Icon(Icons.pin),
        label: Text('PIN')),
    NavigationRailDestination(
        icon: Icon(Icons.history_outlined),
        selectedIcon: Icon(Icons.history),
        label: Text('History')),
    NavigationRailDestination(
        icon: Icon(Icons.settings_outlined),
        selectedIcon: Icon(Icons.settings),
        label: Text('Settings')),
  ];

  static const _barDestinations = [
    NavigationDestination(
        icon: Icon(Icons.home_outlined),
        selectedIcon: Icon(Icons.home),
        label: 'Home'),
    NavigationDestination(
        icon: Icon(Icons.notifications_outlined),
        selectedIcon: Icon(Icons.notifications),
        label: 'Pending'),
    NavigationDestination(
        icon: Icon(Icons.pin_outlined),
        selectedIcon: Icon(Icons.pin),
        label: 'PIN'),
    NavigationDestination(
        icon: Icon(Icons.history_outlined),
        selectedIcon: Icon(Icons.history),
        label: 'History'),
    NavigationDestination(
        icon: Icon(Icons.settings_outlined),
        selectedIcon: Icon(Icons.settings),
        label: 'Settings'),
  ];

  static const _routes = ['/', '/pending', '/pin', '/history', '/settings'];

  int _currentIndex(BuildContext context) {
    final location = GoRouterState.of(context).uri.toString();
    final idx = _routes.indexOf(location);
    return idx >= 0 ? idx : 0;
  }

  @override
  Widget build(BuildContext context) {
    final width = MediaQuery.sizeOf(context).width;
    final useRail = width >= 600;
    final selectedIndex = _currentIndex(context);

    if (useRail) {
      return Scaffold(
        body: Row(
          children: [
            NavigationRail(
              selectedIndex: selectedIndex,
              onDestinationSelected: (i) => context.go(_routes[i]),
              labelType: NavigationRailLabelType.all,
              destinations: _railDestinations,
            ),
            const VerticalDivider(thickness: 1, width: 1),
            Expanded(child: child),
          ],
        ),
      );
    }

    return Scaffold(
      body: child,
      bottomNavigationBar: NavigationBar(
        selectedIndex: selectedIndex,
        onDestinationSelected: (i) => context.go(_routes[i]),
        destinations: _barDestinations,
      ),
    );
  }
}
