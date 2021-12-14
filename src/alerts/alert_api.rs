#![allow(warnings)]

use crate::api::ApiFuture;
use crate::error::Error;
use chrono::offset::Utc;
use chrono::prelude::*;
use lettre::smtp::authentication::Credentials;
use lettre::smtp::extension::ClientId;
use lettre::{ClientSecurity, ClientTlsParameters, Envelope, SendableEmail, SmtpClient, Transport};
use lettre_email::EmailBuilder;
use log::{debug, error, info};
use native_tls::TlsConnector;
use openapi_client::models;

pub trait AlertApi {
    fn send_msteams_msg(
        &self,
        alert: &models::MsTeamsAlertBody,
        payload: &AlertPayload,
    ) -> ApiFuture<()>;
    fn send_email(&self, alert: &models::EmailAlertBody, payload: &AlertPayload) -> ApiFuture<()>;
    fn send_webhook(
        &self,
        alert: &models::WebhookAlertBody,
        payload: &AlertPayload,
    ) -> ApiFuture<()>;
}

pub struct AlertApiImpl {}

impl AlertApiImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl AlertApi for AlertApiImpl {
    fn send_msteams_msg(
        &self,
        _alert: &models::MsTeamsAlertBody,
        _payload: &AlertPayload,
    ) -> ApiFuture<()> {
        Box::pin(async { Err(Error::new("MS Teams alert is not yet implemented")) })
    }

    #[allow(unused_variables)]
    fn send_email(&self, alert: &models::EmailAlertBody, payload: &AlertPayload) -> ApiFuture<()> {
        let alert = alert.clone();
        let payload: AlertPayload = payload.clone();
        Box::pin(async move {
            let (subject, _body) = match payload.status.status {
                models::MonitorStatusIndicator::OK => (
                    format!("[Schnooty] Monitor {} has recovered", payload.monitor_name),
                    format!(
                        "The following monitor has recovered: {}\n",
                        payload.monitor_name
                    ),
                ),
                models::MonitorStatusIndicator::DOWN => (
                    format!("[Schnooty] Monitor {} is DOWN", payload.monitor_name),
                    format!(
                        "The following monitor is now DOWN (failing): {}\n",
                        payload.monitor_name
                    ),
                ),
            };
            let mut email_body = String::new();
            let timestamp: DateTime<Utc> = payload.status.timestamp;

            email_body.push_str(&format!(
                "The following monitor {}: {}\n\n",
                if let models::MonitorStatusIndicator::OK = payload.status.status {
                    "is up"
                } else {
                    "is down"
                },
                payload.monitor_name
            ));
            email_body.push_str(&format!("Got result: {}\n", payload.status.actual_result));
            email_body.push_str(&format!(
                "Expected result: {}\n\n",
                payload.status.expected_result
            ));
            email_body.push_str(&format!("Description: {}\n", payload.status.description));
            email_body.push_str(&format!("Timestamp: {}\n", timestamp));
            email_body.push_str(&format!("Hostname: {}\n", payload.node_info.hostname));
            email_body.push_str(&format!("Platform: {}\n", payload.node_info.platform));
            email_body.push_str(&format!("CPU info: {}\n", payload.node_info.cpu));
            email_body.push_str(&format!("RAM info: {}\n\n", payload.node_info.ram));

            let logs = payload.status.log;

            if logs.len() > 0 {
                email_body.push_str("Monitor log below\n");
                for log in logs.iter() {
                    let timestamp: DateTime<Utc> = log.timestamp;
                    email_body.push_str(&format!("{}: {}\n", timestamp, log.value));
                }
                email_body.push_str("\n\n");
            }

            email_body
                .push_str("You can view your monitors by logging in at www.openmonitors.com\n");

            let (recipients, from, username, password, host, port, tls_mode) = match (
                alert.recipients,
                alert.from,
                alert.username,
                alert.password,
                alert.host,
                alert.port,
                alert.tls_mode,
            ) {
                (
                    Some(recipients),
                    Some(from),
                    Some(username),
                    Some(password),
                    Some(host),
                    Some(port),
                    Some(tls_mode),
                ) => (
                    recipients,
                    from,
                    username,
                    password,
                    host,
                    port as u16,
                    tls_mode,
                ),
                _ => {
                    error!("Error getting alert data. At least one property missing: from, username, password, host, tls_mode");
                    return Err(Error::new(
                        "Error getting alert data. The alert is misconfigured.",
                    ));
                }
            };

            let message_id = format!(
                "https://api.schnooty.com/{}/{}",
                from,
                Utc::now().timestamp().to_string()
            );

            debug!("Email parameter (message_id={})", message_id);
            debug!("Email parameter (from={})", from);

            let mut email_builder = EmailBuilder::new()
                .from(from)
                .subject(subject)
                .body(email_body);

            //let from = from.parse().unwrap();
            //let mut email_recipients = vec![];
            for recipient in recipients {
                email_builder = email_builder.to(recipient); // TODO This may fail if formatted badly
            }

            debug!("Creating client (host={}, tls_mode={:?})", host, tls_mode);

            let address = format!("{}:{}", host, port);
            let client_result = match tls_mode {
                models::TlsMode::NONE => SmtpClient::new(address, ClientSecurity::None),
                models::TlsMode::TLS => {
                    let connector = match TlsConnector::new() {
                        Ok(c) => c,
                        Err(err) => {
                            error!("Error with TLS connector: {}", err);
                            return Err(Error::from(err));
                        }
                    };
                    SmtpClient::new(
                        address,
                        ClientSecurity::Required(ClientTlsParameters {
                            connector,
                            domain: host.clone(),
                        }),
                    )
                }
                models::TlsMode::STARTTLS => {
                    let connector = match TlsConnector::new() {
                        Ok(c) => c,
                        Err(err) => {
                            error!("Error with TLS connector: {}", err);
                            return Err(Error::from(err));
                        }
                    };
                    SmtpClient::new(
                        address,
                        ClientSecurity::Opportunistic(ClientTlsParameters {
                            connector,
                            domain: host.clone(),
                        }),
                    )
                }
            };

            let client = match client_result {
                Ok(c) => c,
                Err(err) => {
                    error!("Error getting SMTP client: {}", err);
                    return Err(Error::from(err));
                }
            };

            debug!("Using credentials (username={})", username);

            let creds = Credentials::new(username, password);

            let mut transport = client
                .credentials(creds)
                .hello_name(ClientId::Domain(host))
                .transport();

            debug!("Sending email");

            let email = match email_builder.build() {
                Ok(e) => e,
                Err(err) => {
                    error!("Error building email: {}", err);
                    return Err(Error::from(err));
                }
            };

            match transport.send(email.into()) {
                Ok(_) => info!("Email sent successfully!"),
                Err(e) => {
                    error!("Failed to send email: {:?}", e);
                    return Err(Error::from(e));
                }
            };

            Ok(())
        })
    }

    fn send_webhook(
        &self,
        _alert: &models::WebhookAlertBody,
        _payload: &AlertPayload,
    ) -> ApiFuture<()> {
        Box::pin(async { Ok(()) })
    }
}

#[derive(Clone, Debug)]
pub struct AlertPayload {
    pub monitor_name: String,
    pub status: models::MonitorStatus,
    pub node_info: NodeInfo,
}

#[derive(Clone, Debug)]
pub struct NodeInfo {
    pub hostname: String,
    pub platform: String,
    pub cpu: String,
    pub ram: String,
}
