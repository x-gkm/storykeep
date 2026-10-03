**Relationship Memory Platform**

**Antigravity Development Specification – V1**

*University Web / Backend Project*  
*PostgreSQL \+ REST API \+ HTML/CSS/JavaScript*

# **1\. Project Overview**

The project is a relationship-centered digital memory platform. Its purpose is to allow users to preserve memories, milestones, media, notes and selected structured information belonging to a special relationship in a chronological and private digital space.

The system is not limited to children. A relationship may represent a parent-child relationship, pet-owner relationship, friendship, family relationship, partnership, or another user-defined relationship type.

# **2\. Core Concept**

The central entity is the relationship rather than a specific person or animal. This allows the same application architecture to support different kinds of relationships.

* User: the account that logs into the system.  
* Profile: the person or animal about whom information is stored.  
* Relationship: the connection between users and a profile.  
* Memory: a dated event, note or special moment belonging to a relationship.  
* Media: images, videos, audio or documents attached to memories or future time capsules.  
* Development records: structured data for child profiles; these belong to the profile rather than a relationship.  
* Time capsule: content intentionally locked until a future date.

# **3\. Technology Constraints**

| Layer | Technology / Requirement |
| :---- | :---- |
| Database | PostgreSQL |
| Backend | REST API; Spring Boot is the preferred implementation if the course permits |
| Frontend | HTML \+ CSS \+ JavaScript |
| Communication | HTTP/JSON REST endpoints |
| Authentication | Session or token-based authentication; implementation to be finalized during backend setup |
| File handling | Image/video upload with metadata stored in PostgreSQL; actual storage strategy to be selected during implementation |
| Version control | Git |

# **4\. Scope of Version 1**

The first implementation must focus on a stable, working non-AI application.

* User registration and login.  
* User profile management.  
* Create and manage relationship records.  
* Create profiles for children, pets, people and other supported profile types.  
* Relationship-based timeline.  
* Create, edit, view and delete memories.  
* Memory categories.  
* Photo/media upload and management.  
* Tags for memories.  
* Child development records and measurements.  
* Time capsules with future unlock dates.  
* Basic privacy and authorization rules.  
* Search/filtering where required by the UI.  
* REST API documentation and clean project structure.

AI, LLM, RAG, semantic search and AI-generated summaries are explicitly OUT OF SCOPE for the initial implementation. They may be added later as an independent module after the core system is complete.

# **5\. User and Relationship Model**

A user can participate in one or more relationships. A relationship can have multiple users as members, which allows shared access to a profile and its memories.

## **5.1 Profile Types**

| Example | Profile Type |
| :---- | :---- |
| Child | CHILD |
| Pet | PET |
| Person | PERSON |
| Other | OTHER |

## **5.2 Relationship Types**

| Example | Relationship Type |
| :---- | :---- |
| Parent / child | PARENT\_CHILD |
| Owner / pet | OWNER\_PET |
| Friends | FRIEND |
| Family | FAMILY |
| Partners | PARTNER |
| Other | OTHER |

# **6\. Database Design Principles**

* The database must be designed to at least Third Normal Form (3NF).  
* Do not store repeated descriptive values such as category names, type names or role names directly in transactional tables when a reference table is appropriate.  
* Many-to-many relationships must use junction tables.  
* Do not store multiple photos in columns such as photo1, photo2, photo3.  
* Do not place child-specific development fields directly in the generic memory table.  
* Foreign keys, UNIQUE constraints, NOT NULL constraints and appropriate CHECK constraints must be used.  
* Dates must use appropriate PostgreSQL date/time types.  
* Passwords must never be stored as plaintext.  
* The schema must avoid unnecessary duplication while remaining understandable for a university project.

# **7\. Proposed PostgreSQL Schema**

