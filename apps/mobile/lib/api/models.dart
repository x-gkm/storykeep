// Data transfer objects mirroring docs/api/*.md.
//
// Dates (`YYYY-MM-DD`) are parsed into local-midnight [DateTime]s and written back
// with [formatApiDate]; timestamps (RFC 3339, UTC) are parsed as UTC [DateTime]s.

typedef Json = Map<String, dynamic>;

/// Formats a calendar date as the API's `YYYY-MM-DD`.
String formatApiDate(DateTime d) =>
    '${d.year.toString().padLeft(4, '0')}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';

DateTime _date(Object? v) {
  final s = v as String;
  final parts = s.split('-').map(int.parse).toList();
  return DateTime(parts[0], parts[1], parts[2]);
}

DateTime? _dateOrNull(Object? v) => v == null ? null : _date(v);

DateTime _timestamp(Object? v) => DateTime.parse(v as String).toUtc();

double _num(Object? v) => (v as num).toDouble();

/// Lookup values (see `GET /api/reference`). The API exchanges them by name.
abstract final class Lookups {
  static const profileTypes = ['CHILD', 'PET', 'PERSON', 'OTHER'];
  static const relationshipTypes = ['PARENT_CHILD', 'OWNER_PET', 'FRIEND', 'FAMILY', 'PARTNER', 'OTHER'];
  static const roles = ['OWNER', 'PARENT', 'MEMBER', 'VIEWER'];
  static const memoryCategories = ['GENERAL', 'MILESTONE', 'BIRTHDAY', 'HOLIDAY', 'TRAVEL', 'FIRST_TIME', 'EVERYDAY'];
  static const mediaTypes = ['IMAGE', 'VIDEO', 'AUDIO', 'DOCUMENT'];
  static const developmentDomains = ['PHYSICAL', 'MOTOR', 'LANGUAGE', 'COGNITIVE', 'SOCIAL_EMOTIONAL'];
  static const measurementTypes = ['HEIGHT', 'WEIGHT', 'HEAD_CIRCUMFERENCE'];
  static const measurementUnits = {'HEIGHT': 'cm', 'WEIGHT': 'kg', 'HEAD_CIRCUMFERENCE': 'cm'};
  static const capsuleStatuses = ['LOCKED', 'AVAILABLE', 'OPENED', 'CANCELLED'];
}

class ReferenceData {
  const ReferenceData({
    required this.profileTypes,
    required this.relationshipTypes,
    required this.relationshipRoles,
    required this.memoryCategories,
    required this.mediaTypes,
    required this.developmentDomains,
    required this.measurementUnits,
    required this.timeCapsuleStatuses,
  });

  factory ReferenceData.fromJson(Json j) {
    List<String> names(String key) => (j[key] as List).cast<String>();
    return ReferenceData(
      profileTypes: names('profile_types'),
      relationshipTypes: names('relationship_types'),
      relationshipRoles: names('relationship_roles'),
      memoryCategories: names('memory_categories'),
      mediaTypes: names('media_types'),
      developmentDomains: names('development_domains'),
      measurementUnits: {
        for (final m in (j['measurement_types'] as List).cast<Json>()) m['name'] as String: m['unit'] as String,
      },
      timeCapsuleStatuses: names('time_capsule_statuses'),
    );
  }

  final List<String> profileTypes;
  final List<String> relationshipTypes;
  final List<String> relationshipRoles;
  final List<String> memoryCategories;
  final List<String> mediaTypes;
  final List<String> developmentDomains;

  /// Measurement type name → unit, in reference order.
  final Map<String, String> measurementUnits;
  final List<String> timeCapsuleStatuses;
}

class User {
  const User({
    required this.id,
    required this.email,
    required this.firstName,
    required this.lastName,
    this.dateOfBirth,
    required this.createdAt,
    required this.updatedAt,
  });

  factory User.fromJson(Json j) => User(
        id: j['id'] as int,
        email: j['email'] as String,
        firstName: j['first_name'] as String,
        lastName: j['last_name'] as String,
        dateOfBirth: _dateOrNull(j['date_of_birth']),
        createdAt: _timestamp(j['created_at']),
        updatedAt: _timestamp(j['updated_at']),
      );

  final int id;
  final String email;
  final String firstName;
  final String lastName;
  final DateTime? dateOfBirth;
  final DateTime createdAt;
  final DateTime updatedAt;

  String get fullName => '$firstName $lastName';
}

class Session {
  const Session({required this.token, required this.expiresAt, required this.user});

  factory Session.fromJson(Json j) => Session(
        token: j['token'] as String,
        expiresAt: _timestamp(j['expires_at']),
        user: User.fromJson(j['user'] as Json),
      );

