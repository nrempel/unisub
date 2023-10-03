CREATE TABLE topics
(
    id   SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE
);

CREATE TYPE message_status AS ENUM ('new', 'processing', 'processed');

CREATE TABLE messages
(
    id           SERIAL PRIMARY KEY,
    topic_id     INTEGER REFERENCES topics (id),
    content      BYTEA NOT NULL,
    status       message_status           DEFAULT 'new',
    published_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE
OR REPLACE FUNCTION notify_new_message() RETURNS TRIGGER AS $$
BEGIN
    PERFORM
pg_notify('new_message', NEW.id::text);
RETURN NEW;
END;
$$
LANGUAGE plpgsql;

CREATE TRIGGER new_message_notify
    AFTER INSERT
    ON messages
    FOR EACH ROW EXECUTE PROCEDURE notify_new_message();