## **users**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | BIGSERIAL / BIGINT | User identifier |
| email | UNIQUE, NOT NULL | VARCHAR | Login email |
| password\_hash | NOT NULL | VARCHAR | Hashed password |
| first\_name | NOT NULL | VARCHAR | First name |
| last\_name | NOT NULL | VARCHAR | Last name |
| date\_of\_birth |  | DATE | Optional birth date |
| created\_at | NOT NULL | TIMESTAMPTZ | Creation time |
| updated\_at | NOT NULL | TIMESTAMPTZ | Last update |

## **profile\_types**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | SMALLINT | Type identifier |
| name | UNIQUE, NOT NULL | VARCHAR | CHILD, PET, PERSON, OTHER |

## **profiles**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | BIGSERIAL / BIGINT | Profile identifier |
| profile\_type\_id | FK, NOT NULL | SMALLINT | References profile\_types |
| name | NOT NULL | VARCHAR | Profile name |
| date\_of\_birth |  | DATE | Optional |
| created\_at | NOT NULL | TIMESTAMPTZ |  |
| updated\_at | NOT NULL | TIMESTAMPTZ |  |

## **relationship\_types**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | SMALLINT |  |
| name | UNIQUE, NOT NULL | VARCHAR | PARENT\_CHILD, OWNER\_PET, etc. |

## **relationships**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | BIGSERIAL / BIGINT |  |
| profile\_id | FK, NOT NULL | BIGINT | References profiles |
| relationship\_type\_id | FK, NOT NULL | SMALLINT | References relationship\_types |
| started\_at |  | DATE | Optional relationship start date |
| ended\_at |  | DATE | Optional |
| created\_at | NOT NULL | TIMESTAMPTZ |  |

## **relationship\_roles**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | SMALLINT |  |
| name | UNIQUE, NOT NULL | VARCHAR | PARENT, OWNER, MEMBER, VIEWER, etc. |

## **relationship\_members**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | BIGSERIAL / BIGINT |  |
| relationship\_id | FK, NOT NULL | BIGINT |  |
| user\_id | FK, NOT NULL | BIGINT |  |
| role\_id | FK, NOT NULL | SMALLINT |  |
| joined\_at | NOT NULL | TIMESTAMPTZ |  |

## **memory\_categories**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | SMALLINT |  |
| name | UNIQUE, NOT NULL | VARCHAR | TRAVEL, BIRTHDAY, MILESTONE, etc. |

## **memories**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | BIGSERIAL / BIGINT |  |
| relationship\_id | FK, NOT NULL | BIGINT |  |
| category\_id | FK, NOT NULL | SMALLINT |  |
| title | NOT NULL | VARCHAR |  |
| description |  | TEXT |  |
| memory\_date | NOT NULL | DATE | Date of the event |
| created\_by | FK, NOT NULL | BIGINT | References users |
| created\_at | NOT NULL | TIMESTAMPTZ |  |
| updated\_at | NOT NULL | TIMESTAMPTZ |  |

## **media\_types**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | SMALLINT |  |
| name | UNIQUE, NOT NULL | VARCHAR | IMAGE, VIDEO, AUDIO, DOCUMENT |

## **media**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | BIGSERIAL / BIGINT |  |
| media\_type\_id | FK, NOT NULL | SMALLINT |  |
| storage\_path | NOT NULL | TEXT | Path/object key |
| file\_name | NOT NULL | VARCHAR |  |
| mime\_type | NOT NULL | VARCHAR |  |
| file\_size |  | BIGINT | Bytes |
| uploaded\_by | FK, NOT NULL | BIGINT |  |
| created\_at | NOT NULL | TIMESTAMPTZ |  |

## **memory\_media**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| memory\_id | PK/FK | BIGINT |  |
| media\_id | PK/FK | BIGINT |  |

## **tags**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | BIGSERIAL / BIGINT |  |
| name | UNIQUE, NOT NULL | VARCHAR |  |

## **memory\_tags**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| memory\_id | PK/FK | BIGINT |  |
| tag\_id | PK/FK | BIGINT |  |

## **development\_domains**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | SMALLINT |  |
| name | UNIQUE, NOT NULL | VARCHAR | PHYSICAL, LANGUAGE, COGNITIVE, etc. |

