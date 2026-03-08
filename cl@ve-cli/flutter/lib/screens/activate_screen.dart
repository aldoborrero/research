import 'package:flutter/material.dart';

class ActivateScreen extends StatefulWidget {
  const ActivateScreen({super.key});

  @override
  State<ActivateScreen> createState() => _ActivateScreenState();
}

class _ActivateScreenState extends State<ActivateScreen> {
  final _formKey = GlobalKey<FormState>();
  final _nifController = TextEditingController();
  final _passwordController = TextEditingController();
  bool _loading = false;

  @override
  void dispose() {
    _nifController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  Future<void> _activate() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() => _loading = true);

    try {
      // TODO: Call Rust bridge:
      // final session = await activateDevice(
      //   nif: _nifController.text,
      //   password: _passwordController.text.isEmpty ? null : _passwordController.text,
      // );
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Device activated successfully')),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Activation failed: $e')),
        );
      }
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Activate Device')),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16),
        child: Form(
          key: _formKey,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Card(
                child: Padding(
                  padding: const EdgeInsets.all(20),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Register this device with your Cl@ve account.',
                        style: Theme.of(context).textTheme.bodyMedium,
                      ),
                      const SizedBox(height: 24),
                      TextFormField(
                        controller: _nifController,
                        decoration: const InputDecoration(
                          labelText: 'NIF (DNI/NIE)',
                          hintText: '12345678A',
                          border: OutlineInputBorder(),
                          prefixIcon: Icon(Icons.badge),
                        ),
                        textCapitalization: TextCapitalization.characters,
                        validator: (v) {
                          if (v == null || v.trim().isEmpty) return 'NIF is required';
                          if (v.trim().length < 8) return 'NIF must be at least 8 characters';
                          return null;
                        },
                      ),
                      const SizedBox(height: 16),
                      TextFormField(
                        controller: _passwordController,
                        decoration: const InputDecoration(
                          labelText: 'Password (optional)',
                          hintText: 'Auto-generated if empty',
                          border: OutlineInputBorder(),
                          prefixIcon: Icon(Icons.lock_outline),
                        ),
                        obscureText: true,
                      ),
                      const SizedBox(height: 24),
                      FilledButton.icon(
                        onPressed: _loading ? null : _activate,
                        icon: _loading
                            ? const SizedBox(
                                width: 18,
                                height: 18,
                                child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                              )
                            : const Icon(Icons.shield),
                        label: Text(_loading ? 'Activating...' : 'Activate'),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
