import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../../api/api_client.dart';
import '../../util/format.dart';
import '../../util/permissions.dart';
import '../../util/validators.dart';
import '../widgets/common.dart';
import 'development_screens.dart';
import 'measurements_section.dart';
import 'relationship_screen.dart';

class _ProfileData {
  const _ProfileData(this.profile, this.relationships);

  final Profile profile;
  final List<Relationship> relationships;
}

/// A profile's details and its relationships; child development and measurements.
class ProfileScreen extends StatelessWidget {
  const ProfileScreen({super.key, required this.profileId});

  final int profileId;

  @override
  Widget build(BuildContext context) {
    final api = context.read<ApiClient>();
    return Loader<_ProfileData>(
      fullScreen: true,
      title: 'Profile',
      load: () async {
        final results = await Future.wait([api.getProfile(profileId), api.listRelationships()]);
        final rels = (results[1] as List<Relationship>).where((r) => r.profileId == profileId).toList();
        return _ProfileData(results[0] as Profile, rels);
      },
      builder: (context, data, reload) {
        final p = data.profile;
        final tabs = [
          const Tab(text: 'About'),
          if (p.isChild) const Tab(text: 'Development'),
          const Tab(text: 'Measurements'),
        ];
        return DefaultTabController(
          length: tabs.length,
          child: Scaffold(
            appBar: AppBar(
              title: Text(p.name),
              actions: [
                if (Perms.canManage(p.role))
                  IconButton(
                    key: const Key('edit-profile'),
                    tooltip: 'Edit profile',
                    icon: const Icon(Icons.edit_outlined),
                    onPressed: () async {
                      final updated = await showDialog<Profile>(
                        context: context,
                        builder: (_) => _EditProfileDialog(profile: p),
                      );
                      if (updated != null) await reload();
                    },
                  ),
                if (Perms.isOwner(p.role))
                  PopupMenuButton<String>(
                    onSelected: (_) => _delete(context, p),
                    itemBuilder: (_) => const [PopupMenuItem(value: 'delete', child: Text('Delete profile'))],
                  ),
              ],
              bottom: TabBar(tabs: tabs),
            ),
            body: TabBarView(children: [
              _AboutTab(data: data, reload: reload),
              if (p.isChild) DevelopmentSection(profile: p),
              MeasurementsSection(profile: p),
            ]),
          ),
        );
      },
    );
  }

  Future<void> _delete(BuildContext context, Profile p) async {
    final ok = await confirm(
      context,
      title: 'Delete ${p.name}?',
      message: 'This deletes the profile and every relationship with it, including all memories, capsules, '
          'development records and measurements. You must be owner of all its relationships.',
    );
    if (!ok || !context.mounted) return;
    try {
      await context.read<ApiClient>().deleteProfile(p.id);
      if (context.mounted) context.go('/');
    } catch (e) {
      if (context.mounted) showError(context, e);
    }
  }
}

class _AboutTab extends StatelessWidget {
  const _AboutTab({required this.data, required this.reload});

  final _ProfileData data;
  final Future<void> Function() reload;

  @override
  Widget build(BuildContext context) {
    final p = data.profile;
    final theme = Theme.of(context);
    return RefreshIndicator(
      onRefresh: reload,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Row(
            children: [
              CircleAvatar(
                radius: 32,
                backgroundColor: theme.colorScheme.primaryContainer,
                child: Icon(profileTypeIcon(p.profileType), size: 32, color: theme.colorScheme.onPrimaryContainer),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(p.name, style: theme.textTheme.headlineSmall),
                    Text(labelOf(p.profileType)),
                    const SizedBox(height: 4),
                    RolePill(p.role),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          if (p.dateOfBirth != null)
            ListTile(
              contentPadding: EdgeInsets.zero,
              leading: const Icon(Icons.cake_outlined),
              title: Text('Born ${formatDate(p.dateOfBirth!)}'),
              subtitle: Text(formatAge(p.dateOfBirth!)),
            ),
          SectionTitle(
            'Relationships',
            trailing: Perms.canManage(p.role)
                ? TextButton.icon(
                    onPressed: () async {
                      final created = await showRelationshipDialog(context, profileId: p.id);
                      if (created != null) await reload();
                    },
                    icon: const Icon(Icons.add),
                    label: const Text('Add'),
                  )
                : null,
          ),
          for (final r in data.relationships)
            Card(
              child: ListTile(
                leading: const Icon(Icons.timeline),
                title: Text(labelOf(r.relationshipType)),
                subtitle: Text([
                  if (r.startedAt != null) 'Since ${formatDate(r.startedAt!)}',
                  if (r.endedAt != null) 'until ${formatDate(r.endedAt!)}',
                ].join(' ')),
                trailing: RolePill(r.role),
                onTap: () => context.push('/relationships/${r.id}'),
              ),
            ),
        ],
      ),
    );
  }
}

class _EditProfileDialog extends StatefulWidget {
  const _EditProfileDialog({required this.profile});

  final Profile profile;

  @override
  State<_EditProfileDialog> createState() => _EditProfileDialogState();
}

class _EditProfileDialogState extends State<_EditProfileDialog> {
  final _form = GlobalKey<FormState>();
  late final _name = TextEditingController(text: widget.profile.name);
  late String _type = widget.profile.profileType;
  late DateTime? _dob = widget.profile.dateOfBirth;
  bool _busy = false;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (!_form.currentState!.validate()) return;
    setState(() => _busy = true);
    try {
      final p = await context
          .read<ApiClient>()
          .updateProfile(widget.profile.id, profileType: _type, name: _name.text.trim(), dateOfBirth: _dob);
      if (mounted) Navigator.pop(context, p);
    } catch (e) {
      if (mounted) showError(context, e);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Edit profile'),
      content: Form(
        key: _form,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextFormField(
                controller: _name,
                decoration: const InputDecoration(labelText: 'Name'),
                validator: requiredText('Name', max: 100),
              ),
              const SizedBox(height: 12),
              DropdownButtonFormField<String>(
                initialValue: _type,
                decoration: const InputDecoration(labelText: 'Type'),
                items: [for (final t in Lookups.profileTypes) DropdownMenuItem(value: t, child: Text(labelOf(t)))],
                onChanged: (v) => setState(() => _type = v!),
              ),
              const SizedBox(height: 12),
              DateFormField(
                label: 'Date of birth',
                initialValue: _dob,
                lastDate: today(),
                clearable: true,
                onChanged: (d) => _dob = d,
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