## **development\_records**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | BIGSERIAL / BIGINT |  |
| profile\_id | FK, NOT NULL | BIGINT | Intended for CHILD profiles |
| record\_date | NOT NULL | DATE |  |
| created\_by | FK, NOT NULL | BIGINT |  |
| notes |  | TEXT |  |
| created\_at | NOT NULL | TIMESTAMPTZ |  |

## **development\_observations**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | BIGSERIAL / BIGINT |  |
| development\_record\_id | FK, NOT NULL | BIGINT |  |
| domain\_id | FK, NOT NULL | SMALLINT |  |
| observation | NOT NULL | TEXT |  |
| created\_at | NOT NULL | TIMESTAMPTZ |  |

## **measurement\_types**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | SMALLINT |  |
| name | UNIQUE, NOT NULL | VARCHAR | HEIGHT, WEIGHT, etc. |
| unit | NOT NULL | VARCHAR | cm, kg, etc. |

## **measurements**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | BIGSERIAL / BIGINT |  |
| profile\_id | FK, NOT NULL | BIGINT |  |
| measurement\_type\_id | FK, NOT NULL | SMALLINT |  |
| value | NOT NULL | NUMERIC |  |
| measurement\_date | NOT NULL | DATE |  |
| created\_by | FK, NOT NULL | BIGINT |  |
| created\_at | NOT NULL | TIMESTAMPTZ |  |

## **time\_capsule\_statuses**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | SMALLINT |  |
| name | UNIQUE, NOT NULL | VARCHAR | LOCKED, AVAILABLE, OPENED, CANCELLED |

## **time\_capsules**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| id | PK | BIGSERIAL / BIGINT |  |
| relationship\_id | FK, NOT NULL | BIGINT |  |
| created\_by | FK, NOT NULL | BIGINT |  |
| title | NOT NULL | VARCHAR |  |
| message |  | TEXT |  |
| unlock\_at | NOT NULL | TIMESTAMPTZ |  |
| status\_id | FK, NOT NULL | SMALLINT |  |
| created\_at | NOT NULL | TIMESTAMPTZ |  |
| updated\_at | NOT NULL | TIMESTAMPTZ |  |

## **capsule\_media**

| Column | Constraint | Type | Purpose |
| :---- | :---- | :---- | :---- |
| capsule\_id | PK/FK | BIGINT |  |
| media\_id | PK/FK | BIGINT |  |

# **8\. Main Relationships / Cardinalities**

| Relationship | Cardinality | Meaning |
| :---- | :---- | :---- |
| users → relationship\_members | 1:N | A user can participate in many relationships. |
| profiles → relationships | 1:N | A profile can be part of multiple relationships. |
| relationship\_types → relationships | 1:N | Many relationships can share the same type. |
| relationships → relationship\_members | 1:N | A relationship can have multiple members. |
| relationships → memories | 1:N | A relationship can contain many memories. |
| memories ↔ tags | N:M | Implemented with memory\_tags. |
| memories ↔ media | N:M | Implemented with memory\_media. |
| profiles → development\_records | 1:N | A child profile can have many development records. |
| development\_records → development\_observations | 1:N | A record can contain several observations. |
| profiles → measurements | 1:N | A profile can have many dated measurements. |
| relationships → time\_capsules | 1:N | A relationship can contain multiple capsules. |
| time\_capsules ↔ media | N:M | Implemented with capsule\_media. |

# **9\. Important 3NF Notes**

* Type names are stored once in lookup tables such as profile\_types, relationship\_types and media\_types.  
* Category names are not duplicated in every memory row.  
* Many-to-many data is separated into junction tables.  
* A memory does not contain profile name, user name or relationship type as duplicated text.  
* Child development observations are separated from generic memories because they represent a different subject and structure.  
* Measurements are separated from observations because their numeric values have different semantics and constraints.  
* Media metadata is separated from the entities that use media, preventing repeated file information.  
* Composite primary keys in junction tables prevent duplicate associations.

