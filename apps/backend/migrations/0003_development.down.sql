DROP TABLE measurements;
DROP TABLE measurement_types;
DROP TABLE development_observations;
DROP TRIGGER profiles_keep_child_with_development ON profiles;
DROP FUNCTION profiles_keep_child_with_development();
DROP TABLE development_records;
DROP FUNCTION development_records_require_child();
DROP TABLE development_domains;
