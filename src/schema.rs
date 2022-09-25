table! {
    runner_logs (id) {
        id -> Int4,
        time_started -> Timestamptz,
        time_finished -> Timestamptz,
        success -> Bool,
        failure_reason -> Nullable<Text>,
        tests_passed -> Nullable<Int4>,
        tests_failed -> Nullable<Int4>,
        tests_skipped -> Nullable<Int4>,
    }
}

table! {
    sessions (id) {
        id -> Int4,
        user_id -> Int4,
        token -> Varchar,
        created -> Timestamptz,
        last_used -> Timestamptz,
        last_ip -> Bytea,
        user_agent -> Varchar,
    }
}

table! {
    test_results (id) {
        id -> Int4,
        test_id -> Int4,
        success -> Bool,
        failure_reason -> Nullable<Text>,
        time_started -> Timestamptz,
        time_finished -> Timestamptz,
    }
}

table! {
    tests (id) {
        id -> Int4,
        name -> Varchar,
        enabled -> Bool,
        config -> Jsonb,
        failing -> Bool,
        failure_threshold -> Int4,
    }
}

table! {
    users (id) {
        id -> Int4,
        name -> Varchar,
        email -> Varchar,
        password_hash -> Nullable<Varchar>,
        password_reset_token -> Nullable<Varchar>,
        password_reset_token_creation -> Nullable<Timestamptz>,
        phone_number -> Nullable<Varchar>,
        sms_notifications -> Bool,
        email_notifications -> Bool,
    }
}

joinable!(sessions -> users (user_id));
joinable!(test_results -> users (test_id));

allow_tables_to_appear_in_same_query!(runner_logs, sessions, test_results, tests, users,);
