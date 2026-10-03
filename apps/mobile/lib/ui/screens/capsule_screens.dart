import 'dart:async';

import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../../api/api_client.dart';
import '../../state/auth_state.dart';
import '../../util/format.dart';
import '../../util/permissions.dart';
import '../../util/validators.dart';
import '../widgets/common.dart';
import '../widgets/media.dart';
import 'capsules_tab.dart';

class CapsuleDetailScreen extends StatefulWidget {
  const CapsuleDetailScreen({super.key, required this.capsuleId});

  final int capsuleId;

  @override
  State<CapsuleDetailScreen> createState() => _CapsuleDetailScreenState();
}

class _CapsuleDetailScreenState extends State<CapsuleDetailScreen> {
  Capsule? _capsule;
  Relationship? _relationship;
  Object? _error;
  bool _busy = false;

  ApiClient get _api => context.read<ApiClient>();

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final capsule = await _api.getCapsule(widget.capsuleId);
      final rel = _relationship ?? await _api.getRelationship(capsule.relationshipId);
      if (mounted) {
        setState(() {
          _capsule = capsule;
          _relationship = rel;
          _error = null;
        });
      }
    } catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  Future<void> _act(Future<void> Function() action) async {
    setState(() => _busy = true);
    try {
      await action();
    } catch (e) {
      if (mounted) showError(context, e);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _openCapsule() => _act(() async {
        final opened = await _api.openCapsule(widget.capsuleId);
        if (mounted) setState(() => _capsule = opened);
      });

  Future<void> _cancel() async {
    final ok = await confirm(context,
        title: 'Cancel capsule?',
        message: 'A cancelled capsule can never be opened. Its content stays sealed forever.',
        action: 'Cancel capsule');
    if (!ok) return;
    await _act(() async {
      final c = await _api.cancelCapsule(widget.capsuleId);
      if (mounted) setState(() => _capsule = c);
    });
  }

  Future<void> _delete() async {
    final ok = await confirm(context, title: 'Delete capsule?', message: 'The capsule is removed for everyone.');
    if (!ok) return;
    await _act(() async {
      await _api.deleteCapsule(widget.capsuleId);
      if (mounted) context.pop();
    });
  }

  Future<void> _attach(List<UploadFile> files) => _act(() async {
        await _api.uploadCapsuleMedia(widget.capsuleId, files);
        if (mounted) showSnack(context, 'Sealed ${files.length} file${files.length == 1 ? '' : 's'} in the capsule');
        await _load();
      });

  @override
  Widget build(BuildContext context) {
    final capsule = _capsule;
    final rel = _relationship;
    if (capsule == null || rel == null) {
      return Scaffold(
        appBar: AppBar(title: const Text('Time capsule')),
        body: _error != null ? ErrorView(error: _error!, onRetry: _load) : const Center(child: CircularProgressIndicator()),
      );
    }
    final me = context.select<AuthState, int?>((a) => a.user?.id);
    final canEdit = Perms.canEditItem(rel.role, createdBy: capsule.createdBy, currentUserId: me);
    return Scaffold(
      appBar: AppBar(
        title: const Text('Time capsule'),
        actions: [
          if (canEdit && capsule.isLocked)
            IconButton(
              tooltip: 'Edit',
              icon: const Icon(Icons.edit_outlined),
              onPressed: () async {
                await context.push('/capsules/${capsule.id}/edit');
                await _load();
              },
            ),
          if (canEdit)
            PopupMenuButton<String>(
              onSelected: (v) => v == 'cancel' ? _cancel() : _delete(),
              itemBuilder: (_) => [
                if (capsule.isLocked) const PopupMenuItem(value: 'cancel', child: Text('Cancel capsule')),
                const PopupMenuItem(value: 'delete', child: Text('Delete')),
              ],
            ),
        ],
      ),
      body: RefreshIndicator(
        onRefresh: _load,
        child: CapsuleBody(
          capsule: capsule,
          canAttach: Perms.canWrite(rel.role) && capsule.isLocked,
          busy: _busy,
          onOpen: _openCapsule,
          onAttach: _attach,
          onUnlocked: _load,
        ),
      ),
    );
  }
}

/// The body of a capsule's page. Content is rendered only for an `OPENED` capsule.
class CapsuleBody extends StatefulWidget {
  const CapsuleBody({
    super.key,
    required this.capsule,
    this.canAttach = false,
    this.busy = false,
    this.onOpen,
    this.onAttach,
    this.onUnlocked,
  });

  final Capsule capsule;
  final bool canAttach;
  final bool busy;
  final VoidCallback? onOpen;
  final void Function(List<UploadFile>)? onAttach;

  /// Called once when the countdown of a locked capsule reaches zero.
  final VoidCallback? onUnlocked;

  @override
  State<CapsuleBody> createState() => _CapsuleBodyState();
}

class _CapsuleBodyState extends State<CapsuleBody> {
  Timer? _timer;
  bool _notifiedUnlock = false;

  @override
  void initState() {
    super.initState();
    _syncTimer();
  }

  @override
  void didUpdateWidget(CapsuleBody old) {
    super.didUpdateWidget(old);
    if (old.capsule.status != widget.capsule.status) _notifiedUnlock = false;
    _syncTimer();
  }

  void _syncTimer() {
    _timer?.cancel();
    _timer = null;
    if (!widget.capsule.isLocked) return;
    _timer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (!mounted) return;
      setState(() {});
      if (!_notifiedUnlock && !widget.capsule.unlockAt.isAfter(DateTime.now())) {
        _notifiedUnlock = true;
        // Give the server clock a moment before asking for the new status.
        Future.delayed(const Duration(seconds: 2), () => widget.onUnlocked?.call());
      }
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final c = widget.capsule;
    final theme = Theme.of(context);
    final content = c.isOpened ? c.content : null;
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          children: [
            Icon(capsuleStatusIcon(c.status), color: theme.colorScheme.primary),
            const SizedBox(width: 8),
            Pill(labelOf(c.status)),
          ],
        ),
        const SizedBox(height: 12),
        Text(c.title, style: theme.textTheme.headlineSmall),
        const SizedBox(height: 4),
        Text('Unlock time: ${formatDateTime(c.unlockAt)}', style: theme.textTheme.bodyMedium),
        Text('Created ${formatDateTime(c.createdAt)}', style: theme.textTheme.bodySmall),
        const SizedBox(height: 20),
        if (c.isLocked) ..._locked(context, c),
        if (c.isAvailable) ...[
          Card(
            color: theme.colorScheme.tertiaryContainer,
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                children: [
                  const Icon(Icons.celebration_outlined, size: 40),
                  const SizedBox(height: 8),
                  const Text('This capsule is ready to be opened.', textAlign: TextAlign.center),
                  const SizedBox(height: 12),
                  FilledButton.icon(
                    key: const Key('open-capsule'),
                    onPressed: widget.busy ? null : widget.onOpen,
                    icon: const Icon(Icons.lock_open),
                    label: const Text('Open capsule'),
                  ),
                ],
              ),
            ),
          ),
        ],
        if (c.isCancelled)
          const Card(
            child: ListTile(
              leading: Icon(Icons.block),
              title: Text('This capsule was cancelled'),
              subtitle: Text('Its content will never be shown.'),
            ),
          ),
        if (content != null) ...[
          SectionTitle('Message'),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: SelectableText(
                key: const Key('capsule-message'),
                content.message ?? 'No message was left in this capsule.',
                style: theme.textTheme.bodyLarge,
              ),
            ),
          ),
          if (content.media.isNotEmpty) ...[
            SectionTitle('Media (${content.media.length})'),
            MediaGrid(items: content.media),
          ],
        ],
      ],
    );
  }

  List<Widget> _locked(BuildContext context, Capsule c) {
    final theme = Theme.of(context);
    final remaining = c.unlockAt.difference(DateTime.now());
    return [
      Card(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            children: [
              const Icon(Icons.lock_clock, size: 40),
              const SizedBox(height: 8),
              Text('Unlocks in', style: theme.textTheme.labelLarge),
              Text(
                remaining.isNegative ? 'any moment' : _longCountdown(remaining),
                key: const Key('capsule-countdown'),
                style: theme.textTheme.headlineSmall,
              ),
              const SizedBox(height: 8),
              Text(
                'The message${c.mediaCount > 0 ? ' and ${c.mediaCount} sealed file${c.mediaCount == 1 ? '' : 's'}' : ''} '
                'can\'t be read by anyone until then, including its creator.',
                textAlign: TextAlign.center,
                style: theme.textTheme.bodySmall,
              ),
            ],
          ),
        ),
      ),
      if (widget.canAttach && widget.onAttach != null) ...[
        SectionTitle('Add to the capsule'),
        AttachButtons(enabled: !widget.busy, onPicked: widget.onAttach!),
        if (widget.busy) const Padding(padding: EdgeInsets.only(top: 12), child: LinearProgressIndicator()),
      ],
    ];
  }

  static String _longCountdown(Duration d) {
    final days = d.inDays;
    final h = (d.inHours % 24).toString().padLeft(2, '0');
    final m = (d.inMinutes % 60).toString().padLeft(2, '0');
    final s = (d.inSeconds % 60).toString().padLeft(2, '0');
    return days > 0 ? '${days}d $h:$m:$s' : '$h:$m:$s';
  }
}

