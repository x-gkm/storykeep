import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../../api/api_client.dart';
import '../../state/auth_state.dart';
import '../../util/format.dart';
import '../../util/permissions.dart';
import '../../util/validators.dart';
import '../widgets/common.dart';

IconData domainIcon(String domain) => switch (domain) {
      'PHYSICAL' => Icons.fitness_center,
      'MOTOR' => Icons.directions_walk,
      'LANGUAGE' => Icons.record_voice_over_outlined,
      'COGNITIVE' => Icons.psychology_outlined,
      _ => Icons.diversity_1_outlined,
    };

/// A child's development records, newest first, filterable by domain.
class DevelopmentSection extends StatefulWidget {
  const DevelopmentSection({super.key, required this.profile});

  final Profile profile;

  @override
  State<DevelopmentSection> createState() => _DevelopmentSectionState();
}

class _DevelopmentSectionState extends State<DevelopmentSection> with AutomaticKeepAliveClientMixin {
  List<DevelopmentRecord>? _records;
  Object? _error;
  String? _domain;

  @override
  bool get wantKeepAlive => true;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final records = await context.read<ApiClient>().listDevelopmentRecords(widget.profile.id, domain: _domain);
      if (mounted) {
        setState(() {
          _records = records;
          _error = null;
        });
      }
    } catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  Future<void> _delete(DevelopmentRecord r) async {
    final ok = await confirm(context, title: 'Delete record?', message: 'The record from ${formatDate(r.recordDate)} and its observations will be removed.');
    if (!ok || !mounted) return;
    try {
      await context.read<ApiClient>().deleteDevelopmentRecord(r.id);
      await _load();
    } catch (e) {
      if (mounted) showError(context, e);
    }
  }

  Future<void> _push(String path) async {
    final saved = await context.push<DevelopmentRecord>(path);
    if (saved != null && mounted) await _load();
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    final role = widget.profile.role;
    final me = context.select<AuthState, int?>((a) => a.user?.id);
    final records = _records;
    return Scaffold(
      floatingActionButton: Perms.canWrite(role)
          ? FloatingActionButton.extended(
              onPressed: () => _push('/profiles/${widget.profile.id}/development/new'),
              icon: const Icon(Icons.add),
              label: const Text('Record'),
            )
          : null,
      body: Column(
        children: [
          SizedBox(
            height: 56,
            child: ListView(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              children: [
                for (final d in [null, ...Lookups.developmentDomains])
                  Padding(
                    padding: const EdgeInsets.only(right: 8),
                    child: ChoiceChip(
                      label: Text(d == null ? 'All domains' : labelOf(d)),
                      selected: _domain == d,
                      onSelected: (_) {
                        setState(() {
                          _domain = d;
                          _records = null;
                        });
                        _load();
                      },
                    ),
                  ),
              ],
            ),
          ),
          Expanded(
            child: records == null
                ? (_error != null ? ErrorView(error: _error!, onRetry: _load) : const Center(child: CircularProgressIndicator()))
                : RefreshIndicator(
                    onRefresh: _load,
                    child: records.isEmpty
                        ? ListView(children: const [
                            SizedBox(height: 48),
                            EmptyState(
                              icon: Icons.child_friendly_outlined,
                              title: 'No development records',
                              message: 'Note what you observe — words, movements, play — by date.',
                            ),
                          ])
                        : ListView.separated(
                            padding: const EdgeInsets.fromLTRB(16, 0, 16, 96),
                            itemCount: records.length,
                            separatorBuilder: (_, _) => const SizedBox(height: 8),
                            itemBuilder: (context, i) {
                              final r = records[i];
                              final canEdit = Perms.canEditItem(role, createdBy: r.createdBy.id, currentUserId: me);
                              return _RecordCard(
                                record: r,
                                onEdit: canEdit ? () => _push('/development-records/${r.id}/edit') : null,
                                onDelete: canEdit ? () => _delete(r) : null,
                              );
                            },
                          ),
                  ),
          ),
        ],
      ),
    );
  }
}

class _RecordCard extends StatelessWidget {
  const _RecordCard({required this.record, this.onEdit, this.onDelete});

  final DevelopmentRecord record;
  final VoidCallback? onEdit;
  final VoidCallback? onDelete;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final r = record;
    return Card(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 8, 8, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(child: Text(formatDate(r.recordDate), style: theme.textTheme.titleMedium)),
                if (onEdit != null || onDelete != null)
                  PopupMenuButton<String>(
                    onSelected: (v) => v == 'edit' ? onEdit?.call() : onDelete?.call(),
                    itemBuilder: (_) => [
                      if (onEdit != null) const PopupMenuItem(value: 'edit', child: Text('Edit')),
                      if (onDelete != null) const PopupMenuItem(value: 'delete', child: Text('Delete')),
                    ],
                  ),
              ],
            ),
            if (r.notes != null) ...[
              Text(r.notes!),
              const SizedBox(height: 8),
            ],
            for (final o in r.observations)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Pill(labelOf(o.domain), icon: domainIcon(o.domain)),
                    const SizedBox(width: 8),
                    Expanded(child: Text(o.observation)),
                  ],
                ),
              ),
            const SizedBox(height: 4),
            Text('Recorded by ${r.createdBy.fullName}',
                style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.outline)),
          ],
        ),
      ),
    );
  }
}

class _ObservationDraft {
  _ObservationDraft(this.domain, String text) : controller = TextEditingController(text: text);

  String domain;
  final TextEditingController controller;
}

