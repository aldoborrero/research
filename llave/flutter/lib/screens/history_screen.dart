import 'dart:convert';

import 'package:flutter/material.dart';

import '../src/rust/api/api.dart' as native_ffi;

class HistoryScreen extends StatefulWidget {
  const HistoryScreen({super.key});

  @override
  State<HistoryScreen> createState() => _HistoryScreenState();
}

class _HistoryScreenState extends State<HistoryScreen> {
  bool _loading = false;
  List<Map<String, dynamic>> _operations = [];
  String? _error;

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });

    try {
      final result = await native_ffi.getHistory();
      if (!result.ok) {
        setState(() => _error = result.error ?? 'Unknown error');
        return;
      }
      final parsed = jsonDecode(result.data);
      final List<Map<String, dynamic>> items = [];
      // The Rust core returns { operaciones: [...] }
      final ops = parsed is Map ? (parsed['operaciones'] ?? parsed) : parsed;
      if (ops is List) {
        for (final item in ops) {
          if (item is Map<String, dynamic>) items.add(item);
        }
      }
      setState(() => _operations = items);
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
        title: const Text('History'),
        actions: [
          if (_operations.isNotEmpty)
            IconButton(
              onPressed: _loading ? null : _load,
              icon: const Icon(Icons.refresh),
              tooltip: 'Refresh',
            ),
        ],
      ),
      body: _operations.isEmpty && !_loading && _error == null
          ? Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.history,
                      size: 64,
                      color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.4),
                    ),
                    const SizedBox(height: 16),
                    Text('Operations History', style: theme.textTheme.titleMedium),
                    const SizedBox(height: 8),
                    Text(
                      'View past authentication operations.',
                      style: theme.textTheme.bodyMedium?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                    const SizedBox(height: 24),
                    FilledButton.icon(
                      onPressed: _loading ? null : _load,
                      icon: const Icon(Icons.download),
                      label: const Text('Load History'),
                    ),
                  ],
                ),
              ),
            )
          : _loading
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
                            Text('Error: $_error', textAlign: TextAlign.center),
                            const SizedBox(height: 16),
                            FilledButton(onPressed: _load, child: const Text('Retry')),
                          ],
                        ),
                      ),
                    )
                  : _operations.isEmpty
                      ? Center(
                          child: Column(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Icon(Icons.history, size: 48,
                                  color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.4)),
                              const SizedBox(height: 16),
                              Text('No operations found', style: theme.textTheme.titleMedium),
                              const SizedBox(height: 16),
                              FilledButton(onPressed: _load, child: const Text('Refresh')),
                            ],
                          ),
                        )
                      : ListView.builder(
                          padding: const EdgeInsets.all(16),
                          itemCount: _operations.length,
                          itemBuilder: (context, index) =>
                              _OperationCard(operation: _operations[index]),
                        ),
    );
  }
}

class _OperationCard extends StatelessWidget {
  final Map<String, dynamic> operation;
  const _OperationCard({required this.operation});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final tipo = operation['tipo']?.toString() ?? operation['type']?.toString() ?? '';
    final resultado = operation['resultado']?.toString() ??
        operation['result']?.toString() ??
        '';
    final organismo = operation['organismo']?.toString() ??
        operation['nombreProveedor']?.toString() ??
        '';
    final fecha = operation['fecha']?.toString() ??
        operation['date']?.toString() ??
        '';

    final isSuccess = resultado.toUpperCase() == 'OK' ||
        resultado.toUpperCase() == 'ACEPTADA';

    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: ListTile(
        leading: Icon(
          _iconForType(tipo),
          color: isSuccess ? Colors.green : theme.colorScheme.error,
        ),
        title: Text(organismo.isNotEmpty ? organismo : tipo.isNotEmpty ? tipo : 'Operation'),
        subtitle: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (tipo.isNotEmpty)
              Text(tipo, style: theme.textTheme.bodySmall),
            if (fecha.isNotEmpty)
              Text(fecha, style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              )),
          ],
        ),
        trailing: Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
          decoration: BoxDecoration(
            color: isSuccess
                ? Colors.green.withValues(alpha: 0.1)
                : theme.colorScheme.errorContainer,
            borderRadius: BorderRadius.circular(8),
          ),
          child: Text(
            resultado.isNotEmpty ? resultado : '—',
            style: theme.textTheme.labelSmall?.copyWith(
              color: isSuccess ? Colors.green : theme.colorScheme.error,
              fontWeight: FontWeight.bold,
            ),
          ),
        ),
      ),
    );
  }

  static IconData _iconForType(String tipo) {
    final t = tipo.toLowerCase();
    if (t.contains('qr')) return Icons.qr_code;
    if (t.contains('dni') || t.contains('nie')) return Icons.badge;
    if (t.contains('pin')) return Icons.pin;
    return Icons.vpn_key;
  }
}
