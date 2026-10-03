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

class _MemoryData {
  const _MemoryData(this.memory, this.relationship);

  final Memory memory;
  final Relationship relationship;
}

Future<_MemoryData> _loadMemory(ApiClient api, int id) async {
  final memory = await api.getMemory(id);
  return _MemoryData(memory, await api.getRelationship(memory.relationshipId));
}

class MemoryDetailScreen extends StatelessWidget {
  const MemoryDetailScreen({super.key, required this.memoryId});

  final int memoryId;

  @override
  Widget build(BuildContext context) {
    final api = context.read<ApiClient>();
    final me = context.select<AuthState, int?>((a) => a.user?.id);
    return Loader<_MemoryData>(
      fullScreen: true,
      title: 'Memory',
      load: () => _loadMemory(api, memoryId),
      builder: (context, data, reload) {
        final m = data.memory;
        final role = data.relationship.role;
        final canEdit = Perms.canEditItem(role, createdBy: m.createdBy.id, currentUserId: me);
        return Scaffold(
          appBar: AppBar(
            title: Text(data.relationship.profileName),
            actions: [
              if (canEdit)
                IconButton(
                  key: const Key('edit-memory'),
                  tooltip: 'Edit',
                  icon: const Icon(Icons.edit_outlined),
                  onPressed: () async {
                    await context.push('/memories/${m.id}/edit');
                    await reload();
                  },
                ),
              if (canEdit)
                IconButton(
                  key: const Key('delete-memory'),
                  tooltip: 'Delete',
                  icon: const Icon(Icons.delete_outline),
                  onPressed: () => _delete(context, m),
                ),
            ],
          ),
          body: RefreshIndicator(
            onRefresh: reload,
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: [
                Text(m.title, style: Theme.of(context).textTheme.headlineSmall),
                const SizedBox(height: 8),
                Wrap(spacing: 8, runSpacing: 8, children: [
                  Pill(labelOf(m.category), icon: categoryIcon(m.category)),
                  for (final t in m.tags) Pill('#$t'),
                ]),
                const SizedBox(height: 16),
                _InfoRow(icon: Icons.event, label: 'Happened on', value: formatDate(m.memoryDate)),
                _InfoRow(
                  icon: Icons.edit_calendar_outlined,
                  label: 'Recorded',
                  value: '${formatDateTime(m.createdAt)} by ${m.createdBy.fullName}',
                ),
                if (m.updatedAt.difference(m.createdAt).inSeconds > 1)
                  _InfoRow(icon: Icons.update, label: 'Last edited', value: formatDateTime(m.updatedAt)),
                if (m.description != null) ...[
                  const SizedBox(height: 16),
                  SelectableText(m.description!, style: Theme.of(context).textTheme.bodyLarge),
                ],
                SectionTitle(
                  'Media (${m.media.length})',
                  trailing: Perms.canWrite(role)
                      ? TextButton.icon(
                          onPressed: () => _attach(context, m, reload),
                          icon: const Icon(Icons.add),
                          label: const Text('Add'),
                        )
                      : null,
                ),
                if (m.media.isEmpty)
                  const Text('No photos, videos or files attached.')
                else
                  MediaGrid(
                    items: m.media,
                    canRemove: (item) => Perms.canRemoveMedia(role,
                        uploadedBy: item.uploadedBy?.id, currentUserId: me, fallback: canEdit),
                    onRemove: (item) => _removeMedia(context, m, item, reload),
                  ),
              ],
            ),
          ),
        );
      },
    );
  }

  Future<void> _delete(BuildContext context, Memory m) async {
    final ok = await confirm(context, title: 'Delete memory?', message: '"${m.title}" will be removed for everyone.');
    if (!ok || !context.mounted) return;
    try {
      await context.read<ApiClient>().deleteMemory(m.id);
      if (context.mounted) context.pop();
    } catch (e) {
      if (context.mounted) showError(context, e);
    }
  }

  Future<void> _attach(BuildContext context, Memory m, Future<void> Function() reload) async {
    final files = await showModalBottomSheet<List<UploadFile>>(
      context: context,
      showDragHandle: true,
      builder: (context) => Padding(
        padding: const EdgeInsets.fromLTRB(16, 0, 16, 24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Attach to this memory', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 12),
            AttachButtons(onPicked: (files) => Navigator.pop(context, files)),
          ],
        ),
      ),
    );
    if (files == null || files.isEmpty || !context.mounted) return;
    showSnack(context, 'Uploading ${files.length} file${files.length == 1 ? '' : 's'}…');
    try {
      await context.read<ApiClient>().uploadMemoryMedia(m.id, files);
      if (context.mounted) showSnack(context, 'Uploaded');
      await reload();
    } catch (e) {
      if (context.mounted) showError(context, e);
    }
  }

  Future<void> _removeMedia(BuildContext context, Memory m, MediaItem item, Future<void> Function() reload) async {
    final ok = await confirm(context, title: 'Remove file?', message: '${item.fileName} will be detached.', action: 'Remove');
    if (!ok || !context.mounted) return;
    try {
      await context.read<ApiClient>().removeMemoryMedia(m.id, item.id);
      await reload();
    } catch (e) {
      if (context.mounted) showError(context, e);
    }
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({required this.icon, required this.label, required this.value});

  final IconData icon;
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: [
          Icon(icon, size: 18, color: theme.colorScheme.outline),
          const SizedBox(width: 8),
          Text('$label: ', style: theme.textTheme.bodyMedium?.copyWith(color: theme.colorScheme.outline)),
          Expanded(child: Text(value)),
        ],
      ),
    );
  }
}

