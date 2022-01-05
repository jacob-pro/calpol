use crate::database::Test;
use crate::mailer::lettre_async;
use crate::settings::{MailerSetting, TwilioSetting};
use crate::state::AppState;
use crate::test_runner::database::NotificationTargets;
use anyhow::Context;
use futures::future::join_all;
use lettre::message::Mailbox;
use lettre::{AsyncTransport, Message};
use twilio::OutboundMessage;

const MAX_SMS_CHARS: usize = 70;

pub async fn send_notifications(
    failed_tests: &Vec<(Test, anyhow::Error)>,
    targets: NotificationTargets,
    state: &AppState,
) -> anyhow::Result<()> {
    if failed_tests.is_empty() {
        return Ok(());
    }
    if !targets.sms.is_empty() {
        if let Some(twilio_setting) = state.settings().twilio.as_ref() {
            let body = create_sms_body(failed_tests);
            send_sms_notifications(targets.sms, body, twilio_setting).await?;
        } else {
            log::error!(
                "Unable to send {} sms notifications because twilio is not configured",
                targets.sms.len()
            );
        }
    }
    if !targets.emails.is_empty() {
        let body = create_email_body(failed_tests);
        send_email_notifications(targets.emails, body, &state.settings().mailer).await?;
    }
    Ok(())
}

async fn send_email_notifications(
    emails: Vec<Mailbox>,
    message: String,
    setting: &MailerSetting,
) -> anyhow::Result<()> {
    let mailer = lettre_async(setting).context("Failed to start mailer")?;
    let results = join_all(emails.into_iter().map(|email| {
        let mailer = &mailer;
        let message = Message::builder()
            .to(email.clone())
            .from(setting.send_from.clone())
            .reply_to(setting.reply_to().clone())
            .subject("Calpol Test Failure")
            .body(message.clone())
            .unwrap();
        async move { (mailer.send(message).await, email) }
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
    phone_numbers: Vec<String>,
    message: String,
    setting: &TwilioSetting,
) -> anyhow::Result<()> {
    let client = setting.new_client();
    let results = join_all(phone_numbers.into_iter().map(|number| {
        let client = &client;
        let message = message.clone();
        async move {
            let from = &setting.send_from.clone();
            let outbound = OutboundMessage::new(&from, &number, &message);
            client.send_message(outbound).await
        }
    }))
    .await;
    for m in results
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .context("Failed sending twilio message")?
    {
        log::info!("Sent sms message to {}, has status {:?}", m.to, m.status)
    }
    Ok(())
}

fn create_sms_body(failed_tests: &Vec<(Test, anyhow::Error)>) -> String {
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

fn create_email_body(failed_tests: &Vec<(Test, anyhow::Error)>) -> String {
    let mut message = format!("Calpol: {} tests failed\n\n", failed_tests.len());
    for (t, e) in failed_tests {
        message.push_str(&format!("{}: {:#}\n\n", t.name, e));
    }
    message
}
