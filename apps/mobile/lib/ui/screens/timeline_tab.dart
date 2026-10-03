import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../../api/api_client.dart';
import '../../util/format.dart';
import '../../util/permissions.dart';
import '../widgets/common.dart';
import '../widgets/media.dart';

/// A relationship's memories ordered by event date, with filters and paging.
class TimelineTab extends StatefulWidget {
  const TimelineTab({super.key, required this.relationship, this.pageSize = 20});

  final Relationship relationship;
  final int pageSize;

  @override
  State<TimelineTab> createState() => _TimelineTabState();
}

class _TimelineTabState extends State<TimelineTab> with AutomaticKeepAliveClientMixin {
  final _scroll = ScrollController();
  final _search = TextEditingController();
  TimelineFilter _filter = const TimelineFilter();
  final List<Memory> _memories = [];
  int _total = 0;
  bool _loading = false;
  Object? _error;
  int _generation = 0;

  ApiClient get _api => context.read<ApiClient>();

  @override
  bool get wantKeepAlive => true;

  @override
  void initState() {
    super.initState();
    _scroll.addListener(() {
      if (_scroll.position.extentAfter < 400) _loadMore();
    });
    _refresh();
  }

  @override
  void dispose() {
    _scroll.dispose();
    _search.dispose();
    super.dispose();
  }

  bool get _hasMore => _memories.length < _total;

  Future<void> _refresh() async {
    _generation++;
    setState(() {
      _memories.clear();
      _total = 0;
      _error = null;
      _loading = false;
    });
    await _loadMore(force: true);
  }

