import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

class PinScreen extends StatefulWidget {
  const PinScreen({super.key});

  @override
  State<PinScreen> createState() => _PinScreenState();
}

class _PinScreenState extends State<PinScreen> {
  bool _loading = false;
  String? _pin;
  String? _ttl;
  String? _error;

  Future<void> _requestPin() async {
    setState(() {
      _loading = true;
      _error = null;
    });

    try {
      // TODO: Call Rust bridge:
      // final result = await requestPin();
      // _pin = result.pin;
      // _ttl = result.timeToLiveSeconds;
      setState(() {
        _pin = '------'; // placeholder
        _ttl = '180';
      });
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(title: const Text('Request PIN')),
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.pin, size: 64, color: theme.colorScheme.primary),
              const SizedBox(height: 16),
              Text(
                'Generate a temporary Cl@ve PIN',
                style: theme.textTheme.titleMedium,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 8),
              Text(
                'Use it to authenticate on government portals.',
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 32),

              if (_pin != null) ...[
                Card(
                  color: theme.colorScheme.primaryContainer,
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 24),
                    child: Column(
                      children: [
                        Text(
                          _pin!,
                          style: theme.textTheme.displaySmall?.copyWith(
                            fontWeight: FontWeight.bold,
                            letterSpacing: 8,
                            color: theme.colorScheme.onPrimaryContainer,
                          ),
                        ),
                        const SizedBox(height: 8),
                        Text(
                          'Valid for $_ttl seconds',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onPrimaryContainer.withValues(alpha: 0.7),
                          ),
                        ),
                        const SizedBox(height: 12),
                        OutlinedButton.icon(
                          onPressed: () {
                            Clipboard.setData(ClipboardData(text: _pin!));
                            ScaffoldMessenger.of(context).showSnackBar(
                              const SnackBar(content: Text('PIN copied to clipboard')),
                            );
                          },
                          icon: const Icon(Icons.copy, size: 16),
                          label: const Text('Copy'),
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: 16),
              ],

              if (_error != null) ...[
                Card(
                  color: theme.colorScheme.errorContainer,
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Text(
                      _error!,
                      style: TextStyle(color: theme.colorScheme.onErrorContainer),
                    ),
                  ),
                ),
                const SizedBox(height: 16),
              ],

              FilledButton.icon(
                onPressed: _loading ? null : _requestPin,
                icon: _loading
                    ? const SizedBox(
                        width: 18,
                        height: 18,
                        child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                      )
                    : const Icon(Icons.refresh),
                label: Text(_loading ? 'Generating...' : 'Generate PIN'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
