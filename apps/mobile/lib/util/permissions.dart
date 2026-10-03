/// Client-side mirror of the API's role rules (docs/api/README.md), used only to
/// hide actions the user can't perform. The server remains the authority.
abstract final class Perms {
  /// Create memories, media, tags, development data and capsules.
  static bool canWrite(String role) => role == 'OWNER' || role == 'PARENT' || role == 'MEMBER';

  /// Manage a relationship, its members, or a profile.
  static bool canManage(String role) => role == 'OWNER' || role == 'PARENT';

  static bool isOwner(String role) => role == 'OWNER';

  /// Edit or delete an item: its creator while they can write, or a manager.
  static bool canEditItem(String role, {required int createdBy, required int? currentUserId}) =>
      canManage(role) || (canWrite(role) && createdBy == currentUserId);

  /// Remove a media item: its uploader while they can write, or a manager. When the
  /// payload has no `uploaded_by` (older servers), [fallback] decides.
  static bool canRemoveMedia(String role, {required int? uploadedBy, required int? currentUserId, bool fallback = false}) {
    if (canManage(role)) return true;
    if (!canWrite(role)) return false;
    return uploadedBy == null ? fallback : uploadedBy == currentUserId;
  }

  /// Roles [actorRole] may assign to others: only an owner may grant `OWNER`.
  static List<String> assignableRoles(String actorRole) =>
      isOwner(actorRole) ? const ['OWNER', 'PARENT', 'MEMBER', 'VIEWER'] : const ['PARENT', 'MEMBER', 'VIEWER'];

  /// Whether [actorRole] may change the role of, or remove, a member holding [targetRole].
  static bool canManageMember(String actorRole, String targetRole) =>
      canManage(actorRole) && (targetRole != 'OWNER' || isOwner(actorRole));
}
