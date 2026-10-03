import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../api/api_client.dart';
import '../../state/auth_state.dart';
import '../../util/format.dart';
import '../../util/validators.dart';
import '../widgets/common.dart';

/// Edit your name and date of birth, change your password, sign out.
class AccountScreen extends StatelessWidget {
  const AccountScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final user = context.select<AuthState, User?>((a) => a.user);
    return Scaffold(
      appBar: AppBar(title: const Text('Account')),
      body: user == null
          ? const SizedBox.shrink()
          : ListView(
              padding: const EdgeInsets.all(16),
              children: [
                ListTile(
                  contentPadding: EdgeInsets.zero,
                  leading: const CircleAvatar(child: Icon(Icons.person_outline)),
                  title: Text(user.fullName),
                  subtitle: Text('${user.email}\nMember since ${formatDate(user.createdAt.toLocal())}'),
                  isThreeLine: true,
                ),
                const SectionTitle('Your details'),
                _ProfileForm(user: user),
                const SectionTitle('Change password'),
                const _PasswordForm(),
                const SizedBox(height: 32),
                OutlinedButton.icon(
                  key: const Key('logout'),
                  onPressed: () => context.read<AuthState>().logout(),
                  icon: const Icon(Icons.logout),
                  label: const Text('Sign out'),
                ),
              ],
            ),
    );
  }
}

class _ProfileForm extends StatefulWidget {
  const _ProfileForm({required this.user});

  final User user;

  @override
  State<_ProfileForm> createState() => _ProfileFormState();
}

class _ProfileFormState extends State<_ProfileForm> {
  final _form = GlobalKey<FormState>();
  late final _first = TextEditingController(text: widget.user.firstName);
  late final _last = TextEditingController(text: widget.user.lastName);
  late DateTime? _dob = widget.user.dateOfBirth;
  bool _busy = false;

  @override
  void dispose() {
    _first.dispose();
    _last.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (!_form.currentState!.validate()) return;
    setState(() => _busy = true);
    try {
      final user = await context
          .read<ApiClient>()
          .updateMe(firstName: _first.text.trim(), lastName: _last.text.trim(), dateOfBirth: _dob);
      if (!mounted) return;
      context.read<AuthState>().updateUser(user);
      showSnack(context, 'Saved');
    } catch (e) {
      if (mounted) showError(context, e);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Form(
      key: _form,
      child: Column(
        children: [
          TextFormField(
            controller: _first,
            decoration: const InputDecoration(labelText: 'First name'),
            validator: requiredText('First name', max: 100),
          ),
          const SizedBox(height: 12),
          TextFormField(
            controller: _last,
            decoration: const InputDecoration(labelText: 'Last name'),
            validator: requiredText('Last name', max: 100),
          ),
          const SizedBox(height: 12),
          DateFormField(
            label: 'Date of birth',
            initialValue: _dob,
            lastDate: today(),
            clearable: true,
            onChanged: (d) => _dob = d,
          ),
          const SizedBox(height: 12),
          BusyButton(busy: _busy, onPressed: _save, label: 'Save details'),
        ],
      ),
    );
  }
}

class _PasswordForm extends StatefulWidget {
  const _PasswordForm();

  @override
  State<_PasswordForm> createState() => _PasswordFormState();
}

class _PasswordFormState extends State<_PasswordForm> {
  final _form = GlobalKey<FormState>();
  final _current = TextEditingController();
  final _new = TextEditingController();
  final _confirm = TextEditingController();
  bool _busy = false;

  @override
  void dispose() {
    _current.dispose();
    _new.dispose();
    _confirm.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (!_form.currentState!.validate()) return;
    setState(() => _busy = true);
    try {
      await context.read<ApiClient>().changePassword(currentPassword: _current.text, newPassword: _new.text);
      if (!mounted) return;
      _form.currentState!.reset();
      _current.clear();
      _new.clear();
      _confirm.clear();
      showSnack(context, 'Password changed. Other devices were signed out.');
    } on ApiException catch (e) {
      if (mounted) {
        showSnack(context, e.code == 'invalid_credentials' ? 'Your current password is wrong.' : errorMessage(e));
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Form(
      key: _form,
      child: Column(
        children: [
          TextFormField(
            controller: _current,
            obscureText: true,
            decoration: const InputDecoration(labelText: 'Current password'),
            validator: (v) => (v == null || v.isEmpty) ? 'Required' : null,
          ),
          const SizedBox(height: 12),
          TextFormField(
            controller: _new,
            obscureText: true,
            decoration: const InputDecoration(labelText: 'New password', helperText: '8–128 characters'),
            validator: validatePassword,
          ),
          const SizedBox(height: 12),
          TextFormField(
            controller: _confirm,
            obscureText: true,
            decoration: const InputDecoration(labelText: 'Confirm new password'),
            validator: (v) => v != _new.text ? 'Passwords don\'t match' : null,
          ),
          const SizedBox(height: 12),
          BusyButton(busy: _busy, onPressed: _save, label: 'Change password'),
        ],
      ),
    );
  }
}
