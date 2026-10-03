import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../api/api_client.dart';
import '../../state/auth_state.dart';
import '../../util/format.dart';
import '../../util/permissions.dart';
import '../../util/validators.dart';
import '../widgets/common.dart';
import '../widgets/line_chart.dart';

class _MeasurementData {
  const _MeasurementData(this.series, this.measurements);

  final MeasurementSeries series;
  final List<Measurement> measurements;
}

/// Measurements of a profile, one type at a time: a chart and the list of values.
class MeasurementsSection extends StatefulWidget {
  const MeasurementsSection({super.key, required this.profile});

  final Profile profile;

  @override
  State<MeasurementsSection> createState() => _MeasurementsSectionState();
}

class _MeasurementsSectionState extends State<MeasurementsSection> with AutomaticKeepAliveClientMixin {
  String _type = 'HEIGHT';
  _MeasurementData? _data;
  Object? _error;

  @override
  bool get wantKeepAlive => true;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final api = context.read<ApiClient>();
    final type = _type;
    try {
      final results = await Future.wait([
        api.measurementSeries(widget.profile.id, type),
        api.listMeasurements(widget.profile.id, type: type),
      ]);
      if (!mounted || type != _type) return;
      setState(() {
        _data = _MeasurementData(results[0] as MeasurementSeries, results[1] as List<Measurement>);
        _error = null;
      });
    } catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  Future<void> _edit([Measurement? existing]) async {
    final saved = await showDialog<Measurement>(
      context: context,
      builder: (_) => MeasurementDialog(profile: widget.profile, type: _type, existing: existing),
    );
    if (saved == null || !mounted) return;
    if (saved.measurementType != _type) setState(() => _type = saved.measurementType);
    await _load();
  }

  Future<void> _delete(Measurement m) async {
    final ok = await confirm(context,
        title: 'Delete measurement?', message: '${formatNumber(m.value)} ${m.unit} on ${formatDate(m.measurementDate)}');
    if (!ok || !mounted) return;
    try {
      await context.read<ApiClient>().deleteMeasurement(m.id);
      await _load();
    } catch (e) {
      if (mounted) showError(context, e);
    }
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    final role = widget.profile.role;
    final me = context.select<AuthState, int?>((a) => a.user?.id);
    final data = _data;
    final unit = Lookups.measurementUnits[_type]!;
    return Scaffold(
      floatingActionButton: Perms.canWrite(role)
          ? FloatingActionButton.extended(
              onPressed: () => _edit(),
              icon: const Icon(Icons.add),
              label: const Text('Measurement'),
            )
          : null,
      body: RefreshIndicator(
        onRefresh: _load,
        child: ListView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 96),
          children: [
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: SegmentedButton<String>(
                segments: [
                  for (final t in Lookups.measurementTypes) ButtonSegment(value: t, label: Text(labelOf(t))),
                ],
                selected: {_type},
                onSelectionChanged: (s) {
                  setState(() {
                    _type = s.first;
                    _data = null;
                    _error = null;
                  });
                  _load();
                },
              ),
            ),
            const SizedBox(height: 16),
            if (data == null)
              _error != null
                  ? ErrorView(error: _error!, onRetry: _load)
                  : const Padding(padding: EdgeInsets.all(48), child: Center(child: CircularProgressIndicator()))
            else ...[
              Card(
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(8, 16, 16, 8),
                  child: MeasurementChart(points: data.series.points, unit: data.series.unit),
                ),
              ),
              const SizedBox(height: 8),
              Text('Values are shown as entered; no percentiles or assessments are made.',
                  style: Theme.of(context).textTheme.bodySmall),
              SectionTitle('${labelOf(_type)} ($unit)'),
              if (data.measurements.isEmpty) const Text('No measurements yet.'),
              for (final m in data.measurements.reversed)
                ListTile(
                  contentPadding: EdgeInsets.zero,
                  title: Text('${formatNumber(m.value)} ${m.unit}'),
                  subtitle: Text('${formatDate(m.measurementDate)} · by ${m.createdBy.firstName}'),
                  trailing: Perms.canEditItem(role, createdBy: m.createdBy.id, currentUserId: me)
                      ? PopupMenuButton<String>(
                          onSelected: (v) => v == 'edit' ? _edit(m) : _delete(m),
                          itemBuilder: (_) => const [
                            PopupMenuItem(value: 'edit', child: Text('Edit')),
                            PopupMenuItem(value: 'delete', child: Text('Delete')),
                          ],
                        )
                      : null,
                ),
            ],
          ],
        ),
      ),
    );
  }
}

class MeasurementDialog extends StatefulWidget {
  const MeasurementDialog({super.key, required this.profile, required this.type, this.existing});

  final Profile profile;
  final String type;
  final Measurement? existing;

  @override
  State<MeasurementDialog> createState() => _MeasurementDialogState();
}

class _MeasurementDialogState extends State<MeasurementDialog> {
  final _form = GlobalKey<FormState>();
  late String _type = widget.existing?.measurementType ?? widget.type;
  late final _value = TextEditingController(text: widget.existing == null ? '' : formatNumber(widget.existing!.value));
  late DateTime _date = widget.existing?.measurementDate ?? today();
  bool _busy = false;

  @override
  void dispose() {
    _value.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (!_form.currentState!.validate()) return;
    setState(() => _busy = true);
    final api = context.read<ApiClient>();
    final input = MeasurementInput(measurementType: _type, value: parseDecimal(_value.text)!, measurementDate: _date);
    try {
      final saved = widget.existing == null
          ? await api.createMeasurement(widget.profile.id, input)
          : await api.updateMeasurement(widget.existing!.id, input);
      if (mounted) Navigator.pop(context, saved);
    } catch (e) {
      if (mounted) showError(context, e);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final dob = widget.profile.dateOfBirth;
    return AlertDialog(
      title: Text(widget.existing == null ? 'Add measurement' : 'Edit measurement'),
      content: Form(
        key: _form,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              DropdownButtonFormField<String>(
                initialValue: _type,
                decoration: const InputDecoration(labelText: 'Type'),
                items: [
                  for (final t in Lookups.measurementTypes)
                    DropdownMenuItem(value: t, child: Text('${labelOf(t)} (${Lookups.measurementUnits[t]})')),
                ],
                onChanged: (v) => setState(() => _type = v!),
              ),
              const SizedBox(height: 12),
              TextFormField(
                controller: _value,
                autofocus: widget.existing == null,
                keyboardType: const TextInputType.numberWithOptions(decimal: true),
                decoration: InputDecoration(labelText: 'Value', suffixText: Lookups.measurementUnits[_type]),
                validator: validateMeasurementValue,
              ),
              const SizedBox(height: 12),
              DateFormField(
                label: 'Date',
                initialValue: _date,
                firstDate: dob,
                lastDate: today(),
                onChanged: (d) => _date = d!,
                validator: (d) {
                  if (d == null) return 'Date is required';
                  if (d.isAfter(today())) return 'Can\'t be in the future';
                  if (dob != null && d.isBefore(dob)) return 'Can\'t be before the date of birth';
                  return null;
                },
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
