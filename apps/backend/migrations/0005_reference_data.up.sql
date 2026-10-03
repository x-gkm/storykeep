-- Reference data. IDs are fixed so application code can rely on them; add new
-- values in later migrations rather than editing these.

INSERT INTO profile_types (id, name) VALUES
	(1, 'CHILD'),
	(2, 'PET'),
	(3, 'PERSON'),
	(4, 'OTHER');

INSERT INTO relationship_types (id, name) VALUES
	(1, 'PARENT_CHILD'),
	(2, 'OWNER_PET'),
	(3, 'FRIEND'),
	(4, 'FAMILY'),
	(5, 'PARTNER'),
	(6, 'OTHER');

INSERT INTO relationship_roles (id, name) VALUES
	(1, 'OWNER'),
	(2, 'PARENT'),
	(3, 'MEMBER'),
	(4, 'VIEWER');

INSERT INTO memory_categories (id, name) VALUES
	(1, 'GENERAL'),
	(2, 'MILESTONE'),
	(3, 'BIRTHDAY'),
	(4, 'HOLIDAY'),
	(5, 'TRAVEL'),
	(6, 'FIRST_TIME'),
	(7, 'EVERYDAY');

INSERT INTO media_types (id, name) VALUES
	(1, 'IMAGE'),
	(2, 'VIDEO'),
	(3, 'AUDIO'),
	(4, 'DOCUMENT');

INSERT INTO development_domains (id, name) VALUES
	(1, 'PHYSICAL'),
	(2, 'MOTOR'),
	(3, 'LANGUAGE'),
	(4, 'COGNITIVE'),
	(5, 'SOCIAL_EMOTIONAL');

INSERT INTO measurement_types (id, name, unit) VALUES
	(1, 'HEIGHT', 'cm'),
	(2, 'WEIGHT', 'kg'),
	(3, 'HEAD_CIRCUMFERENCE', 'cm');

INSERT INTO time_capsule_statuses (id, name) VALUES
	(1, 'LOCKED'),
	(2, 'AVAILABLE'),
	(3, 'OPENED'),
	(4, 'CANCELLED');
