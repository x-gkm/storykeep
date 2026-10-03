import 'package:flutter/material.dart';

import '../../api/api_exception.dart';
import '../../util/format.dart';

/// A user-facing message for an error thrown by the API client or elsewhere.
String errorMessage(Object error) {
  if (error is ApiException) {
    if (error.isForbidden) return 'You don\'t have permission to do that. (${error.message})';
    if (error.isNotFound) return 'Not found, or you no longer have access. (${error.message})';
    return error.message;
  }
  return 'Something went wrong: $error';
}

void showSnack(BuildContext context, String message) {
  ScaffoldMessenger.of(context)
    ..hideCurrentSnackBar()
    ..showSnackBar(SnackBar(content: Text(message)));
}

void showError(BuildContext context, Object error) => showSnack(context, errorMessage(error));

Future<bool> confirm(
  BuildContext context, {
  required String title,
  required String message,
  String action = 'Delete',
  bool destructive = true,
}) async {
  final scheme = Theme.of(context).colorScheme;
  final result = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(title),
      content: Text(message),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Cancel')),
        FilledButton(
          style: destructive ? FilledButton.styleFrom(backgroundColor: scheme.error, foregroundColor: scheme.onError) : null,
          onPressed: () => Navigator.pop(context, true),
          child: Text(action),
        ),
      ],
    ),
  );
  return result ?? false;
}

/// Loads data once, shows a spinner / error with retry, then builds the content.
/// The builder gets a `reload` callback that refreshes while keeping old data visible.
class Loader<T> extends StatefulWidget {
  const Loader({super.key, required this.load, required this.builder, this.fullScreen = false, this.title});

  final Future<T> Function() load;

  /// Wrap the loading and error states in a [Scaffold] (for loaders that build whole screens).
  final bool fullScreen;
  final String? title;
  final Widget Function(BuildContext context, T data, Future<void> Function() reload) builder;

  @override
  State<Loader<T>> createState() => _LoaderState<T>();
}

class _LoaderState<T> extends State<Loader<T>> {
  T? _data;
  bool _hasData = false;
  Object? _error;

  @override
  void initState() {
    super.initState();
    _reload();
  }

  Future<void> _reload() async {
    try {
      final data = await widget.load();
      if (!mounted) return;
      setState(() {
        _data = data;
        _hasData = true;
        _error = null;
      });
    } catch (e) {
      if (!mounted) return;
      if (_hasData) {
        showError(context, e);
      } else {
        setState(() => _error = e);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_hasData) return widget.builder(context, _data as T, _reload);
    final Widget body = _error != null
        ? ErrorView(
            error: _error!,
            onRetry: () {
              setState(() => _error = null);
              _reload();
            },
          )
        : const Center(child: CircularProgressIndicator());
    if (!widget.fullScreen) return body;
    return Scaffold(appBar: AppBar(title: widget.title == null ? null : Text(widget.title!)), body: body);
  }
}

class ErrorView extends StatelessWidget {
  const ErrorView({super.key, required this.error, this.onRetry});

  final Object error;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.error_outline, size: 40, color: Theme.of(context).colorScheme.error),
            const SizedBox(height: 12),
            Text(errorMessage(error), textAlign: TextAlign.center),
            if (onRetry != null) ...[
              const SizedBox(height: 12),
              OutlinedButton.icon(onPressed: onRetry, icon: const Icon(Icons.refresh), label: const Text('Retry')),
            ],
          ],
        ),
      ),
    );
  }
}

class EmptyState extends StatelessWidget {
  const EmptyState({super.key, required this.icon, required this.title, this.message, this.action});

  final IconData icon;
  final String title;
  final String? message;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 48, color: theme.colorScheme.outline),
            const SizedBox(height: 12),
            Text(title, style: theme.textTheme.titleMedium, textAlign: TextAlign.center),
            if (message != null) ...[
              const SizedBox(height: 6),
              Text(message!, style: theme.textTheme.bodyMedium, textAlign: TextAlign.center),
            ],
            if (action != null) ...[const SizedBox(height: 16), action!],
          ],
        ),
      ),
    );
  }
}

/// A small rounded label, e.g. a role or category.
class Pill extends StatelessWidget {
  const Pill(this.text, {super.key, this.icon, this.color});

