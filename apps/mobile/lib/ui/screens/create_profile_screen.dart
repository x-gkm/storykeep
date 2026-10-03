import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../../api/api_client.dart';
import '../../util/format.dart';
import '../../util/validators.dart';
import '../widgets/common.dart';

/// The relationship type that usually goes with a profile type.
String suggestedRelationshipType(String profileType) => switch (profileType) {
      'CHILD' => 'PARENT_CHILD',
      'PET' => 'OWNER_PET',
      'PERSON' => 'FRIEND',
      _ => 'OTHER',
    };

/// Creates a profile together with the user's first relationship to it.
class CreateProfileScreen extends StatefulWidget {
  const CreateProfileScreen({super.key});

  @override
  State<CreateProfileScreen> createState() => _CreateProfileScreenState();
}

class _CreateProfileScreenState extends State<CreateProfileScreen> {
  final _form = GlobalKey<FormState>();
  final _name = TextEditingController();
  String _profileType = 'CHILD';
  String _relationshipType = 'PARENT_CHILD';
  bool _relationshipTouched = false;
  DateTime? _dob;
  DateTime? _startedAt;
  bool _busy = false;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    if (!_form.currentState!.validate()) return;
    setState(() => _busy = true);
    try {
      final created = await context.read<ApiClient>().createProfile(
            profileType: _profileType,
            name: _name.text.trim(),
            dateOfBirth: _dob,
            relationshipType: _relationshipType,
            startedAt: _startedAt,
          );
      if (!mounted) return;
      showSnack(context, '${created.profile.name} was created');
      context.pushReplacement('/relationships/${created.relationship.id}');
    } catch (e) {
      if (mounted) showError(context, e);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Scaffold(
      appBar: AppBar(title: const Text('New profile')),
      body: Form(
        key: _form,
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            Text('Who is this about?', style: theme.textTheme.titleMedium),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                for (final t in Lookups.profileTypes)
                  ChoiceChip(
                    avatar: Icon(profileTypeIcon(t), size: 18),
                    label: Text(labelOf(t)),
                    selected: _profileType == t,
                    onSelected: (_) => setState(() {
                      _profileType = t;
                      if (!_relationshipTouched) _relationshipType = suggestedRelationshipType(t);
                    }),
                  ),
              ],
            ),
            const SizedBox(height: 16),
            TextFormField(
              controller: _name,
              decoration: const InputDecoration(labelText: 'Name'),
              textCapitalization: TextCapitalization.words,
              validator: requiredText('Name', max: 100),
            ),
            const SizedBox(height: 12),
            DateFormField(
              label: 'Date of birth (optional)',
              lastDate: today(),
              clearable: true,
              onChanged: (d) => _dob = d,
            ),
            const SizedBox(height: 24),
            Text('Your relationship', style: theme.textTheme.titleMedium),
            const SizedBox(height: 4),
            Text('You will be its owner and can invite others later.', style: theme.textTheme.bodySmall),
            const SizedBox(height: 12),
            DropdownButtonFormField<String>(
              initialValue: _relationshipType,
              key: ValueKey(_relationshipType),
              decoration: const InputDecoration(labelText: 'Relationship type'),
              items: [
                for (final t in Lookups.relationshipTypes) DropdownMenuItem(value: t, child: Text(labelOf(t))),
              ],
              onChanged: (v) => setState(() {
                _relationshipType = v!;
                _relationshipTouched = true;
              }),
            ),
            const SizedBox(height: 12),
            DateFormField(
              label: 'Started on (optional)',
              clearable: true,
              onChanged: (d) => _startedAt = d,
            ),
            const SizedBox(height: 24),
            BusyButton(busy: _busy, onPressed: _submit, label: 'Create'),
          ],
        ),
      ),
    );
  }
}
