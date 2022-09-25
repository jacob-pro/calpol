use crate::database::Test;
use crate::test_runner::database::{NotificationTargets, ProcessedTests};
use crate::state::AppState;
use anyhow::Context;
use futures::future::join_all;
use lettre::message::Mailbox;
use lettre::{AsyncTransport, Message};

const MAX_SMS_CHARS: usize = 70;

pub async fn send_notifications(
    processed: &ProcessedTests,
    targets: NotificationTargets,
    state: &AppState,
) -> anyhow::Result<()> {
    if !processed.now_failing.is_empty() {
        let body = create_sms_failure_body(&processed.now_failing);
        send_sms_notifications(state, targets.sms.clone(), body).await?;
        let body = create_email_failure_body(&processed.now_failing);
        send_email_notifications(state, &targets.emails, &body, "Calpol Test Failures").await?;
    }
    if !processed.now_passing.is_empty() {
        let body = create_sms_passing_body(&processed.now_passing);
        send_sms_notifications(state, targets.sms, body).await?;
        let body = create_email_passing_body(&processed.now_passing);
        send_email_notifications(state, &targets.emails, &body, "Calpol Tests Passing").await?;
    }
    Ok(())
}

async fn send_email_notifications(
    state: &AppState,
    emails: &[Mailbox],
    message: &str,
    subject: &str,
) -> anyhow::Result<()> {
    let results = join_all(emails.iter().map(|email| {
        let message = Message::builder()
            .to(email.clone())
            .from(state.settings.mailer.send_from.clone())
            .reply_to(state.settings.mailer.reply_to().clone())
            .subject(subject.to_string())
            .body(message.to_string())
            .unwrap();
        async move { (state.mailer.send(message).await, email) }
    }))
    .await;
    for (result, mailbox) in results {
        if let Err(e) = result {
            log::error!("Failed to send email to {} because {:#}", mailbox, e);
        } else {
            log::info!("Sent email to {}", mailbox)
        }
    }
    Ok(())
}

async fn send_sms_notifications(
    state: &AppState,
    phone_numbers: Vec<String>,
    message: String,
) -> anyhow::Result<()> {
    let message = if message.len() > MAX_SMS_CHARS {
        format!(
            "{}...",
            message.chars().take(MAX_SMS_CHARS - 3).collect::<String>()
        )
    } else {
        message
    };
    if let Some(message_bird) = &state.message_bird {
        let count = phone_numbers.len();
        let result = message_bird
            .send_message(&message, phone_numbers)
            .await
            .context("Failed sending SMS messages")?;
        log::info!("Sent {} sms messages: {:?}", count, result);
    } else {
        log::error!(
            "Unable to send {} sms notifications because messagebird is not configured",
            phone_numbers.len()
        );
    }
    Ok(())
}

fn create_sms_failure_body(tests: &[(Test, anyhow::Error)]) -> String {
    let mut message = String::from("Calpol: ");
    if tests.len() == 1 {
        let (test, e) = tests.first().unwrap();
        message.push_str(&format!("Test {} failed: {:#}", test.name, e));
    } else {
        let names = tests
            .iter()
            .map(|(t, _)| t.name.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        message.push_str(&format!(
            "{} tests failed, including: {}",
            tests.len(),
            names
        ));
    }
    message
}

fn create_sms_passing_body(tests: &[Test]) -> String {
    let mut message = String::from("Calpol: ");
    if tests.len() == 1 {
        let test = tests.first().unwrap();
        message.push_str(&format!("Test {} now passing", test.name));
    } else {
        let names = tests
            .iter()
            .map(|t| t.name.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        message.push_str(&format!(
            "{} tests passing, including: {}",
            tests.len(),
            names
        ));
    }
    message
}

fn create_email_failure_body(tests: &[(Test, anyhow::Error)]) -> String {
    let mut message = format!("Calpol: {} tests failed\n\n", tests.len());
    for (t, e) in tests {
        message.push_str(&format!("{}: {:#}\n\n", t.name, e));
    }
    message
}

fn create_email_passing_body(tests: &[Test]) -> String {
    let mut message = format!("Calpol: {} tests now passing\n\n", tests.len());
    for t in tests {
        message.push_str(&format!("{}\n", t.name));
    }
    message
}
