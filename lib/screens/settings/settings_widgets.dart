// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/panel_card.dart';

/// A titled group of settings rows.
class SettingsGroup extends StatelessWidget {
  const SettingsGroup({super.key, required this.title, required this.children, this.subtitle});

  final String title;
  final String? subtitle;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 16),
      child: PanelCard(
        title: title,
        subtitle: subtitle,
        child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: children),
      ),
    );
  }
}

class SettingsSwitch extends StatelessWidget {
  const SettingsSwitch({super.key, required this.title, required this.value, required this.onChanged, this.subtitle});

  final String title;
  final String? subtitle;
  final bool value;
  final ValueChanged<bool>? onChanged;

  @override
  Widget build(BuildContext context) {
    return SwitchListTile(
      contentPadding: EdgeInsets.zero,
      title: Text(title),
      subtitle: subtitle == null ? null : Text(subtitle!, style: TextStyle(color: context.voxel.muted)),
      value: value,
      onChanged: onChanged,
    );
  }
}

class SettingsRow extends StatelessWidget {
  const SettingsRow({super.key, required this.title, required this.child, this.subtitle});

  final String title;
  final String? subtitle;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final label = Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title),
              if (subtitle != null) Text(subtitle!, style: TextStyle(color: context.voxel.muted, fontSize: 12)),
            ],
          );
          if (constraints.maxWidth < 560) {
            return Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [label, const SizedBox(height: 8), child]);
          }
          return Row(
            children: [
              Expanded(child: label),
              const SizedBox(width: 16),
              SizedBox(width: 280, child: child),
            ],
          );
        },
      ),
    );
  }
}

/// Text field that commits when editing ends (enter or focus loss), not on every keystroke.
class CommitTextField extends StatefulWidget {
  const CommitTextField({
    super.key,
    required this.value,
    required this.onCommit,
    this.hint,
    this.obscure = false,
    this.numeric = false,
    this.suffix,
  });

  final String value;
  final ValueChanged<String> onCommit;
  final String? hint;
  final bool obscure;
  final bool numeric;
  final String? suffix;

  @override
  State<CommitTextField> createState() => _CommitTextFieldState();
}

class _CommitTextFieldState extends State<CommitTextField> {
  late final _controller = TextEditingController(text: widget.value);
  final _focus = FocusNode();
  var _hidden = true;

  @override
  void initState() {
    super.initState();
    _focus.addListener(() {
      if (!_focus.hasFocus) {
        _commit();
      }
    });
  }

  @override
  void didUpdateWidget(CommitTextField oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.value != widget.value && !_focus.hasFocus) {
      _controller.text = widget.value;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    _focus.dispose();
    super.dispose();
  }

  void _commit() {
    if (_controller.text != widget.value) {
      widget.onCommit(_controller.text.trim());
    }
  }

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: _controller,
      focusNode: _focus,
      obscureText: widget.obscure && _hidden,
      keyboardType: widget.numeric ? TextInputType.number : null,
      inputFormatters: widget.numeric ? [FilteringTextInputFormatter.digitsOnly] : null,
      decoration: InputDecoration(
        isDense: true,
        hintText: widget.hint,
        suffixText: widget.suffix,
        suffixIcon: widget.obscure ? IconButton(onPressed: () => setState(() => _hidden = !_hidden), icon: Icon(_hidden ? Icons.visibility : Icons.visibility_off)) : null,
      ),
      onSubmitted: (_) => _commit(),
    );
  }
}

class SettingsDropdown<T> extends StatelessWidget {
  const SettingsDropdown({super.key, required this.value, required this.items, required this.onChanged});

  final T value;
  final Map<T, String> items;
  final ValueChanged<T> onChanged;

  @override
  Widget build(BuildContext context) {
    return DropdownButtonFormField<T>(
      initialValue: items.containsKey(value) ? value : null,
      isExpanded: true,
      decoration: const InputDecoration(isDense: true),
      items: [for (final entry in items.entries) DropdownMenuItem(value: entry.key, child: Text(entry.value, overflow: TextOverflow.ellipsis))],
      onChanged: (value) {
        if (value != null) {
          onChanged(value);
        }
      },
    );
  }
}
