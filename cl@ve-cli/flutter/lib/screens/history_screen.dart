import 'package:flutter/material.dart';

class HistoryScreen extends StatefulWidget {
  const HistoryScreen({super.key});

  @override
  State<HistoryScreen> createState() => _HistoryScreenState();
}

class _HistoryScreenState extends State<HistoryScreen> {
  bool _loading = false;
  String? _data;
  String? _error;

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });

    try {
      // TODO: Call Rust bridge:
      // final result = await getHistory();
      // _data = result.data;
      setState(() => _data = '[]');
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
      appBar: AppBar(title: const Text('History')),
      body: _data == null && !_loading
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
                  ? Center(child: Text('Error: $_error'))
                  : ListView(
                      padding: const EdgeInsets.all(16),
                      children: [
                        // TODO: Parse _data JSON and render operation cards
                        Card(
                          child: Padding(
                            padding: const EdgeInsets.all(16),
                            child: SelectableText(
                              _data ?? '',
                              style: theme.textTheme.bodySmall?.copyWith(
                                fontFamily: 'monospace',
                              ),
                            ),
                          ),
                        ),
                      ],
                    ),
    );
  }
}