  Future<void> _loadMore({bool force = false}) async {
    if (_loading || (!force && !_hasMore) || _error != null) return;
    final generation = _generation;
    setState(() => _loading = true);
    try {
      final page = await _api.listMemories(widget.relationship.id,
          filter: _filter, limit: widget.pageSize, offset: _memories.length);
      if (!mounted || generation != _generation) return;
      setState(() {
        _memories.addAll(page.memories);
        _total = page.total;
      });
      // If the loaded pages don't fill the screen there is nothing to scroll; keep loading.
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted && _scroll.hasClients && _scroll.position.extentAfter < 400) _loadMore();
      });
    } catch (e) {
      if (mounted && generation == _generation) setState(() => _error = e);
    } finally {
      if (mounted && generation == _generation) setState(() => _loading = false);
    }
  }

  void _applyFilter(TimelineFilter f) {
    _filter = f;
    _refresh();
  }

  Future<void> _openFilters() async {
    final tags = await _api.listRelationshipTags(widget.relationship.id, limit: 200).catchError((_) => <Tag>[]);
    if (!mounted) return;
    final result = await showModalBottomSheet<TimelineFilter>(
      context: context,
      isScrollControlled: true,
      showDragHandle: true,
      builder: (_) => TimelineFilterSheet(initial: _filter, tags: tags.map((t) => t.name).toList()),
    );
    if (result != null) _applyFilter(result.copyWith(query: () => _filter.query));
  }

  Future<void> _openMemory(Memory m) async {
    await context.push('/memories/${m.id}');
    if (!mounted) return;
    // Re-read the memory: it may have been edited or deleted.
    try {
      final updated = await _api.getMemory(m.id);
      final i = _memories.indexWhere((x) => x.id == m.id);
      if (mounted && i >= 0) setState(() => _memories[i] = updated);
    } on ApiException catch (e) {
      if (e.isNotFound && mounted) {
        setState(() {
          _memories.removeWhere((x) => x.id == m.id);
          _total--;
        });
      }
    }
  }

  Future<void> _create() async {
    final created = await context.push<Memory>('/relationships/${widget.relationship.id}/memories/new');
    if (created == null || !mounted) return;
    _refresh();
    await _openMemory(created);
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    final canWrite = Perms.canWrite(widget.relationship.role);
    return Scaffold(
      floatingActionButton: canWrite
          ? FloatingActionButton.extended(
              key: const Key('add-memory'),
              onPressed: _create,
              icon: const Icon(Icons.add),
              label: const Text('Memory'),
            )
          : null,
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 12, 8, 4),
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _search,
                    textInputAction: TextInputAction.search,
                    maxLength: 200,
                    decoration: InputDecoration(
                      isDense: true,
                      counterText: '',
                      hintText: 'Search title and description',
                      prefixIcon: const Icon(Icons.search),
                      suffixIcon: _search.text.isEmpty
                          ? null
                          : IconButton(
                              icon: const Icon(Icons.clear),
                              onPressed: () {
                                _search.clear();
                                _applyFilter(_filter.copyWith(query: () => null));
                              },
                            ),
                    ),
                    onSubmitted: (q) => _applyFilter(_filter.copyWith(query: () => q.trim().isEmpty ? null : q)),
                  ),
                ),
                IconButton(
                  tooltip: 'Filters',
                  onPressed: _openFilters,
                  icon: Badge(
                    isLabelVisible: _activeFilterCount > 0,
                    label: Text('$_activeFilterCount'),
                    child: const Icon(Icons.tune),
                  ),
                ),
              ],
            ),
          ),
          if (_activeFilterCount > 0 || _filter.ascending) _activeFilterChips(),
          Expanded(child: RefreshIndicator(onRefresh: _refresh, child: _list())),
        ],
      ),
    );
  }

  int get _activeFilterCount =>
      (_filter.from != null || _filter.to != null ? 1 : 0) +
      (_filter.category != null ? 1 : 0) +
      (_filter.tag != null ? 1 : 0);

  Widget _activeFilterChips() {
    final f = _filter;
    return SizedBox(
      height: 48,
      child: ListView(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 16),
        children: [
          if (f.from != null || f.to != null)
            _chip(
              '${f.from == null ? '…' : formatDate(f.from!)} – ${f.to == null ? '…' : formatDate(f.to!)}',
              () => _applyFilter(f.copyWith(from: () => null, to: () => null)),
            ),
          if (f.category != null) _chip(labelOf(f.category!), () => _applyFilter(f.copyWith(category: () => null))),
          if (f.tag != null) _chip('#${f.tag}', () => _applyFilter(f.copyWith(tag: () => null))),
          if (f.ascending) _chip('Oldest first', () => _applyFilter(f.copyWith(ascending: false))),
        ],
      ),
    );
  }

  Widget _chip(String label, VoidCallback onDeleted) => Padding(
        padding: const EdgeInsets.only(right: 8),
        child: InputChip(label: Text(label), onDeleted: onDeleted),
      );

  Widget _list() {
    if (_memories.isEmpty) {
      if (_error != null) {
        return ListView(children: [ErrorView(error: _error!, onRetry: _refresh)]);
      }
      if (_loading) return const Center(child: CircularProgressIndicator());
      return ListView(children: [
        const SizedBox(height: 48),
        EmptyState(
          icon: Icons.photo_album_outlined,
          title: _filter.isEmpty ? 'No memories yet' : 'No memories match these filters',
          message: _filter.isEmpty && Perms.canWrite(widget.relationship.role)
              ? 'Add the first one with the button below.'
              : null,
        ),
      ]);
    }
    final children = <Widget>[];
    int? year;
    for (final m in _memories) {
      if (m.memoryDate.year != year) {
        year = m.memoryDate.year;
        children.add(Padding(
          padding: const EdgeInsets.fromLTRB(4, 12, 4, 6),
          child: Text('$year', style: Theme.of(context).textTheme.titleSmall),
        ));
      }
      children.add(Padding(
        padding: const EdgeInsets.only(bottom: 10),
        child: MemoryCard(memory: m, onTap: () => _openMemory(m)),
      ));
    }
    if (_hasMore || _loading) {
      children.add(const Padding(padding: EdgeInsets.all(16), child: Center(child: CircularProgressIndicator())));
    } else if (_error != null) {
      children.add(TextButton(
          onPressed: () {
            setState(() => _error = null);
            _loadMore();
          },
          child: Text('${errorMessage(_error!)} Tap to retry.')));
    } else {
      children.add(Padding(
        padding: const EdgeInsets.all(16),
        child: Center(child: Text('$_total ${_total == 1 ? 'memory' : 'memories'}')),
      ));
    }
    return ListView(
      controller: _scroll,
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 96),
      children: children,
    );
  }
}

/// One memory on the timeline. The event date is prominent; when it was recorded is secondary.
class MemoryCard extends StatelessWidget {
  const MemoryCard({super.key, required this.memory, this.onTap});

