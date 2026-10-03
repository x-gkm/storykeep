import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:image_picker/image_picker.dart';
import 'package:open_filex/open_filex.dart';
import 'package:path_provider/path_provider.dart';
import 'package:provider/provider.dart';

import '../../api/api_client.dart';
import '../../util/format.dart';
import 'common.dart';

IconData mediaTypeIcon(String type) => switch (type) {
  'IMAGE' => Icons.image_outlined,
  'VIDEO' => Icons.movie_outlined,
  'AUDIO' => Icons.audiotrack_outlined,
  _ => Icons.description_outlined,
};

/// An image from an authenticated `content_url`, loaded with the bearer token.
class AuthImage extends StatelessWidget {
  const AuthImage(this.contentUrl, {super.key, this.fit = BoxFit.cover});

  final String contentUrl;
  final BoxFit fit;

  @override
  Widget build(BuildContext context) {
    final api = context.read<ApiClient>();
    return Image.network(
      api.resolve(contentUrl).toString(),
      headers: api.authHeaders,
      fit: fit,
      loadingBuilder: (context, child, progress) => progress == null
          ? child
          : const Center(child: CircularProgressIndicator(strokeWidth: 2)),
      errorBuilder: (context, error, stack) => Center(
        child: Icon(
          Icons.broken_image_outlined,
          color: Theme.of(context).colorScheme.outline,
        ),
      ),
    );
  }
}

/// A square thumbnail for a media item: the image itself, or an icon and file name.
class MediaThumb extends StatelessWidget {
  const MediaThumb(this.item, {super.key, this.onTap, this.onRemove});

  final MediaItem item;
  final VoidCallback? onTap;
  final VoidCallback? onRemove;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return ClipRRect(
      borderRadius: BorderRadius.circular(12),
      child: Material(
        color: scheme.surfaceContainerHighest,
        child: InkWell(
          onTap: onTap,
          child: Stack(
            fit: StackFit.expand,
            children: [
              if (item.isImage)
                AuthImage(item.contentUrl)
              else
                Padding(
                  padding: const EdgeInsets.all(8),
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(
                        mediaTypeIcon(item.mediaType),
                        size: 32,
                        color: scheme.primary,
                      ),
                      const SizedBox(height: 6),
                      Text(
                        item.fileName,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        textAlign: TextAlign.center,
                        style: Theme.of(context).textTheme.labelSmall,
                      ),
                      Text(
                        formatBytes(item.fileSize),
                        style: Theme.of(context).textTheme.labelSmall,
                      ),
                    ],
                  ),
                ),
              if (onRemove != null)
                Positioned(
                  top: 2,
                  right: 2,
                  child: IconButton.filledTonal(
                    visualDensity: VisualDensity.compact,
                    tooltip: 'Remove',
                    icon: const Icon(Icons.close, size: 16),
                    onPressed: onRemove,
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

/// A grid of media; tapping opens images in a viewer and other files externally.
class MediaGrid extends StatelessWidget {
  const MediaGrid({
    super.key,
    required this.items,
    this.canRemove,
    this.onRemove,
  });

  final List<MediaItem> items;
  final bool Function(MediaItem)? canRemove;
  final void Function(MediaItem)? onRemove;

  @override
  Widget build(BuildContext context) {
    return GridView.builder(
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(
        maxCrossAxisExtent: 140,
        mainAxisSpacing: 8,
        crossAxisSpacing: 8,
      ),
      itemCount: items.length,
      itemBuilder: (context, i) {
        final item = items[i];
        final removable = onRemove != null && (canRemove?.call(item) ?? true);
        return MediaThumb(
          item,
          onTap: () => openMedia(context, items, i),
          onRemove: removable ? () => onRemove!(item) : null,
        );
      },
    );
  }
}

/// Opens [items][index]: images in an in-app viewer, other types downloaded to a
/// temporary file and handed to the platform's default app.
Future<void> openMedia(
  BuildContext context,
  List<MediaItem> items,
  int index,
) async {
  final item = items[index];
  if (item.isImage) {
    final images = items.where((m) => m.isImage).toList();
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) =>
            ImageViewer(images: images, initialIndex: images.indexOf(item)),
      ),
    );
    return;
  }
  final api = context.read<ApiClient>();
  final messenger = ScaffoldMessenger.of(context);
  messenger.showSnackBar(
    SnackBar(content: Text('Downloading ${item.fileName}…')),
  );
  try {
    final dir = await getTemporaryDirectory();
    final safeName = item.fileName.replaceAll(RegExp(r'[/\\]'), '_');
    final file = File('${dir.path}/media-${item.id}-$safeName');
    if (!await file.exists() || await file.length() != item.fileSize) {
      await api.downloadMediaTo(item.contentUrl, file);
    }
    messenger.hideCurrentSnackBar();
    final result = await OpenFilex.open(file.path, type: item.mimeType);
    if (result.type != ResultType.done) {
      messenger.showSnackBar(
        SnackBar(content: Text('Couldn\'t open the file: ${result.message}')),
      );
    }
  } catch (e) {
    messenger
      ..hideCurrentSnackBar()
      ..showSnackBar(SnackBar(content: Text(errorMessage(e))));
  }
}

class ImageViewer extends StatefulWidget {
  const ImageViewer({super.key, required this.images, this.initialIndex = 0});

  final List<MediaItem> images;
  final int initialIndex;

  @override
  State<ImageViewer> createState() => _ImageViewerState();
}

class _ImageViewerState extends State<ImageViewer> {
  late final PageController _controller = PageController(
    initialPage: widget.initialIndex,
  );
  late int _index = widget.initialIndex;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final item = widget.images[_index];
    return Scaffold(
      backgroundColor: Colors.black,
      appBar: AppBar(
        backgroundColor: Colors.black,
        foregroundColor: Colors.white,
        title: Text(item.fileName, overflow: TextOverflow.ellipsis),
      ),
      body: PageView.builder(
        controller: _controller,
        itemCount: widget.images.length,
        onPageChanged: (i) => setState(() => _index = i),
        itemBuilder: (context, i) => InteractiveViewer(
          maxScale: 5,
          child: AuthImage(widget.images[i].contentUrl, fit: BoxFit.contain),
        ),
      ),
    );
  }
}

/// Files picked on the device and waiting to be uploaded.
class PendingFilesList extends StatelessWidget {
  const PendingFilesList({
    super.key,
    required this.files,
    required this.onRemove,
  });

  final List<UploadFile> files;
  final void Function(UploadFile) onRemove;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        for (final f in files)
          ListTile(
            dense: true,
            contentPadding: EdgeInsets.zero,
            leading: Icon(
              f.path != null && _looksLikeImage(f.filename)
                  ? Icons.image_outlined
                  : Icons.attach_file,
            ),
            title: Text(f.filename, overflow: TextOverflow.ellipsis),
            subtitle: const Text('Will be uploaded on save'),
            trailing: IconButton(
              tooltip: 'Remove',
              icon: const Icon(Icons.close),
              onPressed: () => onRemove(f),
            ),
          ),
      ],
    );
  }
}