  final String text;
  final IconData? icon;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final bg = color ?? scheme.secondaryContainer;
    final fg = color == null ? scheme.onSecondaryContainer : scheme.onPrimary;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(color: bg, borderRadius: BorderRadius.circular(12)),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (icon != null) ...[Icon(icon, size: 14, color: fg), const SizedBox(width: 4)],
          Text(text, style: Theme.of(context).textTheme.labelSmall?.copyWith(color: fg)),
        ],
      ),
    );
  }
}

class RolePill extends StatelessWidget {
  const RolePill(this.role, {super.key});

  final String role;

  @override
  Widget build(BuildContext context) => Pill(labelOf(role), icon: roleIcon(role));
}

IconData roleIcon(String role) => switch (role) {
      'OWNER' => Icons.verified_user_outlined,
      'PARENT' => Icons.shield_outlined,
      'MEMBER' => Icons.edit_outlined,
      _ => Icons.visibility_outlined,
    };

IconData profileTypeIcon(String type) => switch (type) {
      'CHILD' => Icons.child_care,
      'PET' => Icons.pets,
      'PERSON' => Icons.person_outline,
      _ => Icons.favorite_border,
    };

IconData categoryIcon(String category) => switch (category) {
      'MILESTONE' => Icons.flag_outlined,
      'BIRTHDAY' => Icons.cake_outlined,
      'HOLIDAY' => Icons.beach_access_outlined,
      'TRAVEL' => Icons.flight_outlined,
      'FIRST_TIME' => Icons.star_outline,
      'EVERYDAY' => Icons.wb_sunny_outlined,
      _ => Icons.bookmark_border,
    };

/// A form field for a calendar date, opened with the platform date picker.
class DateFormField extends FormField<DateTime> {
  DateFormField({
    super.key,
    required String label,
    super.initialValue,
    DateTime? firstDate,
    DateTime? lastDate,
    bool clearable = false,
    ValueChanged<DateTime?>? onChanged,
    super.validator,
    String? helperText,
  }) : super(
          builder: (state) {
            final context = state.context;
            Future<void> pick() async {
              final first = firstDate ?? DateTime(1900);
              final last = lastDate ?? DateTime(2200);
              var initial = state.value ?? today();
              if (initial.isBefore(first)) initial = first;
              if (initial.isAfter(last)) initial = last;
              final picked = await showDatePicker(
                context: context,
                initialDate: initial,
                firstDate: first,
                lastDate: last,
              );
              if (picked != null) {
                state.didChange(picked);
                onChanged?.call(picked);
              }
            }

            return InkWell(
              onTap: pick,
              borderRadius: BorderRadius.circular(4),
              child: InputDecorator(
                decoration: InputDecoration(
                  labelText: label,
                  helperText: helperText,
                  errorText: state.errorText,
                  prefixIcon: const Icon(Icons.event_outlined),
                  suffixIcon: clearable && state.value != null
                      ? IconButton(
                          tooltip: 'Clear',
                          icon: const Icon(Icons.clear),
                          onPressed: () {
                            state.didChange(null);
                            onChanged?.call(null);
                          },
                        )
                      : null,
                ),
                isEmpty: state.value == null,
                child: Text(state.value == null ? '' : formatDate(state.value!)),
              ),
            );
          },
        );
}

/// A full-width primary button that shows a spinner while [busy].
class BusyButton extends StatelessWidget {
  const BusyButton({super.key, required this.busy, required this.onPressed, required this.label});

  final bool busy;
  final VoidCallback onPressed;
  final String label;

  @override
  Widget build(BuildContext context) {
    return FilledButton(
      onPressed: busy ? null : onPressed,
      style: FilledButton.styleFrom(minimumSize: const Size.fromHeight(48)),
      child: busy
          ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))
          : Text(label),
    );
  }
}

/// Section heading used inside detail screens.
class SectionTitle extends StatelessWidget {
  const SectionTitle(this.text, {super.key, this.trailing});

  final String text;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: 16, bottom: 8),
      child: Row(
        children: [
          Expanded(child: Text(text, style: Theme.of(context).textTheme.titleSmall)),
          ?trailing,
        ],
      ),
    );
  }
}
