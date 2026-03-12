use clap::{Parser, Subcommand};
use serde_json::json;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use llave_core::config::validate_nif;
use llave_core::error::{LlaveError, Result};
use llave_core::{init_storage, LlaveClient, Config, KeyringStorage, Session};

#[derive(Parser)]
#[command(
    name = "llave",
    about = "CLI tool for Llave authentication (Spain's digital identity system)",
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

    /// Proxy URL (http, https, or socks5). Overrides config file.
    /// Example: socks5://127.0.0.1:1080
    #[arg(long, global = true, env = "LLAVE_PROXY")]
    proxy: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Activate this device with Llave
    Activate {
        /// Your NIF (DNI/NIE number)
        #[arg(short, long)]
        nif: String,

        /// Device password (will be generated if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },

    /// Request a Llave PIN
    Pin,

    /// Check if a NIF is registered in Llave
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

    /// View your Llave account data
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

    /// Listen for pending Llave Móvil authentication requests (polls server)
    Listen {
        /// Polling interval in seconds
        #[arg(short, long, default_value = "5")]
        interval: u64,

        /// Maximum number of polling attempts (0 = unlimited)
        #[arg(short, long, default_value = "60")]
        max_attempts: u32,
    },

    /// Confirm a pending Llave Móvil authentication request
    Confirm {
        /// Llave Móvil token from the pending request
        #[arg(short, long)]
        token: String,

        /// Identity Provider code from the pending request
        #[arg(short = 'i', long)]
        idp_code: String,
    },

    /// Reject a pending Llave Móvil authentication request
    Reject {
        /// Llave Móvil token from the pending request
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

fn output_error(cli: &Cli, err: &LlaveError) {
    if let LlaveError::IpRateLimited { message } = err {
        eprintln!();
        eprintln!("  ╔══════════════════════════════════════════════════════════════╗");
        eprintln!("  ║  IP TEMPORARILY BLOCKED BY AEAT                             ║");
        eprintln!("  ╠══════════════════════════════════════════════════════════════╣");
        eprintln!("  ║                                                              ║");
        eprintln!("  ║  Your IP address has exceeded the maximum number of failed   ║");
        eprintln!("  ║  attempts allowed per day. You can try again tomorrow.       ║");
        eprintln!("  ║                                                              ║");
        eprintln!("  ║  Tip: Use a VPN or proxy to switch IP addresses:             ║");
        eprintln!("  ║    llave --proxy socks5://127.0.0.1:1080 <command>           ║");
        eprintln!("  ║                                                              ║");
        eprintln!("  ╚══════════════════════════════════════════════════════════════╝");
        eprintln!();
        if cli.verbose {
            eprintln!("  Server message: {message}");
            eprintln!();
        }
        return;
    }

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

    let filter = if cli.verbose {
        "llave=debug,llave_core=debug,info"
    } else {
        "llave=info,llave_core=info,warn"
    };
    tracing_subscriber::Registry::default()
        .with(tracing_logfmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| filter.parse().unwrap()),
        )
        .init();

    init_storage(Box::new(KeyringStorage::new()));

    // Set proxy: CLI flag > config file > env var (LLAVE_PROXY handled by clap)
    let proxy = cli.proxy.clone().or_else(|| {
        Config::load().ok().and_then(|c| c.proxy)
    });
    if proxy.is_some() {
        llave_core::set_proxy(proxy);
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
                .unwrap_or_else(|| llave_core::crypto::generate_device_password());

            let client = LlaveClient::new()?;
            let session =
                llave_core::auth::activate_device(&client, &nif, &device_id, &device_password)
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
            let client = LlaveClient::new()?;
            let (pin, ttl) = llave_core::auth::request_pin(&client, &session).await?;

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
                cfg.nif.ok_or(LlaveError::Config(
                    "No NIF configured. Use --nif or run `llave activate` first.".into(),
                ))?
            };

            let client = LlaveClient::new()?;
            let device_id = uuid::Uuid::new_v4().to_string();

            let resp = client.clave_is_nif_activated(&device_id, &nif).await?;

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
            let client = LlaveClient::new()?;
            llave_core::auth::authenticate_dni(&client, &nif, fecha, soporte).await?;

            output(
                cli,
                json!({
                    "nif": nif,
                    "auth_type": "dni_nie_weak",
                    "status": "OK",
                    "message": "Session cookies established",
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
                            "message": "No active session. Run `llave activate` to set up."
                        }),
                    );
                }
            };
        }

        Commands::MyData => {
            let session = Session::load()?;
            let client = LlaveClient::new()?;

            // Mirror the Android app: ClaveIsNifActivatedSv → ClaveCheckMyDataSv.
            let _activated = client
                .clave_is_nif_activated(&session.device_id, &session.nif)
                .await?;

            let resp = client
                .clave_check_my_data(&session.device_id, &session.nif)
                .await?;

            output(
                cli,
                json!({
                    "status": resp.status,
                    "nif": session.nif,
                    "data": resp.respuesta,
                }),
            );
        }

        Commands::History => {
            let session = Session::load()?;
            let client = LlaveClient::new()?;

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
            let client = LlaveClient::new()?;

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
            let client = LlaveClient::new()?;

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
            let client = LlaveClient::new()?;

            if !cli.plain {
                eprintln!("Listening for pending Llave Móvil requests (polling every {interval}s, max {max_attempts} attempts)...");
                eprintln!("Press Ctrl+C to stop.");
            }

            let result =
                llave_core::auth::listen_for_requests(&client, &session, *interval, *max_attempts)
                    .await?;

            output(cli, result);
        }

        Commands::Confirm { token, idp_code } => {
            let session = Session::load()?;
            let client = LlaveClient::new()?;

            let result =
                llave_core::auth::confirm_authentication(&client, &session, token, idp_code)
                    .await?;

            output(cli, result);
        }

        Commands::Reject { token, idp_code } => {
            let session = Session::load()?;
            let client = LlaveClient::new()?;

            let result =
                llave_core::auth::reject_authentication(&client, &session, token, idp_code)
                    .await?;

            output(cli, result);
        }

        Commands::Pending => {
            let session = Session::load()?;
            let client = LlaveClient::new()?;

            let result = llave_core::auth::poll_pending_requests(&client, &session).await?;

            output(cli, result);
        }
    }

    Ok(())
}
