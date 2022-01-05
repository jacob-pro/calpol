-- Your SQL goes here
CREATE TABLE users
(
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NULL,
    password_reset_token VARCHAR(255) NULL UNIQUE,
    password_reset_token_creation TIMESTAMPTZ NULL,
    phone_number VARCHAR(255) NULL,
    sms_notifications BOOLEAN NOT NULL,
    email_notifications BOOLEAN NOT NULL
);

CREATE TABLE sessions
(
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    token VARCHAR(255) NOT NULL UNIQUE,
    created TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_used TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_ip BYTEA NOT NULL,         -- Bincode Serialized IpAddr
    user_agent VARCHAR(512) NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE tests
(
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    enabled BOOLEAN NOT NULL,
    config JSONB NOT NULL,
    failing BOOLEAN NOT NULL,
    failure_threshold INT NOT NULL CHECK (failure_threshold > 0)
);

CREATE TABLE test_results
(
    id SERIAL PRIMARY KEY,
    test_id INT NOT NULL,
    success BOOLEAN NOT NULL,
    failure_reason TEXT NULL,
    time_started TIMESTAMPTZ NOT NULL,
    time_finished TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (test_id) REFERENCES tests (id)
);

CREATE TABLE runner_logs
(
    id SERIAL PRIMARY KEY,
    time_started TIMESTAMPTZ NOT NULL,
    time_finished TIMESTAMPTZ NOT NULL,
    success BOOLEAN NOT NULL,
    failure_reason TEXT NULL,
    tests_passed INT NULL,
    tests_failed INT NULL,
    tests_skipped INT NULL
)
