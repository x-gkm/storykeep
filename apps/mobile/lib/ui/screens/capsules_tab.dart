import 'dart:async';

import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../../api/api_client.dart';
import '../../util/format.dart';
import '../../util/permissions.dart';
import '../widgets/common.dart';

IconData capsuleStatusIcon(String status) => switch (status) {
      'LOCKED' => Icons.lock_outline,
      'AVAILABLE' => Icons.lock_open,
      'OPENED' => Icons.drafts_outlined,
      _ => Icons.block,
    };

/// A relationship's time capsules, soonest unlock first, with a status filter.
class CapsulesTab extends StatefulWidget {
  const CapsulesTab({super.key, required this.relationship});

  final Relationship relationship;

  @override
  State<CapsulesTab> createState() => _CapsulesTabState();
}

class _CapsulesTabState extends State<CapsulesTab> with AutomaticKeepAliveClientMixin {
  List<Capsule>? _capsules;
  Object? _error;
  String? _status;
  Timer? _ticker;

  @override
  bool get wantKeepAlive => true;

  @override
  void initState() {
    super.initState();
    _load();
    // Keep countdowns current.
    _ticker = Timer.periodic(const Duration(seconds: 30), (_) {
      if (mounted) setState(() {});
    });
  }

  @override
  void dispose() {
    _ticker?.cancel();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final list = await context.read<ApiClient>().listCapsules(widget.relationship.id, status: _status);
      if (mounted) {
        setState(() {
          _capsules = list;
          _error = null;
        });
      }
    } catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  Future<void> _open(String path) async {
    await context.push(path);
    if (mounted) await _load();
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    final capsules = _capsules;
    return Scaffold(
      floatingActionButton: Perms.canWrite(widget.relationship.role)
          ? FloatingActionButton.extended(
              key: const Key('add-capsule'),
              onPressed: () => _open('/relationships/${widget.relationship.id}/capsules/new'),
              icon: const Icon(Icons.add),
              label: const Text('Capsule'),
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
                for (final s in [null, ...Lookups.capsuleStatuses])
                  Padding(
                    padding: const EdgeInsets.only(right: 8),
                    child: ChoiceChip(
                      label: Text(s == null ? 'All' : labelOf(s)),
                      selected: _status == s,
                      onSelected: (_) {
                        setState(() {
                          _status = s;
                          _capsules = null;
                        });
                        _load();
                      },
                    ),
                  ),
              ],
            ),
          ),
          Expanded(
            child: capsules == null
                ? (_error != null ? ErrorView(error: _error!, onRetry: _load) : const Center(child: CircularProgressIndicator()))
                : RefreshIndicator(
                    onRefresh: _load,
                    child: capsules.isEmpty
                        ? ListView(children: const [
                            SizedBox(height: 48),
                            EmptyState(
                              icon: Icons.hourglass_empty,
                              title: 'No time capsules',
                              message: 'Seal a message and photos to be opened on a future date.',
                            ),
                          ])
                        : ListView.separated(
                            padding: const EdgeInsets.fromLTRB(16, 0, 16, 96),
                            itemCount: capsules.length,
                            separatorBuilder: (_, _) => const SizedBox(height: 8),
                            itemBuilder: (context, i) => CapsuleCard(
                              capsule: capsules[i],
                              onTap: () => _open('/capsules/${capsules[i].id}'),
                            ),
                          ),
                  ),
          ),
        ],
      ),
    );
  }
}

class CapsuleCard extends StatelessWidget {
  const CapsuleCard({super.key, required this.capsule, this.onTap});

  final Capsule capsule;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final c = capsule;
    final theme = Theme.of(context);
    final String detail;
    if (c.isLocked) {
      detail = 'Unlocks in ${formatCountdown(c.unlockAt.difference(DateTime.now()))} · ${formatDateTime(c.unlockAt)}';
    } else if (c.isAvailable) {
      detail = 'Ready to open since ${formatDateTime(c.unlockAt)}';
    } else if (c.isOpened) {
      detail = 'Opened · unlocked ${formatDateTime(c.unlockAt)}';
    } else {
      detail = 'Cancelled';
    }
    return Card(
      child: ListTile(
        onTap: onTap,
        leading: CircleAvatar(
          backgroundColor: c.isAvailable ? theme.colorScheme.tertiaryContainer : theme.colorScheme.surfaceContainerHighest,
          child: Icon(capsuleStatusIcon(c.status)),
        ),
        title: Text(c.title),
        subtitle: Text(detail),
        trailing: c.mediaCount > 0
            ? Row(mainAxisSize: MainAxisSize.min, children: [
                const Icon(Icons.attach_file, size: 16),
                Text('${c.mediaCount}'),
              ])
            : null,
      ),
    );
  }
}
