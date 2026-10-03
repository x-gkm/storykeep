# Storykeep REST API

Base URL: `http://127.0.0.1:3000/api` in development. All bodies are JSON (`Content-Type: application/json`) unless noted.

| Area | Document |
|------|----------|
| Registration, login, sessions, current user | [auth-users.md](auth-users.md) |
| Profiles, relationships, members | [profiles-relationships.md](profiles-relationships.md) |
| Memories, timeline, tags | [memories.md](memories.md) |
| Time capsules | [capsules.md](capsules.md) |

`GET /health` (outside `/api`) reports server and database status.

## Authentication

Log in or register to get a `token`, then send it on every protected request:

```
Authorization: Bearer <token>
```

Tokens last 30 days. A missing, unknown or expired token returns `401 unauthorized`.

## Errors

Every error has the same shape:

```json
{ "error": { "code": "not_found", "message": "relationship not found" } }
```

| Status | `code` | Meaning |
|--------|--------|---------|
| 400 | `bad_request` | Invalid body, query or field value; `message` says which |
| 401 | `unauthorized` | Missing or invalid token |
| 401 | `invalid_credentials` | Wrong email/password (login, password change) |
| 403 | `forbidden` | You're a member, but your role doesn't allow this |
| 404 | `not_found` | Doesn't exist **or** you aren't a member — the two are indistinguishable on purpose |
| 405 | `method_not_allowed` | |
| 409 | `conflict` | Duplicate, or would break an invariant (e.g. removing the last owner) |
| 413 | `payload_too_large` | JSON bodies are limited to 64 KiB |
| 415 | `unsupported_media_type` | |
| 500 | `internal_error` | Details are logged server-side only |

## Authorization

Access is granted through relationship membership; the server checks it on every request and never trusts ids from the client.

| Role | Read | Write content | Manage relationship & members | Delete relationship |
|------|------|---------------|-------------------------------|---------------------|
| `OWNER` | ✓ | ✓ | ✓ | ✓ |
| `PARENT` | ✓ | ✓ | ✓ | |
| `MEMBER` | ✓ | ✓ | | |
| `VIEWER` | ✓ | | | |

"Content" is memories, media, tags, development data and time capsules. Only an `OWNER` can grant, change or remove the `OWNER` role, and every relationship keeps at least one `OWNER`.

Access to a **profile** comes from your strongest role in any relationship with it.

## Conventions

- Lookup values are exchanged by name (`"CHILD"`, `"PARENT_CHILD"`, …). `GET /api/reference` lists them all; it needs no token.
- Dates are `YYYY-MM-DD`; timestamps are RFC 3339 in UTC (`2026-10-03T17:24:13.088527Z`).
- Text fields are trimmed; required text must be non-empty after trimming.
- Ids are integers.
- `201 Created` returns the created resource; `204 No Content` has no body.

### `GET /api/reference`

```json
{
  "profile_types": ["CHILD", "PET", "PERSON", "OTHER"],
  "relationship_types": ["PARENT_CHILD", "OWNER_PET", "FRIEND", "FAMILY", "PARTNER", "OTHER"],
  "relationship_roles": ["OWNER", "PARENT", "MEMBER", "VIEWER"],
  "memory_categories": ["GENERAL", "MILESTONE", "BIRTHDAY", "HOLIDAY", "TRAVEL", "FIRST_TIME", "EVERYDAY"],
  "media_types": ["IMAGE", "VIDEO", "AUDIO", "DOCUMENT"],
  "development_domains": ["PHYSICAL", "MOTOR", "LANGUAGE", "COGNITIVE", "SOCIAL_EMOTIONAL"],
  "measurement_types": [{ "name": "HEIGHT", "unit": "cm" }, { "name": "WEIGHT", "unit": "kg" }, { "name": "HEAD_CIRCUMFERENCE", "unit": "cm" }],
  "time_capsule_statuses": ["LOCKED", "AVAILABLE", "OPENED", "CANCELLED"]
}
```
