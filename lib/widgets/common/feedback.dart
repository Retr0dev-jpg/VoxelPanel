// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/theme.dart';

void showMessage(BuildContext context, String message) {
  if (!context.mounted) {
    return;
  }
  ScaffoldMessenger.of(context)
    ..hideCurrentSnackBar()
    ..showSnackBar(SnackBar(content: Text(message)));
}

void showError(BuildContext context, Object error) {
  if (context.mounted) {
    showMessage(context, describeError(context, error));
  }
}

/// Runs [action] and reports failures with a snackbar. Returns whether it succeeded.
Future<bool> runGuarded(BuildContext context, Future<void> Function() action, {String? success}) async {
  try {
    await action();
    if (success != null && context.mounted) {
      showMessage(context, success);
    }
    return true;
  } catch (error) {
    if (context.mounted) {
      showError(context, error);
    }
    return false;
  }
}

Future<bool> confirmAction(
  BuildContext context, {
  required String title,
  required String message,
  String? confirmLabel,
  bool destructive = false,
}) async {
  final l = context.l10n;
  final result = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(title),
      content: Text(message),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context, false), child: Text(l.cancel)),
        FilledButton(
          style: destructive ? actionButtonStyle(context.voxel.stop) : null,
          onPressed: () => Navigator.pop(context, true),
          child: Text(confirmLabel ?? l.confirm),
        ),
      ],
    ),
  );
  return result ?? false;
}

class EmptyState extends StatelessWidget {
  const EmptyState({super.key, required this.icon, required this.message, this.action});

  final IconData icon;
  final String message;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final colors = context.voxel;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 48, color: colors.muted),
            const SizedBox(height: 12),
            Text(message, textAlign: TextAlign.center, style: TextStyle(color: colors.muted)),
            if (action != null) ...[const SizedBox(height: 16), action!],
          ],
        ),
      ),
    );
  }
}

class ErrorState extends StatelessWidget {
  const ErrorState({super.key, required this.error, this.onRetry});

  final Object error;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    return EmptyState(
      icon: Icons.error_outline,
      message: describeError(context, error),
      action: onRetry == null ? null : OutlinedButton(onPressed: onRetry, child: Text(context.l10n.retry)),
    );
  }
}

/// Button that disables itself and shows a spinner while its async callback runs.
class AsyncActionButton extends StatefulWidget {
  const AsyncActionButton({super.key, required this.label, required this.onPressed, this.icon, this.style, this.tonal = false});

  final String label;
  final Future<void> Function()? onPressed;
  final IconData? icon;
  final ButtonStyle? style;
  final bool tonal;

  @override
  State<AsyncActionButton> createState() => _AsyncActionButtonState();
}

class _AsyncActionButtonState extends State<AsyncActionButton> {
  var _busy = false;

  Future<void> _run() async {
    setState(() => _busy = true);
    try {
      await widget.onPressed!();
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final onPressed = widget.onPressed == null || _busy ? null : _run;
    final icon = _busy
        ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2))
        : widget.icon == null
        ? null
        : Icon(widget.icon);
    final label = Text(widget.label);
    if (widget.tonal) {
      return icon == null
          ? FilledButton.tonal(style: widget.style, onPressed: onPressed, child: label)
          : FilledButton.tonalIcon(style: widget.style, onPressed: onPressed, icon: icon, label: label);
    }
    return icon == null
        ? FilledButton(style: widget.style, onPressed: onPressed, child: label)
        : FilledButton.icon(style: widget.style, onPressed: onPressed, icon: icon, label: label);
  }
}
