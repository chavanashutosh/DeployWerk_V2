//! Transactional SMTP (team invitations, deploy notification emails).

use lettre::message::{Message, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

#[derive(Clone, Debug)]
pub struct SmtpSettings {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    /// Full RFC 5322 From, e.g. `DeployWerk <noreply@example.com>`
    pub from: String,
    pub tls: SmtpTls,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmtpTls {
    Starttls,
    Wrapper,
    None,
}

pub(crate) fn parse_smtp_settings(
    host: Option<String>,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    from: Option<String>,
    tls_mode: Option<String>,
) -> Option<SmtpSettings> {
    let host = host?.trim().to_string();
    if host.is_empty() {
        return None;
    }
    let from = from?.trim().to_string();
    if from.is_empty() {
        return None;
    }
    let tls = match tls_mode
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .as_deref()
    {
        Some("wrapper") | Some("smtps") | Some("implicit") => SmtpTls::Wrapper,
        Some("none") | Some("plain") => SmtpTls::None,
        Some("starttls") => SmtpTls::Starttls,
        _ => {
            if port == 465 {
                SmtpTls::Wrapper
            } else {
                SmtpTls::Starttls
            }
        }
    };
    Some(SmtpSettings {
        host,
        port,
        username: username
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        password: password.map(|s| s.to_string()).filter(|s| !s.is_empty()),
        from,
        tls,
    })
}

fn build_mailer(settings: &SmtpSettings) -> Result<AsyncSmtpTransport<Tokio1Executor>, String> {
    let mut b = match settings.tls {
        SmtpTls::None => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(settings.host.clone())
            .port(settings.port)
            .tls(Tls::None),
        SmtpTls::Starttls => {
            let tls_params =
                TlsParameters::new(settings.host.clone()).map_err(|e| e.to_string())?;
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(settings.host.clone())
                .port(settings.port)
                .tls(Tls::Required(tls_params))
        }
        SmtpTls::Wrapper => {
            let tls_params =
                TlsParameters::new(settings.host.clone()).map_err(|e| e.to_string())?;
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(settings.host.clone())
                .port(settings.port)
                .tls(Tls::Wrapper(tls_params))
        }
    };
    if let (Some(u), Some(p)) = (&settings.username, &settings.password) {
        b = b.credentials(Credentials::new(u.clone(), p.clone()));
    }
    Ok(b.build())
}

/// Returns (subject, plain body) for deploy-style notification payloads.
pub fn deploy_event_email_content(payload: &serde_json::Value) -> (String, String) {
    let event = payload.get("event").and_then(|v| v.as_str()).unwrap_or("event");
    let status = payload.get("status").and_then(|v| v.as_str()).unwrap_or("—");
    let job = payload.get("job_id").and_then(|v| v.as_str()).unwrap_or("");
    let app = payload
        .get("application_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let slug = payload
        .get("application_slug")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let subject = format!("DeployWerk: {event} ({status}) — {app}");
    let body = format!(
        "DeployWerk notification\n\
         \n\
         event: {event}\n\
         status: {status}\n\
         job_id: {job}\n\
         application: {app} ({slug})\n\
         \n\
         ts: {}\n",
        payload.get("ts").and_then(|v| v.as_str()).unwrap_or("")
    );
    (subject, body)
}

pub async fn send_plain_email(
    settings: &SmtpSettings,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    let to = to.trim();
    if to.is_empty() || !to.contains('@') {
        return Err("invalid recipient".into());
    }
    let email = Message::builder()
        .from(settings.from.parse().map_err(|e: lettre::address::AddressError| {
            format!("invalid SMTP from address: {e}")
        })?)
        .to(to
            .parse()
            .map_err(|e: lettre::address::AddressError| format!("invalid recipient: {e}"))?)
        .subject(subject)
        .singlepart(
            SinglePart::builder()
                .header(lettre::message::header::ContentType::TEXT_PLAIN)
                .body(body.to_string()),
        )
        .map_err(|e| format!("build message: {e}"))?;

    let mailer = build_mailer(settings)?;
    mailer
        .send(email)
        .await
        .map_err(|e| format!("smtp send failed: {e}"))?;
    Ok(())
}

pub async fn send_invitation_email(
    settings: &SmtpSettings,
    public_app_base: &str,
    token: &str,
    invitee_email: &str,
    team_name: &str,
    role_label: &str,
) -> Result<(), String> {
    let base = public_app_base.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err("DEPLOYWERK_PUBLIC_APP_URL is not set".into());
    }
    let link = format!("{base}/invite/{token}");
    let subject = format!("You're invited to {team_name} on DeployWerk");
    let body = format!(
        "You've been invited to join the team \"{team_name}\" on DeployWerk as a {role_label}.\n\
         \n\
         Open this link to accept (expires in 14 days):\n\
         {link}\n\
         \n\
         If you did not expect this message, you can ignore it.\n"
    );
    send_plain_email(settings, invitee_email, &subject, &body).await
}

/// Subject and plain body when super-admin access is granted or revoked.
pub fn admin_platform_admin_notice(granted: bool, target_email: &str, actor_email: &str) -> (String, String) {
    let action = if granted {
        "granted super administrator access"
    } else {
        "revoked super administrator access"
    };
    let subject = format!("DeployWerk: {action} ({target_email})");
    let body = format!(
        "DeployWerk instance notification\n\
         \n\
         Your account ({target_email}) had super administrator (operator) access {verb}.\n\
         Performed by: {actor_email}\n\
         \n\
         If this was unexpected, contact your instance operators.\n",
        verb = if granted { "granted" } else { "revoked" },
    );
    (subject, body)
}

/// Subject and body when an operator updates team billing metadata.
pub fn admin_billing_notice(team_name: &str, plan: &str, status: &str, actor_email: &str) -> (String, String) {
    let subject = format!("DeployWerk: billing updated for team \"{team_name}\"");
    let body = format!(
        "DeployWerk instance notification\n\
         \n\
         Billing metadata for team \"{team_name}\" was updated by an operator ({actor_email}).\n\
         Plan label: {plan}\n\
         Status: {status}\n\
         \n\
         For questions, reply to your DeployWerk operator or support contact.\n"
    );
    (subject, body)
}
