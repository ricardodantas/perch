//! Perch - A beautiful terminal social client for Mastodon and Bluesky
#![allow(clippy::uninlined_format_args)]

use anyhow::Result;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use perch::api::SocialApi;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (RUST_LOG=debug for verbose output)
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    // Parse CLI arguments
    match parse_args()? {
        Command::Run => run_tui(),
        Command::Demo => run_demo(),
        Command::Auth { network, instance } => auth_flow(&network, instance.as_deref()).await,
        Command::Post { content, networks } => post_cli(&content, &networks).await,
        Command::Timeline { network, limit } => timeline_cli(network.as_deref(), limit).await,
        Command::Accounts => list_accounts(),
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::Version => {
            print_version();
            Ok(())
        }
    }
}

/// CLI commands
enum Command {
    Run,
    Demo,
    Auth {
        network: String,
        instance: Option<String>,
    },
    Post {
        content: String,
        networks: Vec<String>,
    },
    Timeline {
        network: Option<String>,
        limit: usize,
    },
    Accounts,
    Help,
    Version,
}

fn parse_args() -> Result<Command> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        return Ok(Command::Run);
    }

    match args[1].as_str() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "-v" | "--version" | "version" => Ok(Command::Version),
        "--demo" | "demo" => Ok(Command::Demo),
        
        "auth" => {
            let network = args
                .get(2)
                .ok_or_else(|| anyhow::anyhow!("Missing network (mastodon or bluesky)"))?
                .clone();
            let instance = args.get(3).cloned();
            Ok(Command::Auth { network, instance })
        }
        
        "post" => {
            let content = args
                .get(2)
                .ok_or_else(|| anyhow::anyhow!("Missing post content"))?
                .clone();
            
            // Parse --to flag
            let mut networks = Vec::new();
            let mut i = 3;
            while i < args.len() {
                if args[i] == "--to" || args[i] == "-t" {
                    if let Some(nets) = args.get(i + 1) {
                        networks.extend(nets.split(',').map(String::from));
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            
            // Default to all configured networks
            if networks.is_empty() {
                networks = vec!["mastodon".to_string(), "bluesky".to_string()];
            }
            
            Ok(Command::Post { content, networks })
        }
        
        "timeline" | "tl" => {
            let network = args.get(2).cloned();
            let limit = args
                .iter()
                .position(|a| a == "--limit" || a == "-l")
                .and_then(|i| args.get(i + 1))
                .and_then(|s| s.parse().ok())
                .unwrap_or(20);
            Ok(Command::Timeline { network, limit })
        }
        
        "accounts" => Ok(Command::Accounts),
        
        other => Err(anyhow::anyhow!(
            "Unknown command: {other}\nRun 'perch --help' for usage"
        )),
    }
}

fn print_help() {
    let config_path = perch::Config::default_path()
        .map_or_else(|_| "Unknown".to_string(), |p| p.display().to_string());

    println!(
        r#"{}
🐦 Perch - A beautiful terminal social client

USAGE:
    perch                              Launch TUI
    perch [COMMAND]

COMMANDS:
    auth <network> [instance]          Authenticate with a network
      Examples:
        perch auth mastodon mastodon.social
        perch auth bluesky

    post <content> [OPTIONS]           Post to networks
      Options:
        -t, --to <networks>            Comma-separated networks (default: all)
      Examples:
        perch post "Hello world!"
        perch post "Hello Fediverse!" --to mastodon
        perch post "Hello!" --to mastodon,bluesky

    timeline [network] [OPTIONS]       Show timeline
      Options:
        -l, --limit <n>                Number of posts (default: 20)
      Examples:
        perch timeline
        perch timeline mastodon --limit 50

    accounts                           List configured accounts

OPTIONS:
    -h, --help                         Show this help message
    -v, --version                      Show version information

KEYBINDINGS (TUI):
    Navigation
      j/↓           Move down
      k/↑           Move up
      Tab           Switch panel
      g/G           Jump to top/bottom

    Actions
      n             New post (compose)
      /             Search
      r             Refresh
      o             Open in browser
      l             Like/favorite
      b             Boost/repost

    View
      f             Cycle filter (All/Mastodon/Bluesky)
      t             Change theme
      ?             Help

CONFIG:
    {}

HOMEPAGE:
    https://github.com/ricardodantas/perch
"#,
        perch::LOGO,
        config_path
    );
}

fn print_version() {
    println!("perch {}", perch::VERSION);
}

fn run_tui() -> Result<()> {
    perch::app::run()
}

fn run_demo() -> Result<()> {
    perch::app::run_demo()
}

async fn auth_flow(network: &str, instance: Option<&str>) -> Result<()> {
    match network.to_lowercase().as_str() {
        "mastodon" | "masto" => {
            let instance = instance.ok_or_else(|| {
                anyhow::anyhow!("Mastodon requires an instance URL\nExample: perch auth mastodon mastodon.social")
            })?;
            
            let instance = if instance.starts_with("http") {
                instance.to_string()
            } else {
                format!("https://{}", instance)
            };
            
            println!("🐘 Authenticating with Mastodon ({})...", instance);
            
            // Register app
            let app = perch::api::mastodon::oauth::register_app(&instance).await?;
            println!("✓ App registered");
            
            // Store client credentials
            perch::auth::store_oauth_client(&instance, &app.client_id, &app.client_secret)?;
            
            // Get auth URL
            let auth_url = perch::api::mastodon::oauth::get_auth_url(&instance, &app.client_id);
            println!("\n📋 Open this URL in your browser:\n\n  {}\n", auth_url);
            
            // Try to open browser
            let _ = open::that(&auth_url);
            
            println!("Paste the authorization code here:");
            let mut code = String::new();
            std::io::stdin().read_line(&mut code)?;
            let code = code.trim();
            
            // Exchange for token
            let token = perch::api::mastodon::oauth::get_token(
                &instance,
                &app.client_id,
                &app.client_secret,
                code,
            )
            .await?;
            
            // Verify and get account info
            let client = perch::api::mastodon::MastodonClient::new(&instance, &token.access_token);
            let account_info = client.verify_credentials().await?;
            
            // Create and store account
            let mut account = perch::Account::new_mastodon(
                &account_info.handle,
                &instance,
                &account_info.display_name,
            );
            account.avatar_url = account_info.avatar_url;
            
            let db = perch::Database::open()?;
            db.insert_account(&account)?;
            
            // Store token
            perch::auth::store_credentials(&account, &token.access_token)?;
            
            println!("\n✓ Logged in as @{}", account_info.handle);
            println!("✓ Account saved");
        }
        
        "bluesky" | "bsky" => {
            println!("🦋 Authenticating with Bluesky...");
            println!("\nEnter your handle (e.g., you.bsky.social):");
            let mut handle = String::new();
            std::io::stdin().read_line(&mut handle)?;
            let handle = handle.trim();
            
            // Ask for PDS URL (optional)
            println!("\nEnter your PDS URL (press Enter for default bsky.social):");
            let mut pds_input = String::new();
            std::io::stdin().read_line(&mut pds_input)?;
            let pds_url = pds_input.trim();
            let pds_url = if pds_url.is_empty() {
                perch::api::bluesky::DEFAULT_PDS_URL
            } else if pds_url.starts_with("http") {
                pds_url
            } else {
                &format!("https://{}", pds_url)
            };
            
            println!("\nEnter your app password:");
            println!("(Create one at https://bsky.app/settings/app-passwords)");
            let mut password = String::new();
            std::io::stdin().read_line(&mut password)?;
            let password = password.trim();
            
            // Login and verify
            let client = perch::api::bluesky::BlueskyClient::login_with_pds(handle, password, pds_url).await?;
            let account_info = client.verify_credentials().await?;
            
            // Create and store account with the actual PDS URL
            let mut account = perch::Account::new_bluesky_with_pds(
                &account_info.handle,
                &account_info.display_name,
                pds_url,
            );
            account.avatar_url = account_info.avatar_url;
            
            let db = perch::Database::open()?;
            db.insert_account(&account)?;
            
            // Store app password
            perch::auth::store_credentials(&account, password)?;
            
            println!("\n✓ Logged in as @{}", account_info.handle);
            println!("✓ Account saved (PDS: {})", pds_url);
        }
        
        _ => {
            return Err(anyhow::anyhow!(
                "Unknown network: {}\nSupported: mastodon, bluesky",
                network
            ));
        }
    }
    
    Ok(())
}

async fn post_cli(content: &str, networks: &[String]) -> Result<()> {
    let db = perch::Database::open()?;
    
    for network_name in networks {
        let network = perch::Network::from_str(network_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown network: {}", network_name))?;
        
        let account = db
            .get_default_account(network)?
            .ok_or_else(|| anyhow::anyhow!("No {} account configured. Run: perch auth {}", network.name(), network_name))?;
        
        let token = perch::auth::get_credentials(&account)?
            .ok_or_else(|| anyhow::anyhow!("No credentials found for {}", account.handle))?;
        
        let client = perch::api::get_client(&account, &token).await?;
        
        println!("{} Posting to {}...", network.emoji(), network.name());
        let post = client.post(content).await?;
        
        if let Some(url) = &post.url {
            println!("✓ Posted: {}", url);
        } else {
            println!("✓ Posted successfully");
        }
    }
    
    Ok(())
}

async fn timeline_cli(network: Option<&str>, limit: usize) -> Result<()> {
    let db = perch::Database::open()?;
    
    let networks: Vec<perch::Network> = if let Some(name) = network {
        vec![perch::Network::from_str(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown network: {}", name))?]
    } else {
        perch::Network::all().to_vec()
    };
    
    for network in networks {
        let Some(account) = db.get_default_account(network)? else {
            continue;
        };
        
        let Some(token) = perch::auth::get_credentials(&account)? else {
            continue;
        };
        
        let client = perch::api::get_client(&account, &token).await?;
        
        println!("\n{} {} Timeline (@{})", network.emoji(), network.name(), account.handle);
        println!("{}", "─".repeat(60));
        
        let posts = client.timeline(limit).await?;
        
        for post in posts {
            println!(
                "\n@{} · {}",
                post.author_handle,
                post.relative_time()
            );
            println!("{}", post.content);
            println!(
                "♥ {}  🔁 {}  💬 {}",
                post.like_count, post.repost_count, post.reply_count
            );
        }
    }
    
    Ok(())
}

fn list_accounts() -> Result<()> {
    let db = perch::Database::open()?;
    let accounts = db.get_accounts()?;
    
    if accounts.is_empty() {
        println!("No accounts configured.");
        println!("\nAdd an account with:");
        println!("  perch auth mastodon <instance>");
        println!("  perch auth bluesky");
        return Ok(());
    }
    
    println!("Configured accounts:\n");
    
    for account in accounts {
        let default_marker = if account.is_default { " (default)" } else { "" };
        println!(
            "  {} {} @{}{}\n    Server: {}",
            account.network.emoji(),
            account.display_name,
            account.handle,
            default_marker,
            account.server
        );
    }
    
    Ok(())
}
