# Profiles, relationships and members

A **profile** is the person or animal memories are about. A **relationship** connects users (its members) to one profile. A profile can have several relationships, e.g. a `PARENT_CHILD` one for the parents and a `FAMILY` one for grandparents.

## Profile object

```json
{
  "id": 7,
  "profile_type": "CHILD",
  "name": "Mira",
  "date_of_birth": "2024-03-01",
  "created_at": "2026-10-03T17:24:13Z",
  "updated_at": "2026-10-03T17:24:13Z",
  "role": "OWNER"
}
```

`role` is your strongest role across your relationships with the profile.

## Relationship object

```json
{
  "id": 3,
  "profile_id": 7,
  "profile_name": "Mira",
  "profile_type": "CHILD",
  "relationship_type": "PARENT_CHILD",
  "started_at": "2024-03-01",
  "ended_at": null,
  "created_at": "2026-10-03T17:24:13Z",
  "role": "OWNER"
}
```

`role` is your role in this relationship.

## Member object

```json
{ "user_id": 2, "email": "grace@example.com", "first_name": "Grace", "last_name": "Hopper", "role": "PARENT", "joined_at": "2026-10-03T17:24:13Z" }
```

## Profiles

### `GET /api/profiles`

Profiles you can access, ordered by name. → `200` array of Profile objects.

### `POST /api/profiles`

Creates a profile **and** your first relationship with it; you become its `OWNER`.

```json
{ "profile_type": "CHILD", "name": "Mira", "date_of_birth": "2024-03-01", "relationship_type": "PARENT_CHILD", "started_at": "2024-03-01" }
```

`name` required (≤ 100 chars); `date_of_birth` and `started_at` optional.

→ `201` `{ "profile": Profile, "relationship": Relationship }`

### `GET /api/profiles/{id}`

Read access. → `200` Profile object.

### `PUT /api/profiles/{id}`

Manage access (`OWNER`/`PARENT` in any relationship with the profile).

```json
{ "profile_type": "CHILD", "name": "Mira Rose", "date_of_birth": "2024-03-01" }
```

A profile with development records must stay `CHILD` → `400` otherwise. → `200` Profile object.

### `DELETE /api/profiles/{id}`

You must be `OWNER` of **every** relationship with the profile, since they're all deleted with it (with their memories, capsules, etc.). → `204`.

## Relationships

### `GET /api/relationships`

Your relationships, ordered by profile name. → `200` array of Relationship objects.

### `POST /api/relationships`

Adds another relationship to an existing profile you manage; you become its `OWNER`.

```json
{ "profile_id": 7, "relationship_type": "FAMILY", "started_at": null, "ended_at": null }
```

`ended_at` must not be before `started_at`. → `201` Relationship object.

### `GET /api/relationships/{id}`

Read access. → `200` Relationship object.

### `PUT /api/relationships/{id}`

Manage access.

```json
{ "relationship_type": "FAMILY", "started_at": "2024-03-01", "ended_at": null }
```

→ `200` Relationship object.

### `DELETE /api/relationships/{id}`

`OWNER` only. Deletes its memories, time capsules and memberships (uploaded media files remain). → `204`.

## Members

### `GET /api/relationships/{id}/members`

Read access. → `200` array of Member objects, owners first.

### `POST /api/relationships/{id}/members`

Manage access. Adds an existing user by email.

```json
{ "email": "grace@example.com", "role": "PARENT" }
```

- Unknown email → `404`; already a member → `409`.
- Only an `OWNER` may add another `OWNER` → `403` otherwise.

→ `201` updated array of Member objects.

### `PUT /api/relationships/{id}/members/{user_id}`

Manage access. `{ "role": "VIEWER" }` → `200` updated array of Member objects.

Only an `OWNER` may change an owner's role or grant `OWNER`. Demoting the last `OWNER` → `409`.

### `DELETE /api/relationships/{id}/members/{user_id}`

Any member may remove **themselves** (leave). Removing someone else needs manage access, and removing an `OWNER` needs `OWNER`. Removing the last `OWNER` → `409`. → `204`.