/// Creates a record for [profileId] or edits [recordId].
class DevelopmentRecordFormScreen extends StatefulWidget {
  const DevelopmentRecordFormScreen({super.key, this.profileId, this.recordId})
      : assert((profileId == null) != (recordId == null));

  final int? profileId;
  final int? recordId;

  @override
  State<DevelopmentRecordFormScreen> createState() => _DevelopmentRecordFormScreenState();
}

class _DevelopmentRecordFormScreenState extends State<DevelopmentRecordFormScreen> {
  final _form = GlobalKey<FormState>();
  final _notes = TextEditingController();
  final List<_ObservationDraft> _observations = [];
  DateTime _date = today();
  Profile? _profile;
  Object? _loadError;
  bool _busy = false;

  bool get _isEdit => widget.recordId != null;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() => _loadError = null);
    final api = context.read<ApiClient>();
    try {
      if (_isEdit) {
        final r = await api.getDevelopmentRecord(widget.recordId!);
        final p = await api.getProfile(r.profileId);
        if (!mounted) return;
        setState(() {
          _profile = p;
          _date = r.recordDate;
          _notes.text = r.notes ?? '';
          _observations
            ..clear()
            ..addAll(r.observations.map((o) => _ObservationDraft(o.domain, o.observation)));
        });
      } else {
        final p = await api.getProfile(widget.profileId!);
        if (!mounted) return;
        setState(() {
          _profile = p;
          if (_observations.isEmpty) _observations.add(_ObservationDraft('MOTOR', ''));
        });
      }
    } catch (e) {
      if (mounted) setState(() => _loadError = e);
    }
  }

  @override
  void dispose() {
    _notes.dispose();
    for (final o in _observations) {
      o.controller.dispose();
    }
    super.dispose();
  }

  Future<void> _save() async {
    if (!_form.currentState!.validate()) return;
    final observations = [
      for (final o in _observations)
        if (o.controller.text.trim().isNotEmpty) Observation(domain: o.domain, observation: o.controller.text.trim()),
    ];
    final notes = _notes.text.trim();
    if (notes.isEmpty && observations.isEmpty) {
      showSnack(context, 'Add notes or at least one observation');
      return;
    }
    setState(() => _busy = true);
    final api = context.read<ApiClient>();
    final input = DevelopmentRecordInput(recordDate: _date, notes: notes.isEmpty ? null : notes, observations: observations);
    try {
      final saved = _isEdit
          ? await api.updateDevelopmentRecord(widget.recordId!, input)
          : await api.createDevelopmentRecord(widget.profileId!, input);
      if (mounted) context.pop(saved);
    } catch (e) {
      if (mounted) showError(context, e);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final title = _isEdit ? 'Edit development record' : 'New development record';
    final profile = _profile;
    if (profile == null) {
      return Scaffold(
        appBar: AppBar(title: Text(title)),
        body: _loadError != null ? ErrorView(error: _loadError!, onRetry: _load) : const Center(child: CircularProgressIndicator()),
      );
    }
    return Scaffold(
      appBar: AppBar(title: Text(title)),
      body: Form(
        key: _form,
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            DateFormField(
              label: 'Date',
              initialValue: _date,
              firstDate: profile.dateOfBirth,
              lastDate: today(),
              onChanged: (d) => _date = d!,
              validator: (d) {
                if (d == null) return 'Date is required';
                if (d.isAfter(today())) return 'Can\'t be in the future';
                if (profile.dateOfBirth != null && d.isBefore(profile.dateOfBirth!)) return 'Can\'t be before birth';
                return null;
              },
            ),
            const SizedBox(height: 12),
            TextFormField(
              controller: _notes,
              decoration: const InputDecoration(labelText: 'Notes (optional)', alignLabelWithHint: true),
              minLines: 2,
              maxLines: 8,
              validator: optionalText('Notes', max: 10000),
            ),
            SectionTitle(
              'Observations',
              trailing: TextButton.icon(
                onPressed: _observations.length >= 50
                    ? null
                    : () => setState(() => _observations.add(_ObservationDraft('LANGUAGE', ''))),
                icon: const Icon(Icons.add),
                label: const Text('Add'),
              ),
            ),
            for (final o in _observations)
              Card(
                key: ObjectKey(o),
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: Column(
                    children: [
                      Row(
                        children: [
                          Expanded(
                            child: DropdownButtonFormField<String>(
                              initialValue: o.domain,
                              decoration: const InputDecoration(labelText: 'Domain', isDense: true),
                              items: [
                                for (final d in Lookups.developmentDomains)
                                  DropdownMenuItem(value: d, child: Text(labelOf(d))),
                              ],
                              onChanged: (v) => o.domain = v!,
                            ),
                          ),
                          IconButton(
                            tooltip: 'Remove',
                            icon: const Icon(Icons.delete_outline),
                            onPressed: () {
                              setState(() => _observations.remove(o));
                              WidgetsBinding.instance.addPostFrameCallback((_) => o.controller.dispose());
                            },
                          ),
                        ],
                      ),
                      const SizedBox(height: 8),
                      TextFormField(
                        controller: o.controller,
                        decoration: const InputDecoration(labelText: 'What did you observe?'),
                        minLines: 1,
                        maxLines: 5,
                        validator: optionalText('Observation', max: 5000),
                      ),
                    ],
                  ),
                ),
              ),
            const SizedBox(height: 24),
            BusyButton(busy: _busy, onPressed: _save, label: 'Save'),
          ],
        ),
      ),
    );
  }
}
