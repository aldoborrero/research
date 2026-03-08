use clap::{Parser, Subcommand};
use serde_json::json;

use clave_core::config::validate_nif;
use clave_core::error::{ClaveError, Result};
use clave_core::{ClaveClient, Config, Session};

#[derive(Parser)]
#[command(
    name = "clave",
    about = "CLI tool for Cl@ve AEAT authentication (Spain's digital identity system)",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output plain text instead of JSON
    #[arg(long, global = true)]
    plain: bool,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Activate this device with Cl@ve
    Activate {
        /// Your NIF (DNI/NIE number)
        #[arg(short, long)]
        nif: String,

        /// Device password (will be generated if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },

    /// Request a Cl@ve PIN
    Pin,

    /// Check if a NIF is registered in Cl@ve
    Check {
        /// NIF to check (uses saved NIF if not provided)
        #[arg(short, long)]
        nif: Option<String>,
    },

    /// Authenticate with DNI/NIE + date of birth (weak auth)
    #[command(name = "dni-auth")]
    DniAuth {
        /// NIF (DNI/NIE number)
        #[arg(short, long)]
        nif: String,

        /// Date of validity from your ID card (DD-MM-YYYY)
        #[arg(short, long)]
        fecha: String,

        /// Support number from your ID card
        #[arg(short, long)]
        soporte: String,
    },

    /// Show saved session/account info
    Status,

    /// View your Cl@ve account data
    #[command(name = "my-data")]
    MyData,

    /// View operations history
    History,

    /// Deactivate this device
    Deactivate,

    /// Log out and clear saved credentials
    Logout,

    /// Authenticate via QR code
    Qr {
        /// QR code value
        #[arg(short, long)]
        value: String,
    },

    /// Listen for pending Cl@ve Móvil authentication requests (polls server)
    Listen {
        /// Polling interval in seconds
        #[arg(short, long, default_value = "5")]
        interval: u64,

        /// Maximum number of polling attempts (0 = unlimited)
        #[arg(short, long, default_value = "60")]
        max_attempts: u32,
    },

    /// Confirm a pending Cl@ve Móvil authentication request
    Confirm {
        /// Cl@ve Móvil token from the pending request
        #[arg(short, long)]
        token: String,

        /// Identity Provider code from the pending request
        #[arg(short = 'i', long)]
        idp_code: String,
    },

    /// Reject a pending Cl@ve Móvil authentication request
    Reject {
        /// Cl@ve Móvil token from the pending request
        #[arg(short, long)]
        token: String,

        /// Identity Provider code from the pending request
        #[arg(short = 'i', long)]
        idp_code: String,
    },

    /// Check for pending authentication requests (single poll)
    Pending,
}

fn output(cli: &Cli, json_value: serde_json::Value) {
    if cli.plain {
        match &json_value {
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    match v {
                        serde_json::Value::String(s) => println!("{k}: {s}"),
                        serde_json::Value::Null => println!("{k}: (none)"),
                        other => println!("{k}: {other}"),
                    }
                }
            }
            serde_json::Value::String(s) => println!("{s}"),
            other => println!("{other}"),
        }
    } else {
        println!("{}", serde_json::to_string_pretty(&json_value).unwrap());
    }
}

fn output_error(cli: &Cli, err: &ClaveError) {
    if cli.plain {
        eprintln!("Error: {err}");
    } else {
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "error": err.to_string()
            }))
            .unwrap()
        );
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("clave=debug,clave_core=debug,info")
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter("clave=warn,clave_core=warn")
            .init();
    }

    if let Err(e) = run(&cli).await {
        output_error(&cli, &e);
        std::process::exit(1);
    }
}

async fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Commands::Activate { nif, password } => {
            let nif = validate_nif(nif)?;
            let device_id = uuid::Uuid::new_v4().to_string();
            let device_password = password
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let client = ClaveClient::new()?;
            let session =
                clave_core::auth::activate_device(&client, &nif, &device_id, &device_password)
                    .await?;

            let mut cfg = Config::load()?;
            cfg.nif = Some(nif.clone());
            cfg.device_id = Some(device_id.clone());
            cfg.save()?;

            output(
                cli,
                json!({
                    "status": "activated",
                    "nif": session.nif,
                    "device_id": session.device_id,
                    "created_at": session.created_at,
                }),
            );
        }

        Commands::Pin => {
            let session = Session::load()?;
            let client = ClaveClient::new()?;
            let (pin, ttl) = clave_core::auth::request_pin(&client, &session).await?;

            output(
                cli,
                json!({
                    "pin": pin,
                    "time_to_live_seconds": ttl,
                    "nif": session.nif,
                }),
            );
        }

        Commands::Check { nif } => {
            let nif = if let Some(n) = nif {
                validate_nif(n)?
            } else {
                let cfg = Config::load()?;
                cfg.nif.ok_or(ClaveError::Config(
                    "No NIF configured. Use --nif or run `clave activate` first.".into(),
                ))?
            };

            let client = ClaveClient::new()?;
            let device_id = uuid::Uuid::new_v4().to_string();

            let _starting = client.starting(&device_id, &nif, "").await?;
            let resp = client.is_nif_activated(&device_id, &nif).await?;

            output(
                cli,
                json!({
                    "nif": nif,
                    "status": resp.status,
                    "response": resp.respuesta,
                }),
            );
        }

        Commands::DniAuth {
            nif,
            fecha,
            soporte,
        } => {
            let nif = validate_nif(nif)?;
            let client = ClaveClient::new()?;
            let html = clave_core::auth::authenticate_dni(&client, &nif, fecha, soporte).await?;

            output(
                cli,
                json!({
                    "nif": nif,
                    "auth_type": "dni_nie_weak",
                    "response_length": html.len(),
                    "response_html": if html.len() > 500 { &html[..500] } else { &html },
                }),
            );
        }

        Commands::Status => {
            match Session::load() {
                Ok(session) => {
                    output(
                        cli,
                        json!({
                            "active": true,
                            "nif": session.nif,
                            "device_id": session.device_id,
                            "created_at": session.created_at,
                            "has_firebase_token": session.firebase_token.is_some(),
                        }),
                    );
                }
                Err(_) => {
                    output(
                        cli,
                        json!({
                            "active": false,
                            "message": "No active session. Run `clave activate` to set up."
                        }),
                    );
                }
            };
        }

        Commands::MyData => {
            let session = Session::load()?;
            let client = ClaveClient::new()?;

            let _starting = client
                .starting(&session.device_id, &session.nif, "")
                .await?;

            let resp = client
                .check_my_data(&session.device_id, &session.nif)
                .await?;

            output(
                cli,
                json!({
                    "status": resp.status,
                    "data": resp.respuesta,
                }),
            );
        }

        Commands::History => {
            let session = Session::load()?;
            let client = ClaveClient::new()?;

            let _starting = client
                .starting(&session.device_id, &session.nif, "")
                .await?;

            let resp = client
                .operations_history(
                    &session.device_id,
                    &session.device_password,
                    &session.nif,
                )
                .await?;

            output(
                cli,
                json!({
                    "status": resp.status,
                    "operations": resp.respuesta,
                }),
            );
        }

        Commands::Deactivate => {
            let session = Session::load()?;
            let client = ClaveClient::new()?;

            let _starting = client
                .starting(&session.device_id, &session.nif, "")
                .await?;

            let resp = client
                .deactivate_authentication(&session.device_id, &session.nif)
                .await?;

            Session::delete()?;

            output(
                cli,
                json!({
                    "status": resp.status,
                    "message": "Device deactivated",
                }),
            );
        }

        Commands::Logout => {
            Session::delete()?;
            output(
                cli,
                json!({
                    "status": "logged_out",
                    "message": "Session cleared"
                }),
            );
        }

        Commands::Qr { value } => {
            let session = Session::load()?;
            let client = ClaveClient::new()?;

            let _starting = client
                .starting(&session.device_id, &session.nif, "")
                .await?;

            let resp = client
                .qr_authenticate(&session.device_id, &session.nif, value)
                .await?;

            output(
                cli,
                json!({
                    "status": resp.status,
                    "response": resp.respuesta,
                }),
            );
        }

        Commands::Listen {
            interval,
            max_attempts,
        } => {
            let session = Session::load()?;
            let client = ClaveClient::new()?;

            if !cli.plain {
                eprintln!("Listening for pending Cl@ve Móvil requests (polling every {interval}s, max {max_attempts} attempts)...");
                eprintln!("Press Ctrl+C to stop.");
            }

            let result =
                clave_core::auth::listen_for_requests(&client, &session, *interval, *max_attempts)
                    .await?;

            output(cli, result);
        }

        Commands::Confirm { token, idp_code } => {
            let session = Session::load()?;
            let client = ClaveClient::new()?;

            let result =
                clave_core::auth::confirm_authentication(&client, &session, token, idp_code)
                    .await?;

            output(cli, result);
        }

        Commands::Reject { token, idp_code } => {
            let session = Session::load()?;
            let client = ClaveClient::new()?;

            let result =
                clave_core::auth::reject_authentication(&client, &session, token, idp_code)
                    .await?;

            output(cli, result);
        }

        Commands::Pending => {
            let session = Session::load()?;
            let client = ClaveClient::new()?;

            let result = clave_core::auth::poll_pending_requests(&client, &session).await?;

            output(cli, result);
        }
    }

    Ok(())
}
