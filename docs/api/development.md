# Child development and measurements

A **development record** is a dated note about a child, holding any number of **observations**, each tagged with a development domain (`PHYSICAL`, `MOTOR`, `LANGUAGE`, `COGNITIVE`, `SOCIAL_EMOTIONAL`). Records exist for `CHILD` profiles only.

A **measurement** is a dated numeric value of a measurement type (`HEIGHT` in cm, `WEIGHT` in kg, `HEAD_CIRCUMFERENCE` in cm). Measurements work for **any** profile type, e.g. a pet's weight.

Both belong to the profile, not to a relationship: everyone with access to the profile through any relationship sees the same data. The API only stores what users enter — it does not interpret, score or compare it (no percentiles, no diagnosis).

## Permissions

| Action | Who |
|--------|-----|
| Read | Any role |
| Create | `OWNER`, `PARENT`, `MEMBER` |
| Edit / delete | The item's creator while they still have write access, or anyone with `OWNER`/`PARENT` |

Your role is your strongest one across all relationships with the profile. `VIEWER`s get `403` on writes; users without access get `404` for the profile, record or measurement.

## Date rules

`record_date` and `measurement_date` must not be in the future and not before the profile's `date_of_birth` (if set) → `400` otherwise.

## Development record object

```json
{
  "id": 12,
  "profile_id": 7,
  "record_date": "2025-03-01",
  "notes": "First birthday week",
  "created_by": { "id": 1, "first_name": "Ada", "last_name": "Lovelace" },
  "created_at": "2026-10-03T17:24:13Z",
  "observations": [
    { "id": 30, "domain": "MOTOR", "observation": "Took three steps unaided", "created_at": "2026-10-03T17:24:13Z" },
    { "id": 31, "domain": "LANGUAGE", "observation": "Says \"mama\"", "created_at": "2026-10-03T17:24:13Z" }
  ]
}
```

Observations keep the order they were sent in.

## Development records

### `GET /api/profiles/{id}/development-records`

Read access. Newest first (`record_date`, then newest created). → `200` array of Development record objects.

| Query | |
|-------|---|
| `from`, `to` | Inclusive `record_date` range; `to` must not be before `from` |
| `domain` | Only records with at least one observation in this domain (the records still include all their observations) |

Non-`CHILD` profile → `400`.

### `POST /api/profiles/{id}/development-records`

Write access; `CHILD` profiles only (`400` otherwise).

```json
{
  "record_date": "2025-03-01",
  "notes": "First birthday week",
  "observations": [
    { "domain": "MOTOR", "observation": "Took three steps unaided" },
    { "domain": "LANGUAGE", "observation": "Says \"mama\"" }
  ]
}
```

- `record_date` required; see [date rules](#date-rules).
- `notes` optional (≤ 10 000 chars); `observations` optional, at most 50, each `observation` non-empty (≤ 5 000 chars). A record needs `notes` or at least one observation.
- Unknown `domain` → `400`.

The record and its observations are saved together (all or nothing). → `201` Development record object.

### `GET /api/development-records/{id}`

Read access. → `200` Development record object.

### `PUT /api/development-records/{id}`

Edit permission (see above). Same body and rules as `POST`; **replaces** `record_date`, `notes` and the whole `observations` list (omitted `notes` becomes `null`, omitted `observations` becomes empty). Observations get new ids. → `200` Development record object.

### `DELETE /api/development-records/{id}`

Edit permission. Deletes the record and its observations. → `204`.

## Measurement object

```json
{
  "id": 4,
  "profile_id": 7,
  "measurement_type": "HEIGHT",
  "unit": "cm",
  "value": 74.25,
  "measurement_date": "2025-03-01",
  "created_by": { "id": 1, "first_name": "Ada", "last_name": "Lovelace" },
  "created_at": "2026-10-03T17:24:13Z"
}
```

`value` is a JSON number; `unit` comes from the measurement type (see `GET /api/reference`).

## Measurements

### `GET /api/profiles/{id}/measurements`

Read access. Oldest first (`measurement_date`, then type, then creation). → `200` array of Measurement objects.

| Query | |
|-------|---|
| `type` | Only this measurement type, e.g. `HEIGHT` |
| `from`, `to` | Inclusive `measurement_date` range; `to` must not be before `from` |

### `POST /api/profiles/{id}/measurements`

Write access; any profile type.

```json
{ "measurement_type": "HEIGHT", "value": 74.25, "measurement_date": "2025-03-01" }
```

- `value` must be greater than 0, less than 10 000 000 and have at most 3 decimal places → `400` otherwise.
- `measurement_date` required; see [date rules](#date-rules).

→ `201` Measurement object.

### `GET /api/measurements/{id}`

Read access. → `200` Measurement object.

### `PUT /api/measurements/{id}`

Edit permission. Same body and rules as `POST`; replaces all three fields. → `200` Measurement object.

### `DELETE /api/measurements/{id}`

Edit permission. → `204`.

## Chart series

### `GET /api/profiles/{id}/measurements/series`

Read access. Points for charting, ordered by date. Accepts the same `type`, `from` and `to` query parameters as the list.

With `?type=HEIGHT` → `200` one series (`points` is empty if nothing is recorded):

```json
{
  "measurement_type": "HEIGHT",
  "unit": "cm",
  "points": [
    { "date": "2025-01-15", "value": 72.0 },
    { "date": "2025-03-01", "value": 76.125 }
  ]
}
```

Without `type` → `200` array with one such series per measurement type that has data in the range, in reference-data order (`HEIGHT`, `WEIGHT`, `HEAD_CIRCUMFERENCE`); `[]` if there is none.

Several measurements of one type on the same date each produce a point.
