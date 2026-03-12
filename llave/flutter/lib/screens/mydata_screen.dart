import 'dart:convert';

import 'package:flutter/material.dart';

import '../src/rust/api/api.dart' as native_ffi;

class MyDataScreen extends StatefulWidget {
  const MyDataScreen({super.key});

  @override
  State<MyDataScreen> createState() => _MyDataScreenState();
}

class _MyDataScreenState extends State<MyDataScreen> {
  bool _loading = false;
  Map<String, dynamic>? _data;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });

    try {
      final result = await native_ffi.getMyData();
      if (!result.ok) {
        setState(() => _error = result.error ?? 'Unknown error');
        return;
      }
      final parsed = jsonDecode(result.data);
      setState(() => _data = parsed is Map<String, dynamic> ? parsed : {'raw': parsed});
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
      appBar: AppBar(
        title: const Text('My Data'),
        actions: [
          IconButton(
            onPressed: _loading ? null : _load,
            icon: const Icon(Icons.refresh),
            tooltip: 'Refresh',
          ),
        ],
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
              ? Center(
                  child: Padding(
                    padding: const EdgeInsets.all(24),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(Icons.error_outline, size: 48, color: theme.colorScheme.error),
                        const SizedBox(height: 16),
                        Text(_error!, textAlign: TextAlign.center),
                        const SizedBox(height: 16),
                        FilledButton(onPressed: _load, child: const Text('Retry')),
                      ],
                    ),
                  ),
                )
              : _data == null
                  ? const Center(child: Text('No data'))
                  : RefreshIndicator(
                      onRefresh: _load,
                      child: ListView(
                        padding: const EdgeInsets.all(16),
                        children: [
                          Card(
                            child: Padding(
                              padding: const EdgeInsets.all(20),
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Row(
                                    children: [
                                      Icon(Icons.person, color: theme.colorScheme.primary),
                                      const SizedBox(width: 12),
                                      Text('Account Information',
                                          style: theme.textTheme.titleMedium),
                                    ],
                                  ),
                                  const SizedBox(height: 16),
                                  ..._buildFields(theme),
                                ],
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
    );
  }

  List<Widget> _buildFields(ThemeData theme) {
    if (_data == null) return [];

    // Fields from ClaveCheckMyDataSv + session NIF.
    final labels = {
      'nif': 'NIF',
      'email': 'Email',
      'numTelefono': 'Phone',
      'nivelRegistro': 'Registration Level',
    };

    final widgets = <Widget>[];
    for (final entry in _data!.entries) {
      if (entry.value == null) continue;
      final label = labels[entry.key] ?? entry.key;
      final value = entry.value.toString();
      widgets.add(Padding(
        padding: const EdgeInsets.only(bottom: 12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SizedBox(
              width: 120,
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
      ));
    }
    return widgets;
  }
}
