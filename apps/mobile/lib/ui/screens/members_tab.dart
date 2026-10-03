import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../../api/api_client.dart';
import '../../state/auth_state.dart';
import '../../util/format.dart';
import '../../util/permissions.dart';
import '../../util/validators.dart';
import '../widgets/common.dart';

/// Lists a relationship's members; managers can add, re-role and remove them,
/// and everyone can leave.
class MembersTab extends StatefulWidget {
  const MembersTab({super.key, required this.relationship, required this.onRelationshipChanged});

  final Relationship relationship;
  final Future<void> Function() onRelationshipChanged;

  @override
  State<MembersTab> createState() => _MembersTabState();
}

class _MembersTabState extends State<MembersTab> with AutomaticKeepAliveClientMixin {
  List<Member>? _members;
  Object? _error;

  ApiClient get _api => context.read<ApiClient>();
  String get _role => widget.relationship.role;
  int get _relId => widget.relationship.id;

  @override
  bool get wantKeepAlive => true;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final members = await _api.listMembers(_relId);
      if (mounted) {
        setState(() {
          _members = members;
          _error = null;
        });
      }
    } catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  Future<void> _run(Future<List<Member>?> Function() action, {bool roleMayChange = false}) async {
    try {
      final members = await action();
      if (!mounted) return;
      if (members != null) {
        setState(() => _members = members);
      } else {
        await _load();
      }
      if (roleMayChange) await widget.onRelationshipChanged();
    } catch (e) {
      if (mounted) showError(context, e);
    }
  }

  Future<void> _add() async {
    final result = await showDialog<(String, String)>(
      context: context,
      builder: (_) => _AddMemberDialog(roles: Perms.assignableRoles(_role)),
    );
    if (result == null) return;
    await _run(() => _api.addMember(_relId, email: result.$1, role: result.$2));
  }

  Future<void> _changeRole(Member m) async {
    final role = await showDialog<String>(
      context: context,
      builder: (context) => SimpleDialog(
        title: Text('Role of ${m.firstName}'),
        children: [
          for (final r in Perms.assignableRoles(_role))
            ListTile(
              leading: Icon(roleIcon(r)),
              title: Text(labelOf(r)),
              trailing: r == m.role ? const Icon(Icons.check) : null,
              onTap: () => Navigator.pop(context, r),
            ),
        ],
      ),
    );
    if (role == null || role == m.role || !mounted) return;
    final me = context.read<AuthState>().user?.id;
    await _run(() => _api.updateMemberRole(_relId, m.userId, role), roleMayChange: m.userId == me);
  }

  Future<void> _remove(Member m, {required bool self}) async {
    final ok = await confirm(
      context,
      title: self ? 'Leave this relationship?' : 'Remove ${m.fullName}?',
      message: self
          ? 'You will lose access to its timeline and capsules unless someone adds you again.'
          : 'They will lose access to this relationship.',
      action: self ? 'Leave' : 'Remove',
    );
    if (!ok || !mounted) return;
    try {
      await _api.removeMember(_relId, m.userId);
      if (!mounted) return;
      if (self) {
        context.go('/');
      } else {
        await _load();
      }
    } catch (e) {
      if (mounted) showError(context, e);
    }
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    final me = context.select<AuthState, int?>((a) => a.user?.id);
    final members = _members;
    return Scaffold(
      floatingActionButton: Perms.canManage(_role)
          ? FloatingActionButton.extended(
              key: const Key('add-member'),
              onPressed: _add,
              icon: const Icon(Icons.person_add_alt),
              label: const Text('Add member'),
            )
          : null,
      body: members == null
          ? (_error != null ? ErrorView(error: _error!, onRetry: _load) : const Center(child: CircularProgressIndicator()))
          : RefreshIndicator(
              onRefresh: _load,
              child: ListView(
                padding: const EdgeInsets.only(bottom: 96),
                children: [
                  for (final m in members) _tile(m, isMe: m.userId == me),
                ],
              ),
            ),
    );
  }

  Widget _tile(Member m, {required bool isMe}) {
    final canManageThis = !isMe && Perms.canManageMember(_role, m.role);
    final canChangeOwnRole = isMe && Perms.canManage(_role);
    return ListTile(
      leading: CircleAvatar(child: Text(m.firstName.isEmpty ? '?' : m.firstName[0].toUpperCase())),
      title: Text(isMe ? '${m.fullName} (you)' : m.fullName),
      subtitle: Text('${m.email}\nJoined ${formatDate(m.joinedAt.toLocal())}'),
      isThreeLine: true,
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          RolePill(m.role),
          PopupMenuButton<String>(
            key: Key('member-menu-${m.userId}'),
            onSelected: (v) => switch (v) {
              'role' => _changeRole(m),
              'remove' => _remove(m, self: false),
              'leave' => _remove(m, self: true),
              _ => null,
            },
            itemBuilder: (_) => [
              if (canManageThis || canChangeOwnRole) const PopupMenuItem(value: 'role', child: Text('Change role')),
              if (canManageThis) const PopupMenuItem(value: 'remove', child: Text('Remove')),
              if (isMe) const PopupMenuItem(value: 'leave', child: Text('Leave relationship')),
            ],
            enabled: canManageThis || isMe,
          ),
        ],
      ),
    );
  }
}

class _AddMemberDialog extends StatefulWidget {
  const _AddMemberDialog({required this.roles});

  final List<String> roles;

  @override
  State<_AddMemberDialog> createState() => _AddMemberDialogState();
}

class _AddMemberDialogState extends State<_AddMemberDialog> {
  final _form = GlobalKey<FormState>();
  final _email = TextEditingController();
  String _role = 'MEMBER';

  @override
  void dispose() {
    _email.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Add member'),
      content: Form(
        key: _form,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text('They need a Storykeep account with this email.'),
            const SizedBox(height: 12),
            TextFormField(
              controller: _email,
              autofocus: true,
              keyboardType: TextInputType.emailAddress,
              decoration: const InputDecoration(labelText: 'Email'),
              validator: validateEmail,
            ),
            const SizedBox(height: 12),
            DropdownButtonFormField<String>(
              initialValue: _role,
              decoration: const InputDecoration(labelText: 'Role'),
              items: [for (final r in widget.roles) DropdownMenuItem(value: r, child: Text(labelOf(r)))],
              onChanged: (v) => setState(() => _role = v!),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
        FilledButton(
          onPressed: () {
            if (_form.currentState!.validate()) Navigator.pop(context, (_email.text.trim(), _role));
          },
          child: const Text('Add'),
        ),
      ],
    );
  }
}