  final String token;
  final DateTime expiresAt;
  final User user;
}

class Profile {
  const Profile({
    required this.id,
    required this.profileType,
    required this.name,
    this.dateOfBirth,
    required this.createdAt,
    required this.updatedAt,
    required this.role,
  });

  factory Profile.fromJson(Json j) => Profile(
        id: j['id'] as int,
        profileType: j['profile_type'] as String,
        name: j['name'] as String,
        dateOfBirth: _dateOrNull(j['date_of_birth']),
        createdAt: _timestamp(j['created_at']),
        updatedAt: _timestamp(j['updated_at']),
        role: j['role'] as String,
      );

  final int id;
  final String profileType;
  final String name;
  final DateTime? dateOfBirth;
  final DateTime createdAt;
  final DateTime updatedAt;

  /// Your strongest role across your relationships with this profile.
  final String role;

  bool get isChild => profileType == 'CHILD';
}

class Relationship {
  const Relationship({
    required this.id,
    required this.profileId,
    required this.profileName,
    required this.profileType,
    required this.relationshipType,
    this.startedAt,
    this.endedAt,
    required this.createdAt,
    required this.role,
  });

  factory Relationship.fromJson(Json j) => Relationship(
        id: j['id'] as int,
        profileId: j['profile_id'] as int,
        profileName: j['profile_name'] as String,
        profileType: j['profile_type'] as String,
        relationshipType: j['relationship_type'] as String,
        startedAt: _dateOrNull(j['started_at']),
        endedAt: _dateOrNull(j['ended_at']),
        createdAt: _timestamp(j['created_at']),
        role: j['role'] as String,
      );

  final int id;
  final int profileId;
  final String profileName;
  final String profileType;
  final String relationshipType;
  final DateTime? startedAt;
  final DateTime? endedAt;
  final DateTime createdAt;

  /// Your role in this relationship.
  final String role;
}

class CreatedProfile {
  const CreatedProfile({required this.profile, required this.relationship});

  factory CreatedProfile.fromJson(Json j) => CreatedProfile(
        profile: Profile.fromJson(j['profile'] as Json),
        relationship: Relationship.fromJson(j['relationship'] as Json),
      );

  final Profile profile;
  final Relationship relationship;
}

class Member {
  const Member({
    required this.userId,
    required this.email,
    required this.firstName,
    required this.lastName,
    required this.role,
    required this.joinedAt,
  });

  factory Member.fromJson(Json j) => Member(
        userId: j['user_id'] as int,
        email: j['email'] as String,
        firstName: j['first_name'] as String,
        lastName: j['last_name'] as String,
        role: j['role'] as String,
        joinedAt: _timestamp(j['joined_at']),
      );

  final int userId;
  final String email;
  final String firstName;
  final String lastName;
  final String role;
  final DateTime joinedAt;

  String get fullName => '$firstName $lastName';
}

/// A short user reference (`created_by`, `uploaded_by`).
class Author {
  const Author({required this.id, required this.firstName, required this.lastName});

  factory Author.fromJson(Json j) =>
      Author(id: j['id'] as int, firstName: j['first_name'] as String, lastName: j['last_name'] as String);

  final int id;
  final String firstName;
  final String lastName;

  String get fullName => '$firstName $lastName';
}

class MediaItem {
  const MediaItem({
    required this.id,
    required this.mediaType,
    required this.fileName,
    required this.mimeType,
    this.fileSize,
    this.uploadedBy,
    required this.createdAt,
    required this.contentUrl,
  });

  factory MediaItem.fromJson(Json j) => MediaItem(
        id: j['id'] as int,
        mediaType: j['media_type'] as String,
        fileName: j['file_name'] as String,
        mimeType: j['mime_type'] as String,
        fileSize: j['file_size'] as int?,
        uploadedBy: j['uploaded_by'] == null ? null : Author.fromJson(j['uploaded_by'] as Json),
        createdAt: _timestamp(j['created_at']),
        contentUrl: j['content_url'] as String,
      );

  final int id;
  final String mediaType;
  final String fileName;
  final String mimeType;
  final int? fileSize;

  /// Who uploaded the file. Optional: older servers omit it inside memories and capsules.
  final Author? uploadedBy;
  final DateTime createdAt;

  /// Path of the file's bytes, e.g. `/api/media/40/content`; needs the bearer token.
  final String contentUrl;

  bool get isImage => mediaType == 'IMAGE';
}

class Memory {
  const Memory({
    required this.id,
    required this.relationshipId,
    required this.category,
    required this.title,
    this.description,
    required this.memoryDate,
    required this.createdBy,
    required this.createdAt,
    required this.updatedAt,
    required this.tags,
    required this.media,
  });

