# Time capsules

A **time capsule** belongs to a relationship and holds a message (and media) sealed until its `unlock_at` time. The lock is enforced by the server: before a capsule is unlocked **and opened**, no endpoint returns its message or media to anyone — not even its creator. All time checks use the database clock.

## Lifecycle

```
                 create
                   │
                   ▼
   ┌──────────── LOCKED ──────────────┐
   │  edit, cancel, attach media      │ unlock_at passes
   │                                  ▼
   │ cancel                       AVAILABLE ──── POST …/open ───► OPENED
   ▼                              (read-only)     (any member)    (content visible)
CANCELLED
(terminal, content never visible)
```

| Status | Meaning | Content returned? | Can edit/cancel? |
|--------|---------|-------------------|------------------|
| `LOCKED` | `unlock_at` is in the future | no | yes |
| `AVAILABLE` | `unlock_at` has passed, nobody has opened it yet | no | no |
| `OPENED` | a member has opened it | yes (`GET` and `open`) | no |
| `CANCELLED` | cancelled while locked | never | no |

`AVAILABLE` is computed when the capsule is read: a capsule becomes `AVAILABLE` the moment `unlock_at` passes, without any request. Opening is recorded once for the whole capsule (not per member). Any capsule can be deleted (see below).

## Capsule object

Metadata, returned by every capsule endpoint:

```json
{
  "id": 12,
  "relationship_id": 3,
  "title": "For your 18th birthday",
  "status": "LOCKED",
  "unlock_at": "2042-03-01T08:00:00Z",
  "created_by": 2,
  "created_at": "2026-10-03T17:24:13Z",
  "updated_at": "2026-10-03T17:24:13Z",
  "media_count": 1
}
```

Only when `status` is `OPENED` (from `GET /api/capsules/{id}` or `POST /api/capsules/{id}/open`) does it also contain the content:

```json
{
  "…": "metadata as above, with \"status\": \"OPENED\"",
  "message": "Happy birthday! …",
  "media": [
    {
      "id": 40,
      "media_type": "IMAGE",
      "file_name": "first-steps.jpg",
      "mime_type": "image/jpeg",
      "file_size": 1234,
      "uploaded_by": { "id": 1, "first_name": "Ada", "last_name": "Lovelace" },
      "created_at": "2026-10-03T17:30:00Z",
      "content_url": "/api/media/40/content"
    }
  ]
}
```

Before that the `message` and `media` keys are **absent**, not `null`. An opened capsule without a message has `"message": null`. Lists never include content.

## Permissions

All endpoints need membership of the capsule's relationship; otherwise `404`.

| Action | Who |
|--------|-----|
| List, get, open | any member (including `VIEWER`) |
| Create | write access (`OWNER`, `PARENT`, `MEMBER`) |
| Update, cancel, delete | the capsule's creator (while they still have write access), or anyone with manage access (`OWNER`, `PARENT`) |

## Endpoints

### `GET /api/relationships/{id}/capsules`

Read access. Capsule objects (metadata only), soonest `unlock_at` first. Optional filter on the effective status: `?status=LOCKED` (`AVAILABLE`, `OPENED`, `CANCELLED`); an unknown status → `400`. → `200` array.

### `POST /api/relationships/{id}/capsules`

Write access.

```json
{ "title": "For your 18th birthday", "message": "Happy birthday! …", "unlock_at": "2042-03-01T08:00:00Z" }
```

- `title` required (≤ 200 chars); `message` optional (≤ 20 000 chars).
- `unlock_at` is an RFC 3339 timestamp; it must be in the future and at most 100 years ahead → `400` otherwise.

→ `201` Capsule object with status `LOCKED` (without the message you just sent).

### `GET /api/capsules/{id}`

Read access. → `200` Capsule object; includes `message` and `media` only if `OPENED`.

### `PUT /api/capsules/{id}`

Update rule (creator or manage access). Only while `LOCKED` → `409` otherwise. Partial update — omitted fields are kept, since the message can't be read back while sealed:

```json
{ "title": "For your 18th", "message": "New text", "unlock_at": "2042-03-01T08:00:00Z" }
```

`"message": null` (or blank) clears the message. The same validation as on create applies; `unlock_at` may be moved earlier or later as long as it stays in the future. → `200` Capsule object (metadata only).

### `POST /api/capsules/{id}/cancel`

Update rule. Only while `LOCKED` → `409` otherwise. The capsule becomes `CANCELLED` permanently and its content is never returned. No body. → `200` Capsule object.

### `POST /api/capsules/{id}/open`

Read access (any member). No body.

- `AVAILABLE` → becomes `OPENED`; `OPENED` → returned again (idempotent). → `200` Capsule object **with** `message` and `media`.
- `LOCKED` or `CANCELLED` → `409`, without content.

### `DELETE /api/capsules/{id}`

Update rule, in any status. Removes the capsule and its media links (the media files themselves are managed by the media endpoints). → `204`.

## Media

Media are attached by uploading to `POST /api/capsules/{id}/media` while the capsule is `LOCKED` (see the media documentation). Media served from `content_url` follow the same lock rule. While sealed, only `media_count` reveals that media exist.