# **10\. Authorization Concept**

Authorization must be based on relationship membership rather than trusting an ID supplied by the frontend. For example, a user must not be able to request /memories/123 and receive a memory merely because the ID exists. The backend must verify that the authenticated user has access to the relationship containing that memory.

* A relationship member may have a role.  
* VIEWER should have read access only.  
* MEMBER/PARENT/OWNER permissions will be finalized during implementation.  
* The frontend must not be treated as the security boundary.  
* Every protected REST endpoint must perform server-side authorization.

# **11\. Child Account Concept**

Initially, a child is represented as a profile created and managed by an authorized parent/guardian account. At a later configurable age, the system may support inviting the person represented by the profile to create or link their own account. This feature should not be implemented until the core authentication and relationship authorization system is stable.

# **12\. Timeline Rules**

* The timeline is generated primarily from relationship memories ordered by memory\_date.  
* Media attached to a memory is displayed as part of that memory.  
* Development records and measurements can have their own profile-specific views and charts.  
* The timeline must support filtering by date and category.  
* The UI should distinguish event date from record creation date.

# **13\. Time Capsule Rules**

* A capsule has an explicit unlock date/time.  
* Before unlock, its protected content must not be returned to unauthorized clients.  
* The backend is responsible for enforcing the lock; hiding the content only in JavaScript is insufficient.  
* After the unlock date, authorized users may view the capsule.  
* Opening status should be tracked.

# **14\. REST API – Initial Endpoint Plan**

| Resource | Example Endpoints | Purpose |
| :---- | :---- | :---- |
| Auth | POST /api/auth/register | Register |
| Auth | POST /api/auth/login | Login |
| Users | GET /api/users/me | Current user |
| Profiles | GET/POST /api/profiles | List/create profiles |
| Profiles | GET/PUT/DELETE /api/profiles/{id} | Manage profile |
| Relationships | GET/POST /api/relationships | List/create relationships |
| Relationships | GET/PUT/DELETE /api/relationships/{id} | Manage relationship |
| Members | POST /api/relationships/{id}/members | Add relationship member |
| Memories | GET/POST /api/relationships/{id}/memories | List/create memories |
| Memories | GET/PUT/DELETE /api/memories/{id} | Manage memory |
| Media | POST /api/memories/{id}/media | Upload media |
| Tags | GET/POST /api/tags | Manage/search tags |
| Development | GET/POST /api/profiles/{id}/development-records | Child development records |
| Measurements | GET/POST /api/profiles/{id}/measurements | Measurements |
| Capsules | GET/POST /api/relationships/{id}/capsules | Time capsules |
| Capsules | GET /api/capsules/{id} | Return only if authorized/unlocked |

These endpoints are a planning baseline, not a requirement to implement every endpoint in the first coding session. The backend should be developed incrementally.

# **15\. Frontend – Initial Screens**

* Login  
* Register  
* Dashboard / My Relationships  
* Create Relationship  
* Relationship Detail / Timeline  
* Create Memory  
* Memory Detail  
* Edit Memory  
* Profile Detail  
* Child Development View  
* Measurements / Charts  
* Time Capsule List  
* Create Time Capsule  
* Profile / Account Settings

Detailed visual design, navigation style, colors, responsive behavior and optional frontend features will be decided in a separate frontend specification after the core backend/database design is accepted.

# **16\. Development Plan for Antigravity**

## **Part 1 – Project Foundation**

* Create repository/project structure.  
* Configure backend project and PostgreSQL connection.  
* Create environment configuration without hard-coding secrets.  
* Configure Git and basic README.

## **Part 2 – Database and Migrations**

* Implement the PostgreSQL schema.  
* Create lookup/reference tables.  
* Create primary keys and foreign keys.  
* Add NOT NULL, UNIQUE and CHECK constraints where appropriate.  
* Use migration tooling.  
* Insert minimal seed/reference data.  
* Verify 3NF-oriented structure.

## **Part 3 – Authentication and Users**

