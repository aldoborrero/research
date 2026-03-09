import 'dart:convert';

import 'package:flutter/material.dart';

import '../src/rust/api/api.dart' as native_ffi;

class PendingScreen extends StatefulWidget {
  const PendingScreen({super.key});

  @override
  State<PendingScreen> createState() => _PendingScreenState();
}

class _PendingScreenState extends State<PendingScreen> {
  bool _loading = false;
  List<Map<String, dynamic>> _requests = [];
  String? _error;

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  Future<void> _refresh() async {
    setState(() {
      _loading = true;
      _error = null;
    });

    try {
      final result = await native_ffi.getPendingRequests();
      if (!result.ok) {
        setState(() => _error = result.error ?? 'Unknown error');
        return;
      }
      final parsed = jsonDecode(result.data);
      final data = parsed['data'];
      final List<Map<String, dynamic>> items = [];
      if (data is Map<String, dynamic> && data.isNotEmpty) {
        // Single request object — wrap in list
        items.add(Map<String, dynamic>.from(data));
      } else if (data is List) {
        for (final item in data) {
          if (item is Map<String, dynamic>) items.add(item);
        }
      }
      setState(() => _requests = items);
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _confirm(String token, String idpCode) async {
    try {
      final result = await native_ffi.confirmRequest(token: token, idpCode: idpCode);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(result.ok ? 'Request confirmed' : (result.error ?? 'Failed'))),
        );
        _refresh();
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Error: $e')),
        );
      }
    }
  }

  Future<void> _reject(String token, String idpCode) async {
    try {
      final result = await native_ffi.rejectRequest(token: token, idpCode: idpCode);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(result.ok ? 'Request rejected' : (result.error ?? 'Failed'))),
        );
        _refresh();
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Error: $e')),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Pending Requests'),
        actions: [
          IconButton(
            onPressed: _loading ? null : _refresh,
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
                        FilledButton(onPressed: _refresh, child: const Text('Retry')),
                      ],
                    ),
                  ),
                )
              : _requests.isEmpty
                  ? Center(
                      child: Padding(
                        padding: const EdgeInsets.all(24),
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(
                              Icons.inbox_outlined,
                              size: 64,
                              color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.4),
                            ),
                            const SizedBox(height: 16),
                            Text(
                              'No pending requests',
                              style: theme.textTheme.titleMedium,
                            ),
                            const SizedBox(height: 8),
                            Text(
                              'Authentication requests will appear here\nwhen detected by the server.',
                              textAlign: TextAlign.center,
                              style: theme.textTheme.bodyMedium?.copyWith(
                                color: theme.colorScheme.onSurfaceVariant,
                              ),
                            ),
                          ],
                        ),
                      ),
                    )
                  : ListView.builder(
                      padding: const EdgeInsets.all(16),
                      itemCount: _requests.length,
                      itemBuilder: (context, index) {
                        final req = _requests[index];
                        final token = req['tokenClaveMovil']?.toString() ?? '';
                        final idpCode = req['codigoIdP']?.toString() ?? '';
                        final organismo = req['nombreProveedor']?.toString() ??
                            req['organismo']?.toString() ??
                            'Unknown provider';

                        return Card(
                          margin: const EdgeInsets.only(bottom: 12),
                          child: Padding(
                            padding: const EdgeInsets.all(16),
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Row(
                                  children: [
                                    Icon(Icons.notifications_active,
                                        color: theme.colorScheme.primary),
                                    const SizedBox(width: 12),
                                    Expanded(
                                      child: Text(
                                        organismo,
                                        style: theme.textTheme.titleSmall,
                                      ),
                                    ),
                                  ],
                                ),
                                if (idpCode.isNotEmpty) ...[
                                  const SizedBox(height: 8),
                                  Text(
                                    'Provider code: $idpCode',
                                    style: theme.textTheme.bodySmall?.copyWith(
                                      color: theme.colorScheme.onSurfaceVariant,
                                    ),
                                  ),
                                ],
                                const SizedBox(height: 16),
                                Row(
                                  mainAxisAlignment: MainAxisAlignment.end,
                                  children: [
                                    OutlinedButton.icon(
                                      onPressed: () => _showConfirmDialog(
                                        context,
                                        reject: true,
                                        onConfirm: () => _reject(token, idpCode),
                                      ),
                                      icon: const Icon(Icons.close, size: 18),
                                      label: const Text('Reject'),
                                      style: OutlinedButton.styleFrom(
                                        foregroundColor: theme.colorScheme.error,
                                      ),
                                    ),
                                    const SizedBox(width: 12),
                                    FilledButton.icon(
                                      onPressed: () => _showConfirmDialog(
                                        context,
                                        reject: false,
                                        onConfirm: () => _confirm(token, idpCode),
                                      ),
                                      icon: const Icon(Icons.check, size: 18),
                                      label: const Text('Confirm'),
                                    ),
                                  ],
                                ),
                              ],
                            ),
                          ),
                        );
                      },
                    ),
    );
  }

  void _showConfirmDialog(
    BuildContext context, {
    required bool reject,
    required VoidCallback onConfirm,
  }) {
    final action = reject ? 'Reject' : 'Confirm';
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('$action request?'),
        content: Text('Are you sure you want to ${action.toLowerCase()} this authentication request?'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
          FilledButton(
            onPressed: () {
              Navigator.pop(ctx);
              onConfirm();
            },
            child: Text(action),
          ),
        ],
      ),
    );
  }
}
