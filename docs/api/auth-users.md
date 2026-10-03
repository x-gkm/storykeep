# Auth and users

## User object

```json
{
  "id": 1,
  "email": "ada@example.com",
  "first_name": "Ada",
  "last_name": "Lovelace",
  "date_of_birth": "1990-12-10",
  "created_at": "2026-10-03T17:24:13Z",
  "updated_at": "2026-10-03T17:24:13Z"
}
```

`date_of_birth` may be `null`. The password hash is never returned.

## Session object

Returned by register and login:

```json
{ "token": "64 hex characters", "expires_at": "2026-11-02T17:24:13Z", "user": { "...": "User object" } }
```

## `POST /api/auth/register`

No token. Creates an account and logs it in.

```json
{ "email": "ada@example.com", "password": "correct horse battery", "first_name": "Ada", "last_name": "Lovelace", "date_of_birth": "1990-12-10" }
```

- `email` is trimmed and lowercased; must be unique (case-insensitive) → `409 conflict` otherwise.
- `password`: 8–128 characters. Stored as an Argon2id hash.
- `first_name`, `last_name`: required, ≤ 100 characters.
- `date_of_birth`: optional, not in the future.

→ `201` Session object.

## `POST /api/auth/login`

No token.

```json
{ "email": "ada@example.com", "password": "correct horse battery" }
```

→ `200` Session object, or `401 invalid_credentials` (same response whether the email or the password is wrong).

## `POST /api/auth/logout`

Ends the current session (other devices stay signed in). → `204`.

## `GET /api/users/me`

→ `200` User object.

## `PUT /api/users/me`

```json
{ "first_name": "Ada", "last_name": "King", "date_of_birth": "1990-12-10" }
```

Same rules as registration. The email can't be changed. → `200` updated User object.

## `PUT /api/users/me/password`

```json
{ "current_password": "correct horse battery", "new_password": "a brand new password" }
```

→ `204`. Wrong `current_password` → `401 invalid_credentials`. Signs out every other session; the current one stays valid.
