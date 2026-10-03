# Database schema

Built from `migrations/0001`–`0006`. (`0006_sessions` adds the `sessions` table.)

**Legend:** `PK` primary key · `FK` foreign key · `UQ` unique · `1 ── N` one-to-many
(`>`/`<` marks the "many" side). Every FK to `users`, `profiles`, `relationships`,
`memories`, `media` and `time_capsules` is `ON DELETE CASCADE`, except
`created_by`/`uploaded_by`, which block deletion.

## 1. Accounts, profiles and relationships

```
 ┌──────────────────────┐          ┌──────────────────────────┐
 │ sessions             │          │ users                    │
 ├──────────────────────┤ N      1 ├──────────────────────────┤
 │ PK id                │>─────────┤ PK id                    │
 │ FK user_id           │          │    email     UQ(lower)   │
 │    token_hash  UQ    │          │    password_hash         │
 │    created_at        │          │    first_name            │
 │    expires_at        │          │    last_name             │
 └──────────────────────┘          │    date_of_birth         │
                                   │    created_at/updated_at │
                                   └────────────┬─────────────┘
                                                │ 1
                                                │
                                                │ N
                                   ┌────────────┴─────────────┐
                                   │ relationship_members     │
                                   ├──────────────────────────┤
                                   │ PK id                    │
                                   │ FK relationship_id       │
                                   │ FK user_id               │
                                   │ FK role_id ─▶ roles      │
                                   │    joined_at             │
                                   │ UQ(relationship_id,      │
                                   │    user_id)              │
                                   └────────────┬─────────────┘
                                                │ N
                                                │
                                                │ 1
 ┌──────────────────────┐          ┌────────────┴─────────────┐
 │ profiles             │          │ relationships            │
 ├──────────────────────┤ 1      N ├──────────────────────────┤
 │ PK id                ├─────────<│ PK id                    │
 │ FK profile_type_id   │          │ FK profile_id            │
 │    name              │          │ FK relationship_type_id  │
 │    date_of_birth     │          │    started_at            │
 │    created_at        │          │    ended_at  (>= start)  │
 │    updated_at        │          │    created_at            │
 └──────────────────────┘          └──────────────────────────┘
```

## 2. Relationship content: memories, time capsules, media and tags

```
                           ┌──────────────────────────┐
                           │ relationships            │
                           └──────┬────────────┬──────┘
                                1 │            │ 1
                    ┌─────────────┘            └───────────────┐
                  N │                                        N │
 ┌──────────────────┴───────┐                   ┌──────────────┴───────────┐
 │ memories                 │                   │ time_capsules            │
 ├──────────────────────────┤                   ├──────────────────────────┤
 │ PK id                    │                   │ PK id                    │
 │ FK relationship_id       │                   │ FK relationship_id       │
 │ FK category_id           │                   │ FK created_by ─▶ users   │
 │ FK created_by ─▶ users   │                   │ FK status_id             │
 │    title                 │                   │    title                 │
 │    description           │                   │    message               │
 │    memory_date           │                   │    unlock_at (> created) │
 │    created_at/updated_at │                   │    created_at/updated_at │
 └──────┬────────────┬──────┘                   └────────────┬─────────────┘
      1 │            │ 1                                   1 │
        │            │ N                                   N │
        │  ┌─────────┴────────┐                 ┌────────────┴─────────────┐
        │  │ memory_media     │                 │ capsule_media            │
        │  ├──────────────────┤                 ├──────────────────────────┤
        │  │ PK,FK memory_id  │                 │ PK,FK capsule_id         │
        │  │ PK,FK media_id   │                 │ PK,FK media_id           │
        │  └─────────┬────────┘                 └────────────┬─────────────┘
        │            │ N                                   N │
        │            │ 1                                   1 │
        │            │      ┌──────────────────────────┐     │
        │            │      │ media                    │     │
        │            └─────>│ PK id                    │<────┘
        │                   │ FK media_type_id         │
        │ N                 │ FK uploaded_by ─▶ users  │
 ┌──────┴───────────┐       │    storage_path  UQ      │
 │ memory_tags      │       │    file_name             │
 ├──────────────────┤       │    mime_type             │
 │ PK,FK memory_id  │       │    file_size             │
 │ PK,FK tag_id     │       │    created_at            │
 └──────┬───────────┘       └──────────────────────────┘
      N │
      1 │
 ┌──────┴───────────┐
 │ tags             │
 ├──────────────────┤
 │ PK id            │
 │    name UQ(lower)│
 └──────────────────┘
```

## 3. Profile content: child development and measurements

```
                           ┌──────────────────────────┐
                           │ profiles   (CHILD only*) │
                           └──────┬────────────┬──────┘
                                1 │            │ 1
                    ┌─────────────┘            └───────────────┐
                  N │                                        N │
 ┌──────────────────┴───────┐                   ┌──────────────┴───────────┐
 │ development_records      │                   │ measurements             │
 ├──────────────────────────┤                   ├──────────────────────────┤
 │ PK id                    │                   │ PK id                    │
 │ FK profile_id            │                   │ FK profile_id            │
 │ FK created_by ─▶ users   │                   │ FK measurement_type_id   │
 │    record_date           │                   │ FK created_by ─▶ users   │
 │    notes                 │                   │    value  NUMERIC(10,3)  │
 │    created_at            │                   │    measurement_date      │
 └────────────┬─────────────┘                   │    created_at            │
            1 │                                 └──────────────────────────┘
            N │
 ┌────────────┴─────────────┐
 │ development_observations │
 ├──────────────────────────┤
 │ PK id                    │
 │ FK development_record_id │
 │ FK domain_id             │
 │    observation           │
 │    created_at            │
 └──────────────────────────┘
```

\* Only `development_records` is limited to CHILD profiles; `measurements`
accepts any profile. Two triggers enforce the limit: one blocks a development
record on a non-CHILD profile, and the other stops a profile with records from
changing away from CHILD.

## Lookup tables

All are `id SMALLINT PK, name UQ`, seeded in `0005_reference_data`.

```
 profile_types             ─▶ profiles              CHILD, PET, PERSON, OTHER
 relationship_types        ─▶ relationships         PARENT_CHILD, OWNER_PET, FRIEND, FAMILY, PARTNER, OTHER
 relationship_roles        ─▶ relationship_members  OWNER, PARENT, MEMBER, VIEWER
 memory_categories         ─▶ memories              GENERAL, MILESTONE, BIRTHDAY, HOLIDAY, TRAVEL, FIRST_TIME, EVERYDAY
 media_types               ─▶ media                 IMAGE, VIDEO, AUDIO, DOCUMENT
 development_domains       ─▶ dev_observations      PHYSICAL, MOTOR, LANGUAGE, COGNITIVE, SOCIAL_EMOTIONAL
 measurement_types (+unit) ─▶ measurements          HEIGHT cm, WEIGHT kg, HEAD_CIRCUMFERENCE cm
 time_capsule_statuses     ─▶ time_capsules         LOCKED, AVAILABLE, OPENED, CANCELLED
```
