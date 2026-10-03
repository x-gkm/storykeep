# Media

Images, videos, audio and documents attached to **memories** or **time capsules**. You upload files to a memory or capsule; each file becomes a media item with its own id.

## Media object

```json
{
  "id": 12,
  "media_type": "IMAGE",
  "file_name": "first steps.jpg",
  "mime_type": "image/jpeg",
  "file_size": 482113,
  "uploaded_by": { "id": 2, "first_name": "Grace", "last_name": "Hopper" },
  "created_at": "2026-10-03T17:24:13Z",
  "content_url": "/api/media/12/content"
}
```

- `file_name` is the name the file was uploaded with. Any path components and control characters are removed, and the name is shortened to 255 characters. It is used only for display and downloads; files are stored on the server under generated names.
- `mime_type` is the type found by reading the file's **content**, not the type the client declared.
- `file_size` is in bytes.
- `content_url` returns the file itself, using the same authorization as the metadata.

## Allowed files

| `media_type` | MIME types |
|--------------|------------|
| `IMAGE` | `image/jpeg`, `image/png`, `image/gif`, `image/webp`, `image/heic` (also accepted as `image/heif`) |
| `VIDEO` | `video/mp4`, `video/quicktime`, `video/webm` |
| `AUDIO` | `audio/mpeg`, `audio/mp4` (m4a), `audio/ogg`, `audio/wav`, `audio/webm` |
| `DOCUMENT` | `application/pdf` |

- The server reads each file's first bytes to work out its type. A file whose content isn't one of these formats → `400`, whatever its declared type. SVG, HTML and other active content are never accepted.
- A part's `Content-Type` is optional, and `application/octet-stream` counts as not declaring a type. If you declare a type, it must match the content, otherwise → `400`. A declared type only matters for files that could be either audio or video: `audio/mp4` or `audio/webm` stores the file as `AUDIO`, and otherwise MP4 and WebM files are stored as `VIDEO`. Common aliases are accepted, such as `image/jpg`, `audio/mp3`, `audio/x-m4a` and `audio/x-wav`.
- **Size:** at most **50 MiB per file** → `413 payload_too_large` otherwise. Empty files → `400`.
- At most **20 files per request**.

## Who can do what

| Action | Required |
|--------|----------|
| Upload | Write access (`OWNER`/`PARENT`/`MEMBER`) to the memory's or capsule's relationship |
| Remove | The uploader, if they still have write access; or anyone with manage access (`OWNER`/`PARENT`) |
| Read metadata or content | See below |

You can read a media item if **either** of these is true:

- it's attached to a memory in a relationship you belong to, or
- it's attached to a time capsule in a relationship you belong to, **and** that capsule is `OPENED` and its unlock time has passed.

Media that is only in a capsule that hasn't been opened (`LOCKED`, waiting to be opened after its unlock time, or `CANCELLED`) → `404` for everyone, **including the person who uploaded it**.

Media in a capsule can only be added or removed while the capsule is `LOCKED` and its unlock time is still in the future → `409` otherwise. Members who didn't upload a capsule's media item and can't manage the relationship get `404`, not `403`, when they try to remove it, so they can't find out what's sealed inside.

As elsewhere, anything you can't read → `404 not_found`. A `VIEWER` trying to change something → `403 forbidden`.

## Uploading

### `POST /api/memories/{id}/media`

### `POST /api/capsules/{id}/media`

The request body is `multipart/form-data` with one or more parts named `file`. Any other field → `400`. The whole request succeeds or fails together: if any file is rejected, nothing is stored.

```sh
curl -X POST http://127.0.0.1:3000/api/memories/42/media \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@first-steps.jpg;type=image/jpeg" \
  -F "file=@birthday.mp4"
```

→ `201` array of Media objects, in upload order.

Errors:

- `400`: not a multipart body, no `file` parts, an empty file, a type that isn't allowed, or content that doesn't match the declared type
- `403`: you're a `VIEWER`
- `404`: the memory or capsule doesn't exist, or you aren't a member
- `409`: the capsule isn't `LOCKED` with a future unlock time
- `413`: a file is larger than 50 MiB

## Removing

### `DELETE /api/memories/{id}/media/{media_id}`

### `DELETE /api/capsules/{id}/media/{media_id}`

Detaches the media item from that memory or capsule. Once nothing uses a media item any more, the item and its file are deleted. → `204`.

- `media_id` not attached to that memory or capsule → `404`.
- Capsule media follows the rules under [Who can do what](#who-can-do-what), so it can only be removed while the capsule is locked.

## Reading

### `GET /api/media/{id}`

→ `200` Media object.

### `GET /api/media/{id}/content`

Returns the file's bytes with these headers:

```
Content-Type: image/jpeg
Content-Length: 482113
Content-Disposition: inline; filename="first steps.jpg"; filename*=UTF-8''first%20steps.jpg
X-Content-Type-Options: nosniff
Cache-Control: private
```

In the plain `filename`, non-ASCII characters, quotes and backslashes become `_`. `filename*` carries the exact name, percent-encoded as UTF-8.

The content endpoint needs the same `Authorization` header as every other endpoint, so a plain `<img src>` can't load it. Fetch the file with the token and display it from a blob URL.
