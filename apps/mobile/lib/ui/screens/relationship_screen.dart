import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../../api/api_client.dart';
import '../../util/format.dart';
import '../../util/permissions.dart';
import '../widgets/common.dart';
import 'capsules_tab.dart';
import 'members_tab.dart';
import 'timeline_tab.dart';

/// A relationship: its timeline, time capsules and members.
class RelationshipScreen extends StatelessWidget {
  const RelationshipScreen({super.key, required this.relationshipId, this.initialTab = 0});

  final int relationshipId;
  final int initialTab;

  @override
  Widget build(BuildContext context) {
    final api = context.read<ApiClient>();
    return Loader<Relationship>(
      fullScreen: true,
      load: () => api.getRelationship(relationshipId),
      builder: (context, rel, reload) => DefaultTabController(
        length: 3,
        initialIndex: initialTab.clamp(0, 2),
        child: Scaffold(
          appBar: AppBar(
            title: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(rel.profileName),
                Text(labelOf(rel.relationshipType), style: Theme.of(context).textTheme.bodySmall),
              ],
            ),
            actions: [
              IconButton(
                tooltip: 'Profile',
                icon: Icon(profileTypeIcon(rel.profileType)),
                onPressed: () => context.push('/profiles/${rel.profileId}'),
              ),
              if (Perms.canManage(rel.role))
                PopupMenuButton<String>(
                  onSelected: (v) async {
                    if (v == 'edit') {
                      final updated = await showRelationshipDialog(context, existing: rel);
                      if (updated != null) await reload();
                    } else if (v == 'delete') {
                      await _delete(context, rel);
                    }
                  },
                  itemBuilder: (_) => [
                    const PopupMenuItem(value: 'edit', child: Text('Edit relationship')),
                    if (Perms.isOwner(rel.role)) const PopupMenuItem(value: 'delete', child: Text('Delete relationship')),
                  ],
                ),
            ],
            bottom: const TabBar(tabs: [
              Tab(icon: Icon(Icons.timeline), text: 'Timeline'),
              Tab(icon: Icon(Icons.hourglass_bottom), text: 'Capsules'),
              Tab(icon: Icon(Icons.group_outlined), text: 'Members'),
            ]),
          ),
          body: TabBarView(children: [
            TimelineTab(relationship: rel),
            CapsulesTab(relationship: rel),
            MembersTab(relationship: rel, onRelationshipChanged: reload),
          ]),
        ),
      ),
    );
  }

  Future<void> _delete(BuildContext context, Relationship rel) async {
    final ok = await confirm(
      context,
      title: 'Delete relationship?',
      message: 'This permanently deletes its memories, time capsules and memberships for everyone.',
    );
    if (!ok || !context.mounted) return;
    try {
      await context.read<ApiClient>().deleteRelationship(rel.id);
      if (context.mounted) context.go('/');
    } catch (e) {
      if (context.mounted) showError(context, e);
    }
  }
}

/// Edits a relationship, or (with [profileId]) adds a new one to that profile.
/// Returns the saved relationship.
Future<Relationship?> showRelationshipDialog(BuildContext context, {Relationship? existing, int? profileId}) {
  return showDialog<Relationship>(
    context: context,
    builder: (_) => _RelationshipDialog(existing: existing, profileId: profileId),
  );
}

class _RelationshipDialog extends StatefulWidget {
  const _RelationshipDialog({this.existing, this.profileId});

  final Relationship? existing;
  final int? profileId;

  @override
  State<_RelationshipDialog> createState() => _RelationshipDialogState();
}

class _RelationshipDialogState extends State<_RelationshipDialog> {
  final _form = GlobalKey<FormState>();
  late String _type = widget.existing?.relationshipType ?? 'FAMILY';
  late DateTime? _started = widget.existing?.startedAt;
  late DateTime? _ended = widget.existing?.endedAt;
  bool _busy = false;

  Future<void> _save() async {
    if (!_form.currentState!.validate()) return;
    setState(() => _busy = true);
    final api = context.read<ApiClient>();
    try {
      final saved = widget.existing == null
          ? await api.createRelationship(
              profileId: widget.profileId!, relationshipType: _type, startedAt: _started, endedAt: _ended)
          : await api.updateRelationship(widget.existing!.id,
              relationshipType: _type, startedAt: _started, endedAt: _ended);
      if (mounted) Navigator.pop(context, saved);
    } catch (e) {
      if (mounted) showError(context, e);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.existing == null ? 'Add relationship' : 'Edit relationship'),
      content: Form(
        key: _form,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              DropdownButtonFormField<String>(
                initialValue: _type,
                decoration: const InputDecoration(labelText: 'Type'),
                items: [for (final t in Lookups.relationshipTypes) DropdownMenuItem(value: t, child: Text(labelOf(t)))],
                onChanged: (v) => setState(() => _type = v!),
              ),
              const SizedBox(height: 12),
              DateFormField(
                label: 'Started on',
                initialValue: _started,
                clearable: true,
                onChanged: (d) => _started = d,
              ),
              const SizedBox(height: 12),
              DateFormField(
                label: 'Ended on',
                initialValue: _ended,
                clearable: true,
                onChanged: (d) => _ended = d,
                validator: (d) => d != null && _started != null && d.isBefore(_started!)
                    ? 'Can\'t be before the start date'
                    : null,
              ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
        FilledButton(onPressed: _busy ? null : _save, child: const Text('Save')),
      ],
    );
  }
}