* Implement registration and login.  
* Hash passwords securely.  
* Implement authenticated user retrieval.  
* Protect private endpoints.

## **Part 4 – Profiles and Relationships**

* CRUD for profiles.  
* CRUD for relationships.  
* Relationship membership and roles.  
* Server-side authorization.

## **Part 5 – Memories and Timeline**

* Memory CRUD.  
* Categories.  
* Tags and memory\_tags.  
* Timeline queries.  
* Date/category filtering.

## **Part 6 – Media**

* File upload endpoint.  
* Media metadata persistence.  
* Memory-media relationship.  
* Validation for allowed file types and sizes.  
* Secure access to private media.

## **Part 7 – Child Development**

* Development records.  
* Development observations and domains.  
* Measurement types and measurements.  
* API endpoints for structured development data.  
* Basic charts/data endpoints.  
* Do not implement medical diagnosis or AI evaluation.

## **Part 8 – Time Capsules**

* Create/update/list capsules.  
* Unlock date handling.  
* Backend authorization and lock enforcement.  
* Capsule-media support.  
* Opened status.

## **Part 9 – Initial Frontend Integration**

* Connect HTML/CSS/JavaScript frontend to REST API.  
* Implement login flow.  
* Implement dashboard and relationship navigation.  
* Implement timeline and memory forms.  
* Implement basic profile/development views.  
* Implement time capsule screens.

## **Part 10 – Testing and Cleanup**

* API validation.  
* Authorization tests.  
* Database constraint tests.  
* File upload tests.  
* Time capsule lock tests.  
* Error handling.  
* README and setup instructions.  
* Remove dead code and duplicate logic.

# **17\. Explicitly Deferred Features**

* LLM / AI assistant.  
* RAG over personal memories.  
* AI-generated monthly/yearly summaries.  
* Semantic search.  
* AI-based development interpretation.  
* AI chatbot.  
* Automatic memory generation.  
* Advanced recommendation systems.

These features may be specified later as a separate extension once the non-AI system is fully functional. No initial implementation should create unnecessary dependencies on an LLM provider.

# **18\. Future AI Extension – Concept Only**

If the project is later extended with AI, the AI should primarily summarize and retrieve user-provided memories, not act as a medical or developmental diagnostic authority. The exact architecture will be designed separately.

# **19\. Important Implementation Rules for Antigravity**

* Do not implement all parts at once.  
* Complete and test one Part before starting the next Part.  
* Do not redesign the database schema casually during later parts; propose schema changes first.  
* Keep database migrations versioned.  
* Keep secrets and credentials outside source code.  
* Use DTOs rather than exposing database entities directly through every REST endpoint.  
* Validate input on the backend.  
* Return consistent HTTP status codes and error responses.  
* Do not trust IDs or permissions coming from the frontend.  
* Keep frontend, backend and database responsibilities separated.  
* Do not add AI dependencies during the initial implementation.

# **20\. Acceptance Criteria for V1**

* A user can register and log in.  
* A user can create a profile.  
* A user can create a relationship involving that profile.  
* Authorized members can view the relationship timeline.  
* A user can create, edit and delete memories.  
* A memory can contain multiple media files.  
* Memories can be tagged and categorized.  
* Child profiles can have structured development records and measurements.  
* Measurement data can be retrieved for charting.  
* A user can create a time capsule with a future unlock date.  
* Locked capsule content cannot be accessed before the unlock time.  
* Unauthorized users cannot access private relationship data.  
* The database remains normalized to at least 3NF.  
* The application can be started from a documented setup procedure.

# **21\. Decisions Still Reserved for the Frontend Specification**

* Exact color palette.  
* Typography.  
* Responsive layout.  
* Timeline visual design.  
* Dashboard card design.  
* Relationship creation wizard.  
* Memory creation UI.  
* Gallery layout.  
* Chart style.  
* Navigation/sidebar design.  
* Animations and transitions.  
* Mobile-specific UI behavior.  
* Optional frontend-only features.

**End of V1 Development Specification**