/// Creates a memory in [relationshipId], or edits [memoryId]. New attachments
/// are uploaded after the memory is saved. Pops with the saved [Memory].
class MemoryFormScreen extends StatefulWidget {
  const MemoryFormScreen({super.key, this.relationshipId, this.memoryId})
      : assert((relationshipId == null) != (memoryId == null));

  final int? relationshipId;
  final int? memoryId;

  @override
  State<MemoryFormScreen> createState() => _MemoryFormScreenState();
}

class _MemoryFormScreenState extends State<MemoryFormScreen> {
  final _form = GlobalKey<FormState>();
  final _title = TextEditingController();
  final _description = TextEditingController();
  final _tagCtrl = TextEditingController();
  final _tagFocus = FocusNode();
  String _category = 'GENERAL';
  DateTime _date = today();
  final List<String> _tags = [];
  List<String> _knownTags = [];
  final List<UploadFile> _pending = [];
  Memory? _existing;
  bool _loading = false;
  Object? _loadError;
  bool _busy = false;

  bool get _isEdit => widget.memoryId != null;

  @override
  void initState() {
    super.initState();
    final api = context.read<ApiClient>();
    api.listTags(limit: 200).then((tags) {
      if (mounted) setState(() => _knownTags = tags.map((t) => t.name).toList());
    }, onError: (_) {});
    if (_isEdit) _loadExisting();
  }

