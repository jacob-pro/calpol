use crate::database::Test;
use crate::test_runner::database::NotificationTargets;
use crate::AppState;
use anyhow::Context;
use futures::future::join_all;
use lettre::message::Mailbox;
use lettre::{AsyncTransport, Message};

const MAX_SMS_CHARS: usize = 70;

pub async fn send_notifications(
    failed_tests: &[(Test, anyhow::Error)],
    targets: NotificationTargets,
    state: &AppState,
) -> anyhow::Result<()> {
    if failed_tests.is_empty() {
        return Ok(());
    }
    if !targets.sms.is_empty() {
        let body = create_sms_body(failed_tests);
        send_sms_notifications(state, targets.sms, body).await?;
    }
    if !targets.emails.is_empty() {
        let body = create_email_body(failed_tests);
        send_email_notifications(state, targets.emails, body).await?;
    }
    Ok(())
}

async fn send_email_notifications(
    state: &AppState,
    emails: Vec<Mailbox>,
    message: String,
) -> anyhow::Result<()> {
    let results = join_all(emails.into_iter().map(|email| {
        let message = Message::builder()
            .to(email.clone())
            .from(state.settings.mailer.send_from.clone())
            .reply_to(state.settings.mailer.reply_to().clone())
            .subject("Calpol Test Failure")
            .body(message.clone())
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

fn create_sms_body(failed_tests: &[(Test, anyhow::Error)]) -> String {
    let mut message = String::from("Calpol: ");
    if failed_tests.len() == 1 {
        let (test, e) = failed_tests.first().unwrap();
        message.push_str(&format!("Test {} failed: {:#}", test.name, e));
    } else {
        let names = failed_tests
            .iter()
            .map(|(t, _)| t.name.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        message.push_str(&format!(
            "{} tests failed, including: {}",
            failed_tests.len(),
            names
        ));
    }
    if message.len() > MAX_SMS_CHARS {
        format!(
            "{}...",
            message.chars().take(MAX_SMS_CHARS - 3).collect::<String>()
        )
    } else {
        message
    }
}

fn create_email_body(failed_tests: &[(Test, anyhow::Error)]) -> String {
    let mut message = format!("Calpol: {} tests failed\n\n", failed_tests.len());
    for (t, e) in failed_tests {
        message.push_str(&format!("{}: {:#}\n\n", t.name, e));
    }
    message
}