  factory Memory.fromJson(Json j) => Memory(
        id: j['id'] as int,
        relationshipId: j['relationship_id'] as int,
        category: j['category'] as String,
        title: j['title'] as String,
        description: j['description'] as String?,
        memoryDate: _date(j['memory_date']),
        createdBy: Author.fromJson(j['created_by'] as Json),
        createdAt: _timestamp(j['created_at']),
        updatedAt: _timestamp(j['updated_at']),
        tags: ((j['tags'] as List?) ?? const []).cast<String>(),
        media: ((j['media'] as List?) ?? const []).map((m) => MediaItem.fromJson(m as Json)).toList(),
      );

  final int id;
  final int relationshipId;
  final String category;
  final String title;
  final String? description;

  /// When the event happened.
  final DateTime memoryDate;
  final Author createdBy;

  /// When the memory was recorded.
  final DateTime createdAt;
  final DateTime updatedAt;
  final List<String> tags;
  final List<MediaItem> media;
}

class MemoryInput {
  const MemoryInput({
    required this.category,
    required this.title,
    this.description,
    required this.memoryDate,
    this.tags = const [],
  });

  final String category;
  final String title;
  final String? description;
  final DateTime memoryDate;
  final List<String> tags;

  Json toJson() => {
        'category': category,
        'title': title,
        'description': description,
        'memory_date': formatApiDate(memoryDate),
        'tags': tags,
      };
}

class TimelinePage {
  const TimelinePage({required this.memories, required this.total, required this.limit, required this.offset});

  factory TimelinePage.fromJson(Json j) => TimelinePage(
        memories: (j['memories'] as List).map((m) => Memory.fromJson(m as Json)).toList(),
        total: j['total'] as int,
        limit: j['limit'] as int,
        offset: j['offset'] as int,
      );

  final List<Memory> memories;
  final int total;
  final int limit;
  final int offset;

  bool get hasMore => offset + memories.length < total;
}

/// Filters for `GET /api/relationships/{id}/memories`.
class TimelineFilter {
  const TimelineFilter({this.from, this.to, this.category, this.tag, this.query, this.ascending = false});

  final DateTime? from;
  final DateTime? to;
  final String? category;
  final String? tag;
  final String? query;
  final bool ascending;

  bool get isEmpty =>
      from == null && to == null && category == null && tag == null && (query == null || query!.trim().isEmpty);

  TimelineFilter copyWith({
    DateTime? Function()? from,
    DateTime? Function()? to,
    String? Function()? category,
    String? Function()? tag,
    String? Function()? query,
    bool? ascending,
  }) =>
      TimelineFilter(
        from: from != null ? from() : this.from,
        to: to != null ? to() : this.to,
        category: category != null ? category() : this.category,
        tag: tag != null ? tag() : this.tag,
        query: query != null ? query() : this.query,
        ascending: ascending ?? this.ascending,
      );

  /// Query parameters for one page of the timeline. Unset filters are omitted.
  Map<String, String> toQueryParameters({int limit = 20, int offset = 0}) {
    final q = query?.trim();
    return {
      if (from != null) 'from': formatApiDate(from!),
      if (to != null) 'to': formatApiDate(to!),
      'category': ?category,
      if (tag != null && tag!.trim().isNotEmpty) 'tag': tag!.trim(),
      if (q != null && q.isNotEmpty) 'q': q,
      'order': ascending ? 'asc' : 'desc',
      'limit': '$limit',
      'offset': '$offset',
    };
  }
}

class Tag {
  const Tag({required this.name, required this.memoryCount});

  factory Tag.fromJson(Json j) => Tag(name: j['name'] as String, memoryCount: j['memory_count'] as int);

  final String name;
  final int memoryCount;
}

class Observation {
  const Observation({this.id, required this.domain, required this.observation, this.createdAt});

  factory Observation.fromJson(Json j) => Observation(
        id: j['id'] as int,
        domain: j['domain'] as String,
        observation: j['observation'] as String,
        createdAt: _timestamp(j['created_at']),
      );

  final int? id;
  final String domain;
  final String observation;
  final DateTime? createdAt;

  Json toJson() => {'domain': domain, 'observation': observation};
}

class DevelopmentRecord {
  const DevelopmentRecord({
    required this.id,
    required this.profileId,
    required this.recordDate,
    this.notes,
    required this.createdBy,
    required this.createdAt,
    required this.observations,
  });

  factory DevelopmentRecord.fromJson(Json j) => DevelopmentRecord(
        id: j['id'] as int,
        profileId: j['profile_id'] as int,
        recordDate: _date(j['record_date']),
        notes: j['notes'] as String?,
        createdBy: Author.fromJson(j['created_by'] as Json),
        createdAt: _timestamp(j['created_at']),
        observations: (j['observations'] as List).map((o) => Observation.fromJson(o as Json)).toList(),
      );

