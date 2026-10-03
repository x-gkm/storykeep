import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../../api/api_client.dart';
import '../../state/auth_state.dart';
import '../../util/format.dart';
import '../widgets/common.dart';

class _DashboardData {
  const _DashboardData(this.profiles, this.relationships);

  final List<Profile> profiles;
  final List<Relationship> relationships;
}

/// "My relationships", grouped by the profile they are about.
class DashboardScreen extends StatelessWidget {
  const DashboardScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final api = context.read<ApiClient>();
    final user = context.select<AuthState, User?>((a) => a.user);
    return Loader<_DashboardData>(
      fullScreen: true,
      title: 'Storykeep',
      load: () async {
        final results = await Future.wait([api.listProfiles(), api.listRelationships()]);
        return _DashboardData(results[0] as List<Profile>, results[1] as List<Relationship>);
      },
      builder: (context, data, reload) => Scaffold(
        appBar: AppBar(
          title: Text(user == null ? 'Storykeep' : 'Hi, ${user.firstName}'),
          actions: [
            IconButton(
              tooltip: 'Account',
              icon: const Icon(Icons.account_circle_outlined),
              onPressed: () => context.push('/account'),
            ),
          ],
        ),
        floatingActionButton: FloatingActionButton.extended(
          onPressed: () async {
            await context.push('/profiles/new');
            await reload();
          },
          icon: const Icon(Icons.add),
          label: const Text('New profile'),
        ),
        body: RefreshIndicator(onRefresh: reload, child: _body(context, data, reload)),
      ),
    );
  }

  Widget _body(BuildContext context, _DashboardData data, Future<void> Function() reload) {
    if (data.profiles.isEmpty) {
      return ListView(
        children: const [
          SizedBox(height: 80),
          EmptyState(
            icon: Icons.family_restroom_outlined,
            title: 'No relationships yet',
            message: 'Create a profile for a child, pet or someone special to start keeping memories.',
          ),
        ],
      );
    }
    final byProfile = <int, List<Relationship>>{};
    for (final r in data.relationships) {
      byProfile.putIfAbsent(r.profileId, () => []).add(r);
    }
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 96),
      children: [
        Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: Text('My relationships', style: Theme.of(context).textTheme.titleMedium),
        ),
        for (final p in data.profiles) ...[
          _ProfileCard(profile: p, relationships: byProfile[p.id] ?? const [], onChanged: reload),
          const SizedBox(height: 12),
        ],
      ],
    );
  }
}

class _ProfileCard extends StatelessWidget {
  const _ProfileCard({required this.profile, required this.relationships, required this.onChanged});

  final Profile profile;
  final List<Relationship> relationships;
  final Future<void> Function() onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final dob = profile.dateOfBirth;
    return Card(
      clipBehavior: Clip.antiAlias,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          ListTile(
            leading: CircleAvatar(
              backgroundColor: theme.colorScheme.primaryContainer,
              child: Icon(profileTypeIcon(profile.profileType), color: theme.colorScheme.onPrimaryContainer),
            ),
            title: Text(profile.name, style: theme.textTheme.titleMedium),
            subtitle: Text([
              labelOf(profile.profileType),
              if (dob != null && formatAge(dob).isNotEmpty) formatAge(dob),
            ].join(' · ')),
            trailing: const Icon(Icons.chevron_right),
            onTap: () async {
              await context.push('/profiles/${profile.id}');
              await onChanged();
            },
          ),
          const Divider(height: 1),
          for (final r in relationships)
            ListTile(
              dense: true,
              leading: const Icon(Icons.timeline),
              title: Text(labelOf(r.relationshipType)),
              subtitle: r.startedAt == null ? null : Text('Since ${formatDate(r.startedAt!)}'),
              trailing: RolePill(r.role),
              onTap: () async {
                await context.push('/relationships/${r.id}');
                await onChanged();
              },
            ),
        ],
      ),
    );
  }
}