  Future<void> _loadExisting() async {
    setState(() {
      _loading = true;
      _loadError = null;
    });
    try {
      final m = await context.read<ApiClient>().getMemory(widget.memoryId!);
      if (!mounted) return;
      setState(() {
        _existing = m;
        _title.text = m.title;
        _description.text = m.description ?? '';
        _category = m.category;
        _date = m.memoryDate;
        _tags
          ..clear()
          ..addAll(m.tags);
      });
    } catch (e) {
      if (mounted) setState(() => _loadError = e);
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  @override
  void dispose() {
    _title.dispose();
    _description.dispose();
    _tagCtrl.dispose();
    _tagFocus.dispose();
    super.dispose();
  }

  void _addTag(String raw) {
    final name = raw.trim();
    if (name.isEmpty) return;
    final error = validateTag(name);
    if (error != null) {
      showSnack(context, error);
      return;
    }
    if (_tags.length >= 20) {
      showSnack(context, 'At most 20 tags');
      return;
    }
    setState(() {
      if (!_tags.any((t) => t.toLowerCase() == name.toLowerCase())) _tags.add(name);
      _tagCtrl.clear();
    });
  }

  Future<void> _save() async {
    if (_tagCtrl.text.trim().isNotEmpty) _addTag(_tagCtrl.text);
    if (!_form.currentState!.validate()) return;
    setState(() => _busy = true);
    final api = context.read<ApiClient>();
    final description = _description.text.trim();
    final input = MemoryInput(
      category: _category,
      title: _title.text.trim(),
      description: description.isEmpty ? null : description,
      memoryDate: _date,
      tags: List.of(_tags),
    );
    try {
      var memory = _isEdit ? await api.updateMemory(widget.memoryId!, input) : await api.createMemory(widget.relationshipId!, input);
      if (_pending.isNotEmpty) {
        try {
          await api.uploadMemoryMedia(memory.id, _pending);
          memory = await api.getMemory(memory.id);
        } catch (e) {
          if (mounted) showSnack(context, 'Memory saved, but uploading failed: ${errorMessage(e)}');
        }
      }
      if (mounted) context.pop(memory);
    } catch (e) {
      if (mounted) showError(context, e);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final title = _isEdit ? 'Edit memory' : 'New memory';
    if (_loading) return Scaffold(appBar: AppBar(title: Text(title)), body: const Center(child: CircularProgressIndicator()));
    if (_loadError != null) {
      return Scaffold(appBar: AppBar(title: Text(title)), body: ErrorView(error: _loadError!, onRetry: _loadExisting));
    }
    return Scaffold(
      appBar: AppBar(title: Text(title)),
      body: Form(
        key: _form,
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            TextFormField(
              key: const Key('memory-title'),
              controller: _title,
              decoration: const InputDecoration(labelText: 'Title'),
              textCapitalization: TextCapitalization.sentences,
              validator: requiredText('Title', max: 200),
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: DropdownButtonFormField<String>(
                    initialValue: _category,
                    isExpanded: true,
                    decoration: const InputDecoration(labelText: 'Category'),
                    items: [
                      for (final c in Lookups.memoryCategories)
                        DropdownMenuItem(
                          value: c,
                          child: Row(children: [Icon(categoryIcon(c), size: 18), const SizedBox(width: 8), Text(labelOf(c))]),
                        ),
                    ],
                    onChanged: (v) => setState(() => _category = v!),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            DateFormField(
              label: 'When did it happen?',
              initialValue: _date,
              onChanged: (d) => _date = d!,
              validator: (d) => d == null ? 'Date is required' : null,
            ),
            const SizedBox(height: 12),
            TextFormField(
              controller: _description,
              decoration: const InputDecoration(labelText: 'Description (optional)', alignLabelWithHint: true),
              minLines: 3,
              maxLines: 10,
              textCapitalization: TextCapitalization.sentences,
              validator: optionalText('Description', max: 10000),
            ),
            const SizedBox(height: 16),
            _tagInput(),
            const SizedBox(height: 8),
            Text('Attachments', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 8),
            if (_existing != null && _existing!.media.isNotEmpty) ...[
              MediaGrid(items: _existing!.media),
              const SizedBox(height: 4),
              Text('Remove existing files from the memory\'s page.', style: Theme.of(context).textTheme.bodySmall),
              const SizedBox(height: 8),
            ],
            PendingFilesList(files: _pending, onRemove: (f) => setState(() => _pending.remove(f))),
            AttachButtons(onPicked: (files) => setState(() => _pending.addAll(files))),
            const SizedBox(height: 24),
            BusyButton(key: const Key('save-memory'), busy: _busy, onPressed: _save, label: 'Save'),
          ],
        ),
      ),
    );
  }

  Widget _tagInput() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        RawAutocomplete<String>(
          textEditingController: _tagCtrl,
          focusNode: _tagFocus,
          optionsBuilder: (value) {
            final q = value.text.trim().toLowerCase();
            if (q.isEmpty) return const Iterable<String>.empty();
            return _knownTags
                .where((t) => t.toLowerCase().contains(q) && !_tags.any((x) => x.toLowerCase() == t.toLowerCase()))
                .take(8);
          },
          onSelected: _addTag,
          fieldViewBuilder: (context, controller, focusNode, onSubmit) => TextField(
            controller: controller,
            focusNode: focusNode,
            decoration: InputDecoration(
              labelText: 'Tags',
              helperText: 'Type a tag and press enter',
              prefixIcon: const Icon(Icons.sell_outlined),
              suffixIcon: IconButton(icon: const Icon(Icons.add), onPressed: () => _addTag(controller.text)),
            ),
            onSubmitted: (v) {
              _addTag(v);
              focusNode.requestFocus();
            },
          ),
          optionsViewBuilder: (context, onSelected, options) => Align(
            alignment: Alignment.topLeft,
            child: Material(
              elevation: 4,
              borderRadius: BorderRadius.circular(8),
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxHeight: 240, maxWidth: 320),
                child: ListView(
                  padding: EdgeInsets.zero,
                  shrinkWrap: true,
                  children: [
                    for (final o in options) ListTile(dense: true, title: Text(o), onTap: () => onSelected(o)),
                  ],
                ),
              ),
            ),
          ),
        ),
        const SizedBox(height: 8),
        Wrap(
          spacing: 8,
          runSpacing: 4,
          children: [
            for (final t in _tags) InputChip(label: Text(t), onDeleted: () => setState(() => _tags.remove(t))),
          ],
        ),
      ],
    );
  }
}