  final int id;
  final int profileId;
  final DateTime recordDate;
  final String? notes;
  final Author createdBy;
  final DateTime createdAt;
  final List<Observation> observations;
}

class DevelopmentRecordInput {
  const DevelopmentRecordInput({required this.recordDate, this.notes, this.observations = const []});

  final DateTime recordDate;
  final String? notes;
  final List<Observation> observations;

  Json toJson() => {
        'record_date': formatApiDate(recordDate),
        'notes': notes,
        'observations': observations.map((o) => o.toJson()).toList(),
      };
}

class Measurement {
  const Measurement({
    required this.id,
    required this.profileId,
    required this.measurementType,
    required this.unit,
    required this.value,
    required this.measurementDate,
    required this.createdBy,
    required this.createdAt,
  });

  factory Measurement.fromJson(Json j) => Measurement(
        id: j['id'] as int,
        profileId: j['profile_id'] as int,
        measurementType: j['measurement_type'] as String,
        unit: j['unit'] as String,
        value: _num(j['value']),
        measurementDate: _date(j['measurement_date']),
        createdBy: Author.fromJson(j['created_by'] as Json),
        createdAt: _timestamp(j['created_at']),
      );

  final int id;
  final int profileId;
  final String measurementType;
  final String unit;
  final double value;
  final DateTime measurementDate;
  final Author createdBy;
  final DateTime createdAt;
}

class MeasurementInput {
  const MeasurementInput({required this.measurementType, required this.value, required this.measurementDate});

  final String measurementType;
  final double value;
  final DateTime measurementDate;

  Json toJson() => {
        'measurement_type': measurementType,
        'value': value,
        'measurement_date': formatApiDate(measurementDate),
      };
}

class SeriesPoint {
  const SeriesPoint({required this.date, required this.value});

  factory SeriesPoint.fromJson(Json j) => SeriesPoint(date: _date(j['date']), value: _num(j['value']));

  final DateTime date;
  final double value;
}

class MeasurementSeries {
  const MeasurementSeries({required this.measurementType, required this.unit, required this.points});

  factory MeasurementSeries.fromJson(Json j) => MeasurementSeries(
        measurementType: j['measurement_type'] as String,
        unit: j['unit'] as String,
        points: (j['points'] as List).map((p) => SeriesPoint.fromJson(p as Json)).toList(),
      );

  final String measurementType;
  final String unit;
  final List<SeriesPoint> points;
}

class Capsule {
  const Capsule({
    required this.id,
    required this.relationshipId,
    required this.title,
    required this.status,
    required this.unlockAt,
    required this.createdBy,
    required this.createdAt,
    required this.updatedAt,
    required this.mediaCount,
    this.content,
  });

  /// Parses a capsule. Content (`message`, `media`) is only kept when the status is
  /// `OPENED`, so a sealed capsule can never carry content into the UI.
  factory Capsule.fromJson(Json j) {
    final status = j['status'] as String;
    final opened = status == 'OPENED';
    return Capsule(
      id: j['id'] as int,
      relationshipId: j['relationship_id'] as int,
      title: j['title'] as String,
      status: status,
      unlockAt: _timestamp(j['unlock_at']),
      createdBy: j['created_by'] as int,
      createdAt: _timestamp(j['created_at']),
      updatedAt: _timestamp(j['updated_at']),
      mediaCount: (j['media_count'] as int?) ?? 0,
      content: opened && (j.containsKey('message') || j.containsKey('media'))
          ? CapsuleContent(
              message: j['message'] as String?,
              media: ((j['media'] as List?) ?? const []).map((m) => MediaItem.fromJson(m as Json)).toList(),
            )
          : null,
    );
  }

  final int id;
  final int relationshipId;
  final String title;

  /// `LOCKED`, `AVAILABLE`, `OPENED` or `CANCELLED`.
  final String status;
  final DateTime unlockAt;

  /// User id of the creator.
  final int createdBy;
  final DateTime createdAt;
  final DateTime updatedAt;
  final int mediaCount;

  /// Only present for an `OPENED` capsule fetched individually.
  final CapsuleContent? content;

  bool get isLocked => status == 'LOCKED';
  bool get isAvailable => status == 'AVAILABLE';
  bool get isOpened => status == 'OPENED';
  bool get isCancelled => status == 'CANCELLED';
}

class CapsuleContent {
  const CapsuleContent({this.message, required this.media});

  final String? message;
  final List<MediaItem> media;
}