/// Creates a capsule in [relationshipId] or edits [capsuleId] while it is locked.
class CapsuleFormScreen extends StatefulWidget {
  const CapsuleFormScreen({super.key, this.relationshipId, this.capsuleId})
      : assert((relationshipId == null) != (capsuleId == null));

  final int? relationshipId;
  final int? capsuleId;

  @override
  State<CapsuleFormScreen> createState() => _CapsuleFormScreenState();
}

class _CapsuleFormScreenState extends State<CapsuleFormScreen> {
  final _form = GlobalKey<FormState>();
  final _title = TextEditingController();
  final _message = TextEditingController();
  late DateTime _unlockAt = DateTime.now().add(const Duration(days: 365));
  bool _replaceMessage = false;
  final List<UploadFile> _pending = [];
  Capsule? _existing;
  bool _loading = false;
  Object? _loadError;
  bool _busy = false;

  bool get _isEdit => widget.capsuleId != null;

  @override
  void initState() {
    super.initState();
    _unlockAt = DateTime(_unlockAt.year, _unlockAt.month, _unlockAt.day, 9);
    if (_isEdit) _loadExisting();
  }

  @override
  void dispose() {
    _title.dispose();
    _message.dispose();
    super.dispose();
  }

  Future<void> _loadExisting() async {
    setState(() {
      _loading = true;
      _loadError = null;
    });
    try {
      final c = await context.read<ApiClient>().getCapsule(widget.capsuleId!);
      if (!mounted) return;
      setState(() {
        _existing = c;
        _title.text = c.title;
        _unlockAt = c.unlockAt.toLocal();
      });
    } catch (e) {
      if (mounted) setState(() => _loadError = e);
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _pickUnlock() async {
    final now = DateTime.now();
    final date = await showDatePicker(
      context: context,
      initialDate: _unlockAt.isBefore(now) ? now : _unlockAt,
      firstDate: DateTime(now.year, now.month, now.day),
      lastDate: DateTime(now.year + 100, now.month, now.day),
    );
    if (date == null || !mounted) return;
    final time = await showTimePicker(context: context, initialTime: TimeOfDay.fromDateTime(_unlockAt));
    if (time == null) return;
    setState(() => _unlockAt = DateTime(date.year, date.month, date.day, time.hour, time.minute));
  }

  String? _unlockError() {
    final now = DateTime.now();
    if (!_unlockAt.isAfter(now)) return 'The unlock time must be in the future';
    if (_unlockAt.isAfter(DateTime(now.year + 100, now.month, now.day))) return 'At most 100 years ahead';
    return null;
  }

  Future<void> _save() async {
    if (!_form.currentState!.validate()) return;
    final unlockError = _unlockError();
    if (unlockError != null) {
      showSnack(context, unlockError);
      return;
    }
    setState(() => _busy = true);
    final api = context.read<ApiClient>();
    final message = _message.text.trim();
    try {
      final Capsule saved;
      if (_isEdit) {
        saved = await api.updateCapsule(
          widget.capsuleId!,
          title: _title.text.trim(),
          unlockAt: _unlockAt,
          replaceMessage: _replaceMessage,
          message: message.isEmpty ? null : message,
        );
      } else {
        saved = await api.createCapsule(
          widget.relationshipId!,
          title: _title.text.trim(),
          message: message.isEmpty ? null : message,
          unlockAt: _unlockAt,
        );
      }
      if (_pending.isNotEmpty) {
        try {
          await api.uploadCapsuleMedia(saved.id, _pending);
        } catch (e) {
          if (mounted) showSnack(context, 'Capsule saved, but uploading failed: ${errorMessage(e)}');
        }
      }
      if (mounted) context.pop(saved);
    } catch (e) {
      if (mounted) showError(context, e);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final title = _isEdit ? 'Edit capsule' : 'New time capsule';
    if (_loading) return Scaffold(appBar: AppBar(title: Text(title)), body: const Center(child: CircularProgressIndicator()));
    if (_loadError != null) {
      return Scaffold(appBar: AppBar(title: Text(title)), body: ErrorView(error: _loadError!, onRetry: _loadExisting));
    }
    if (_existing != null && !_existing!.isLocked) {
      return Scaffold(
        appBar: AppBar(title: Text(title)),
        body: const EmptyState(icon: Icons.lock_open, title: 'Only locked capsules can be edited'),
      );
    }
    final showMessageField = !_isEdit || _replaceMessage;
    return Scaffold(
      appBar: AppBar(title: Text(title)),
      body: Form(
        key: _form,
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            TextFormField(
              controller: _title,
              decoration: const InputDecoration(labelText: 'Title'),
              textCapitalization: TextCapitalization.sentences,
              validator: requiredText('Title', max: 200),
            ),
            const SizedBox(height: 12),
            InkWell(
              onTap: _pickUnlock,
              child: InputDecorator(
                decoration: InputDecoration(
                  labelText: 'Unlocks at',
                  prefixIcon: const Icon(Icons.lock_clock),
                  errorText: _unlockError(),
                ),
                child: Text(formatDateTime(_unlockAt.toUtc())),
              ),
            ),
            const SizedBox(height: 12),
            if (_isEdit)
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text('Replace the sealed message'),
                subtitle: const Text('The current message can\'t be shown. Leave off to keep it; '
                    'an empty new message removes it.'),
                value: _replaceMessage,
                onChanged: (v) => setState(() => _replaceMessage = v),
              ),
            if (showMessageField)
              TextFormField(
                controller: _message,
                decoration: const InputDecoration(labelText: 'Message', alignLabelWithHint: true),
                minLines: 4,
                maxLines: 12,
                textCapitalization: TextCapitalization.sentences,
                validator: optionalText('Message', max: 20000),
              ),
            const SizedBox(height: 16),
            Text('Seal files in the capsule', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 4),
            Text('Files can only be added while the capsule is locked, and stay hidden until it is opened.',
                style: Theme.of(context).textTheme.bodySmall),
            const SizedBox(height: 8),
            PendingFilesList(files: _pending, onRemove: (f) => setState(() => _pending.remove(f))),
            AttachButtons(onPicked: (files) => setState(() => _pending.addAll(files))),
            const SizedBox(height: 24),
            BusyButton(busy: _busy, onPressed: _save, label: _isEdit ? 'Save' : 'Seal capsule'),
          ],
        ),
      ),
    );
  }
}