  final Memory memory;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final m = memory;
    final firstImage = m.media.where((x) => x.isImage).firstOrNull;
    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              SizedBox(
                width: 52,
                child: Column(
                  children: [
                    Text('${m.memoryDate.day}', style: theme.textTheme.headlineSmall),
                    Text(monthYear(m.memoryDate), style: theme.textTheme.labelSmall, textAlign: TextAlign.center),
                  ],
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(m.title, style: theme.textTheme.titleMedium),
                    const SizedBox(height: 4),
                    Pill(labelOf(m.category), icon: categoryIcon(m.category)),
                    if (m.description != null) ...[
                      const SizedBox(height: 6),
                      Text(m.description!, maxLines: 2, overflow: TextOverflow.ellipsis),
                    ],
                    if (m.tags.isNotEmpty) ...[
                      const SizedBox(height: 6),
                      Text(m.tags.map((t) => '#$t').join('  '),
                          style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.primary)),
                    ],
                    const SizedBox(height: 6),
                    Row(
                      children: [
                        Expanded(
                          child: Text(
                            'Recorded ${formatDate(m.createdAt.toLocal())} by ${m.createdBy.firstName}',
                            style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.outline),
                          ),
                        ),
                        if (m.media.isNotEmpty) ...[
                          Icon(Icons.attach_file, size: 14, color: theme.colorScheme.outline),
                          Text('${m.media.length}', style: theme.textTheme.bodySmall),
                        ],
                      ],
                    ),
                  ],
                ),
              ),
              if (firstImage != null) ...[
                const SizedBox(width: 12),
                ClipRRect(
                  borderRadius: BorderRadius.circular(8),
                  child: SizedBox(width: 64, height: 64, child: AuthImage(firstImage.contentUrl)),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

/// Bottom sheet editing the date range, category, tag and order of the timeline.
class TimelineFilterSheet extends StatefulWidget {
  const TimelineFilterSheet({super.key, required this.initial, required this.tags});

  final TimelineFilter initial;
  final List<String> tags;

  @override
  State<TimelineFilterSheet> createState() => _TimelineFilterSheetState();
}

class _TimelineFilterSheetState extends State<TimelineFilterSheet> {
  late TimelineFilter _f = widget.initial;

  Future<void> _pickRange() async {
    final range = await showDateRangePicker(
      context: context,
      firstDate: DateTime(1900),
      lastDate: DateTime(2200),
      initialDateRange: _f.from != null && _f.to != null ? DateTimeRange(start: _f.from!, end: _f.to!) : null,
    );
    if (range != null) setState(() => _f = _f.copyWith(from: () => range.start, to: () => range.end));
  }

  @override
  Widget build(BuildContext context) {
    final tags = {...widget.tags, ?_f.tag}.toList();
    return Padding(
      padding: EdgeInsets.fromLTRB(16, 0, 16, 16 + MediaQuery.viewInsetsOf(context).bottom),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text('Filter timeline', style: Theme.of(context).textTheme.titleLarge),
          const SizedBox(height: 16),
          OutlinedButton.icon(
            onPressed: _pickRange,
            icon: const Icon(Icons.date_range),
            label: Text(_f.from == null && _f.to == null
                ? 'Any date'
                : '${_f.from == null ? '…' : formatDate(_f.from!)} – ${_f.to == null ? '…' : formatDate(_f.to!)}'),
          ),
          if (_f.from != null || _f.to != null)
            TextButton(
              onPressed: () => setState(() => _f = _f.copyWith(from: () => null, to: () => null)),
              child: const Text('Clear dates'),
            ),
          const SizedBox(height: 12),
          DropdownButtonFormField<String?>(
            key: ValueKey('category-${_f.category}'),
            initialValue: _f.category,
            decoration: const InputDecoration(labelText: 'Category'),
            items: [
              const DropdownMenuItem<String?>(value: null, child: Text('Any category')),
              for (final c in Lookups.memoryCategories) DropdownMenuItem(value: c, child: Text(labelOf(c))),
            ],
            onChanged: (v) => setState(() => _f = _f.copyWith(category: () => v)),
          ),
          const SizedBox(height: 12),
          DropdownButtonFormField<String?>(
            key: ValueKey('tag-${_f.tag}'),
            initialValue: _f.tag,
            decoration: const InputDecoration(labelText: 'Tag'),
            items: [
              const DropdownMenuItem<String?>(value: null, child: Text('Any tag')),
              for (final t in tags) DropdownMenuItem(value: t, child: Text(t)),
            ],
            onChanged: (v) => setState(() => _f = _f.copyWith(tag: () => v)),
          ),
          const SizedBox(height: 12),
          SegmentedButton<bool>(
            segments: const [
              ButtonSegment(value: false, label: Text('Newest first')),
              ButtonSegment(value: true, label: Text('Oldest first')),
            ],
            selected: {_f.ascending},
            onSelectionChanged: (s) => setState(() => _f = _f.copyWith(ascending: s.first)),
          ),
          const SizedBox(height: 20),
          Row(
            children: [
              TextButton(
                onPressed: () => setState(() => _f = TimelineFilter(query: _f.query)),
                child: const Text('Reset'),
              ),
              const Spacer(),
              FilledButton(onPressed: () => Navigator.pop(context, _f), child: const Text('Apply')),
            ],
          ),
        ],
      ),
    );
  }
}
