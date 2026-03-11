import 'package:flutter/material.dart';

/// Sentinel prefix emitted by Rust `LlaveError::IpRateLimited`.
const _prefix = 'IP rate-limited by AEAT';

/// Returns `true` when an error string indicates the AEAT server has
/// temporarily blocked this IP address (error code 16600).
bool isIpRateLimited(String? error) {
  if (error == null) return false;
  return error.contains(_prefix);
}

/// Shows a persistent [MaterialBanner] warning the user that their IP
/// has been temporarily blocked by AEAT.
///
/// The banner stays visible until the user dismisses it.  Safe to call
/// multiple times — call [ScaffoldMessenger.clearMaterialBanners] first
/// to avoid stacking.
void showIpRateLimitBanner(BuildContext context) {
  final theme = Theme.of(context);
  final messenger = ScaffoldMessenger.of(context);

  // Avoid stacking multiple banners.
  messenger.clearMaterialBanners();

  messenger.showMaterialBanner(
    MaterialBanner(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
      leading: Icon(Icons.block, color: theme.colorScheme.error),
      backgroundColor: theme.colorScheme.errorContainer,
      content: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            'IP temporarily blocked by AEAT',
            style: theme.textTheme.titleSmall?.copyWith(
              color: theme.colorScheme.onErrorContainer,
              fontWeight: FontWeight.bold,
            ),
          ),
          const SizedBox(height: 4),
          Text(
            'Your IP address has exceeded the maximum number of failed '
            'attempts allowed per day. Try again tomorrow, or use a '
            'VPN/proxy to switch IP addresses.',
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onErrorContainer,
            ),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => messenger.hideCurrentMaterialBanner(),
          child: Text(
            'Dismiss',
            style: TextStyle(color: theme.colorScheme.onErrorContainer),
          ),
        ),
      ],
    ),
  );
}
