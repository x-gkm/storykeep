# Memories, timeline and tags

A **memory** is something that happened to a relationship's profile: a title, an optional description, a category, the date it happened, tags and attached media. A relationship's **timeline** is its memories ordered by that date.

## Who may do what

| Action | Allowed for |
|--------|-------------|
| Read memories, the timeline and tags | Any member (`OWNER`, `PARENT`, `MEMBER`, `VIEWER`) |
| Create a memory | Write access: `OWNER`, `PARENT`, `MEMBER` |
| Edit or delete a memory | Its creator, while they still have write access, **or** anyone with manage access (`OWNER`, `PARENT`) |

A memory in a relationship you aren't a member of returns `404`, exactly like one that doesn't exist. A `VIEWER` (or a `MEMBER` editing someone else's memory) gets `403`.

## Memory object

```json
{
  "id": 12,
  "relationship_id": 3,
  "category": "MILESTONE",
  "title": "First steps",
  "description": "Across the living room",
  "memory_date": "2025-03-14",
  "created_by": { "id": 1, "first_name": "Ada", "last_name": "Lovelace" },
  "created_at": "2026-10-03T17:24:13Z",
  "updated_at": "2026-10-03T17:24:13Z",
  "tags": ["Home", "walking"],
  "media": [
    {
      "id": 40,
      "media_type": "IMAGE",
      "file_name": "steps.jpg",
      "mime_type": "image/jpeg",
      "file_size": 248113,
      "uploaded_by": { "id": 1, "first_name": "Ada", "last_name": "Lovelace" },
      "created_at": "2026-10-03T17:25:02Z",
      "content_url": "/api/media/40/content"
    }
  ]
}
```

- `memory_date` is when the event **happened**; `created_at` is when it was **recorded**. Show them separately.
- `category` is one of `GENERAL`, `MILESTONE`, `BIRTHDAY`, `HOLIDAY`, `TRAVEL`, `FIRST_TIME`, `EVERYDAY` (see `GET /api/reference`).
- `tags` are names, sorted case-insensitively.
- `media` lists attached files, oldest first; fetch a file's bytes from its `content_url` (with your token). Uploading is described in the media docs.
- `created_by` keeps pointing at the original author when someone else edits the memory.

## Timeline

### `GET /api/relationships/{id}/memories`

Read access. Returns the relationship's memories ordered by `memory_date`, then by `id`.

| Query parameter | Default | Meaning |
|-----------------|---------|---------|
| `from` | | Only memories on or after this date (`YYYY-MM-DD`) |
| `to` | | Only memories on or before this date; must not be before `from` |
| `category` | | Only this category, e.g. `BIRTHDAY` |
| `tag` | | Only memories with this tag (case-insensitive) |
| `q` | | Case-insensitive substring search in title and description (≤ 200 chars) |
| `order` | `desc` | `desc` = newest first, `asc` = oldest first |
| `limit` | `50` | Page size, 1–200 |
| `offset` | `0` | Memories to skip, ≥ 0 |

Filters combine with AND. Invalid values → `400`.

```
GET /api/relationships/3/memories?category=BIRTHDAY&from=2025-01-01&limit=20
```

→ `200`

```json
{ "memories": [Memory, ...], "total": 7, "limit": 20, "offset": 0 }
```

`total` counts every memory matching the filters, ignoring `limit` and `offset`, so the UI can tell whether more pages exist.

## Memories

### `POST /api/relationships/{id}/memories`

Write access. You become the memory's creator.

```json
{
  "category": "MILESTONE",
  "title": "First steps",
  "description": "Across the living room",
  "memory_date": "2025-03-14",
  "tags": ["walking", "Home"]
}
```

- `title` required, ≤ 200 chars. `description` optional, ≤ 10 000 chars; blank becomes `null`.
- `memory_date` required.
- `tags` optional, at most 20; each name ≤ 50 chars and non-blank. Names are matched to existing tags ignoring case (an existing tag keeps its original spelling); new names create tags. Duplicates such as `"cake"` and `"Cake"` collapse into one.

→ `201` Memory object.

### `GET /api/memories/{id}`

Read access. → `200` Memory object.

### `PUT /api/memories/{id}`

Creator or manage access. Replaces every editable field, with the same body and rules as create. Omitting `tags` removes all tags. Attached media is unchanged.

→ `200` Memory object.

### `DELETE /api/memories/{id}`

Creator or manage access. Removes the memory with its tag and media links. → `204`.

## Tags

Tags are shared by everyone and unique regardless of case, but a listing only shows tags used on memories you can read, with how many such memories use each.

### Tag object

```json
{ "name": "Beach", "memory_count": 3 }
```

### `GET /api/tags`

Tags on memories in any of your relationships, ordered by name. Query parameters: `q` (case-insensitive substring, ≤ 50 chars) and `limit` (default 50, 1–200). → `200` array of Tag objects.

### `GET /api/relationships/{id}/tags`

Read access. Same as above, limited to that relationship's memories. → `200` array of Tag objects.

### `POST /api/tags`

Any signed-in user. Creates a tag, or finds the existing one with the same name ignoring case.

```json
{ "name": "Picnic" }
```

→ `200` Tag object (always `200`, whether or not the tag already existed). `name` is the stored spelling if the tag is already in use on memories you can read, otherwise the spelling you sent; `memory_count` counts only memories you can read. Attaching tags to a memory doesn't require this call, since create and update accept names directly.
