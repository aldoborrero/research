import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../src/llave_bridge.dart';

class DniAuthScreen extends ConsumerStatefulWidget {
  const DniAuthScreen({super.key});

  @override
  ConsumerState<DniAuthScreen> createState() => _DniAuthScreenState();
}

class _DniAuthScreenState extends ConsumerState<DniAuthScreen> {
  final _formKey = GlobalKey<FormState>();
  final _nifController = TextEditingController();
  final _fechaController = TextEditingController();
  final _soporteController = TextEditingController();
  bool _loading = false;
  String? _error;
  String? _successHtml;

  @override
  void dispose() {
    _nifController.dispose();
    _fechaController.dispose();
    _soporteController.dispose();
    super.dispose();
  }

  Future<void> _authenticate() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() {
      _loading = true;
      _error = null;
      _successHtml = null;
    });

    try {
      final bridge = ref.read(llaveBridgeProvider);
      final result = await bridge.dniAuthenticate(
        _nifController.text.trim(),
        _fechaController.text.trim(),
        _soporteController.text.trim(),
      );

      if (!mounted) return;

      if (result.ok) {
        setState(() {
          _loading = false;
          _successHtml = result.data;
        });
      } else {
        setState(() {
          _loading = false;
          _error = result.error ?? 'Authentication failed';
        });
      }
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _loading = false;
        _error = e.toString();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('DNI/NIE Authentication'),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/activate'),
        ),
      ),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: _successHtml != null
              ? _buildSuccessView(theme)
              : _buildForm(theme),
        ),
      ),
    );
  }

  Widget _buildSuccessView(ThemeData theme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Card(
          color: theme.colorScheme.primaryContainer,
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Icon(Icons.check_circle, color: theme.colorScheme.primary),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    'DNI/NIE authentication successful',
                    style: TextStyle(color: theme.colorScheme.onPrimaryContainer),
                  ),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),
        FilledButton.icon(
          onPressed: () => context.go('/'),
          icon: const Icon(Icons.home),
          label: const Text('Go to Home'),
        ),
      ],
    );
  }

  Widget _buildForm(ThemeData theme) {
    return Form(
      key: _formKey,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Icon(
            Icons.badge_outlined,
            size: 64,
            color: theme.colorScheme.primary,
          ),
          const SizedBox(height: 16),
          Text(
            'DNI/NIE Login',
            style: theme.textTheme.headlineMedium?.copyWith(
              fontWeight: FontWeight.bold,
            ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 8),
          Text(
            'Authenticate using the data printed on your physical DNI/NIE card. No certificate or password needed.',
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 32),

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
              if (v.trim().length < 8) return 'NIF must be at least 8 characters';
              return null;
            },
          ),
          const SizedBox(height: 16),

          // Fecha (expiry date)
          TextFormField(
            controller: _fechaController,
            decoration: const InputDecoration(
              labelText: 'Fecha de validez del DNI',
              hintText: 'DD/MM/YYYY',
              border: OutlineInputBorder(),
              prefixIcon: Icon(Icons.calendar_today),
              helperText: 'Expiry date printed on your DNI/NIE card',
            ),
            keyboardType: TextInputType.datetime,
            enabled: !_loading,
            validator: (v) {
              if (v == null || v.trim().isEmpty) return 'Date is required';
              final trimmed = v.trim();
              if (!RegExp(r'^\d{2}/\d{2}/\d{4}$').hasMatch(trimmed)) {
                return 'Use DD/MM/YYYY format';
              }
              return null;
            },
          ),
          const SizedBox(height: 16),

          // Soporte (support number)
          TextFormField(
            controller: _soporteController,
            decoration: const InputDecoration(
              labelText: 'Numero de soporte',
              hintText: 'e.g. BAA123456',
              border: OutlineInputBorder(),
              prefixIcon: Icon(Icons.numbers),
              helperText: 'Support number on the back of your DNI/NIE card',
            ),
            textCapitalization: TextCapitalization.characters,
            enabled: !_loading,
            validator: (v) {
              if (v == null || v.trim().isEmpty) return 'Support number is required';
              return null;
            },
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
                        style:
                            TextStyle(color: theme.colorScheme.onErrorContainer),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],

          const SizedBox(height: 24),

          // Submit button
          FilledButton.icon(
            onPressed: _loading ? null : _authenticate,
            icon: _loading
                ? const SizedBox(
                    width: 18,
                    height: 18,
                    child: CircularProgressIndicator(
                        strokeWidth: 2, color: Colors.white),
                  )
                : const Icon(Icons.login),
            label: Text(_loading ? 'Authenticating...' : 'Authenticate'),
            style: FilledButton.styleFrom(
              padding: const EdgeInsets.symmetric(vertical: 16),
            ),
          ),
        ],
      ),
    );
  }
}
