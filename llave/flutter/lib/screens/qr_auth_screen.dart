import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import '../src/llave_bridge.dart';

/// Whether the current platform supports camera-based QR scanning.
bool get _hasCameraSupport => Platform.isAndroid || Platform.isIOS;

class QrAuthScreen extends ConsumerStatefulWidget {
  const QrAuthScreen({super.key});

  @override
  ConsumerState<QrAuthScreen> createState() => _QrAuthScreenState();
}

class _QrAuthScreenState extends ConsumerState<QrAuthScreen> {
  final _controller = TextEditingController();
  bool _loading = false;
  String? _error;
  bool _success = false;
  bool _useManualInput = false;

  @override
  void initState() {
    super.initState();
    _useManualInput = !_hasCameraSupport;
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _authenticate(String value) async {
    if (value.trim().isEmpty) return;

    setState(() {
      _loading = true;
      _error = null;
      _success = false;
    });

    try {
      final bridge = ref.read(llaveBridgeProvider);
      final result = await bridge.qrAuthenticate(value.trim());

      if (!mounted) return;

      if (result.ok) {
        setState(() {
          _loading = false;
          _success = true;
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

  void _onQrDetected(BarcodeCapture capture) {
    if (_loading || _success) return;
    final barcode = capture.barcodes.firstOrNull;
    if (barcode?.rawValue == null) return;
    _authenticate(barcode!.rawValue!);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/'),
        ),
        title: const Text('QR Authentication'),
        actions: [
          if (_hasCameraSupport)
            IconButton(
              icon: Icon(_useManualInput ? Icons.camera_alt : Icons.keyboard),
              tooltip: _useManualInput ? 'Use camera' : 'Enter manually',
              onPressed: () =>
                  setState(() => _useManualInput = !_useManualInput),
            ),
        ],
      ),
      body: _success
          ? _buildSuccessView(theme)
          : _useManualInput
              ? _buildManualInput(theme)
              : _buildCameraScanner(theme),
    );
  }

  Widget _buildSuccessView(ThemeData theme) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.check_circle, size: 64, color: Colors.green),
            const SizedBox(height: 16),
            Text(
              'QR authentication successful',
              style: theme.textTheme.titleMedium,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 24),
            OutlinedButton.icon(
              onPressed: () => setState(() {
                _success = false;
                _controller.clear();
              }),
              icon: const Icon(Icons.qr_code),
              label: const Text('Scan another'),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildCameraScanner(ThemeData theme) {
    return Column(
      children: [
        Expanded(
          child: Stack(
            children: [
              MobileScanner(onDetect: _onQrDetected),
              // Overlay with scanning guide
              Center(
                child: Container(
                  width: 250,
                  height: 250,
                  decoration: BoxDecoration(
                    border: Border.all(
                      color: theme.colorScheme.primary.withValues(alpha: 0.7),
                      width: 3,
                    ),
                    borderRadius: BorderRadius.circular(16),
                  ),
                ),
              ),
              // Loading overlay
              if (_loading)
                Container(
                  color: Colors.black54,
                  child: const Center(
                    child: CircularProgressIndicator(color: Colors.white),
                  ),
                ),
            ],
          ),
        ),
        // Status bar at bottom
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(16),
          child: Column(
            children: [
              if (_error != null) ...[
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
                            style: TextStyle(
                                color: theme.colorScheme.onErrorContainer),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ] else
                Text(
                  _loading
                      ? 'Authenticating...'
                      : 'Point your camera at a Cl@ve QR code',
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                  textAlign: TextAlign.center,
                ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildManualInput(ThemeData theme) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.qr_code_2, size: 64, color: theme.colorScheme.primary),
            const SizedBox(height: 16),
            Text(
              'QR Code Authentication',
              style: theme.textTheme.titleMedium,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            Text(
              'Paste the QR code value from a Cl@ve authentication request.',
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 32),

            if (_error != null) ...[
              Card(
                color: theme.colorScheme.errorContainer,
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Icons.error_outline,
                          color: theme.colorScheme.error, size: 20),
                      const SizedBox(width: 8),
                      Flexible(
                        child: Text(
                          _error!,
                          style: TextStyle(
                              color: theme.colorScheme.onErrorContainer),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 16),
            ],

            // QR value input
            ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 400),
              child: TextField(
                controller: _controller,
                decoration: const InputDecoration(
                  labelText: 'QR code value',
                  hintText: 'Paste QR content here',
                  border: OutlineInputBorder(),
                  prefixIcon: Icon(Icons.qr_code),
                ),
                maxLines: 3,
                minLines: 1,
                enabled: !_loading,
                onSubmitted: (_) => _authenticate(_controller.text),
              ),
            ),
            const SizedBox(height: 16),

            FilledButton.icon(
              onPressed:
                  _loading ? null : () => _authenticate(_controller.text),
              icon: _loading
                  ? const SizedBox(
                      width: 18,
                      height: 18,
                      child: CircularProgressIndicator(
                          strokeWidth: 2, color: Colors.white),
                    )
                  : const Icon(Icons.login),
              label: Text(_loading ? 'Authenticating...' : 'Authenticate'),
            ),
          ],
        ),
      ),
    );
  }
}