bool _looksLikeImage(String name) => RegExp(
  r'\.(jpe?g|png|gif|webp|heic|heif)$',
  caseSensitive: false,
).hasMatch(name);

/// Buttons to pick photos/videos from the gallery, capture with the camera or
/// choose any file. Calls [onPicked] with the chosen files.
class AttachButtons extends StatelessWidget {
  const AttachButtons({super.key, required this.onPicked, this.enabled = true});

  final void Function(List<UploadFile>) onPicked;
  final bool enabled;

  Future<void> _run(
    BuildContext context,
    Future<List<UploadFile>> Function() pick,
  ) async {
    try {
      final files = await pick();
      if (files.isNotEmpty) onPicked(files);
    } catch (e) {
      if (context.mounted) showSnack(context, 'Couldn\'t pick files: $e');
    }
  }

  static UploadFile _fromXFile(XFile f) =>
      UploadFile.path(f.path, filename: f.name);

  @override
  Widget build(BuildContext context) {
    final picker = ImagePicker();
    // Desktop platforms (e.g. Linux) have no camera support in image_picker.
    final hasCamera = picker.supportsImageSource(ImageSource.camera);
    return Wrap(
      spacing: 8,
      runSpacing: 8,
      children: [
        OutlinedButton.icon(
          onPressed: !enabled
              ? null
              : () => _run(
                  context,
                  () async => (await picker.pickMultipleMedia())
                      .map(_fromXFile)
                      .toList(),
                ),
          icon: const Icon(Icons.photo_library_outlined),
          label: const Text('Gallery'),
        ),
        if (hasCamera) ...[
          OutlinedButton.icon(
            onPressed: !enabled
                ? null
                : () => _run(context, () async {
                    final f = await picker.pickImage(
                      source: ImageSource.camera,
                    );
                    return [if (f != null) _fromXFile(f)];
                  }),
            icon: const Icon(Icons.photo_camera_outlined),
            label: const Text('Photo'),
          ),
          OutlinedButton.icon(
            onPressed: !enabled
                ? null
                : () => _run(context, () async {
                    final f = await picker.pickVideo(
                      source: ImageSource.camera,
                    );
                    return [if (f != null) _fromXFile(f)];
                  }),
            icon: const Icon(Icons.videocam_outlined),
            label: const Text('Video'),
          ),
        ],
        OutlinedButton.icon(
          onPressed: !enabled
              ? null
              : () => _run(context, () async {
                  final picked = await FilePicker.pickFiles(type: FileType.any);
                  return [
                    for (final f in picked)
                      f.path != null
                          ? UploadFile.path(f.path!, filename: f.name)
                          : UploadFile.bytes(
                              await f.readAsBytes(),
                              filename: f.name,
                            ),
                  ];
                }),
          icon: const Icon(Icons.attach_file),
          label: const Text('Files'),
        ),
      ],
    );
  }
}
