//! Perch - A beautiful terminal social client for Mastodon and Bluesky
#![allow(clippy::uninlined_format_args)]

use anyhow::Result;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use perch::api::SocialApi;

fn main() -> Result<()> {
    // Initialize logging (RUST_LOG=debug for verbose output)
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    // Parse CLI arguments
    match parse_args()? {
        Command::Run => run_tui(),
        Command::Demo => run_demo(),
        Command::Auth { network, instance } => {
            tokio::runtime::Runtime::new()?.block_on(auth_flow(&network, instance.as_deref()))
        }
        Command::Post {
            content,
            networks,
            schedule,
        } => tokio::runtime::Runtime::new()?.block_on(post_cli(
            &content,
            &networks,
            schedule.as_deref(),
        )),
        Command::Schedule { subcommand } => {
            tokio::runtime::Runtime::new()?.block_on(schedule_cli(subcommand))
        }
        Command::Timeline { network, limit } => {
            tokio::runtime::Runtime::new()?.block_on(timeline_cli(network.as_deref(), limit))
        }
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
        schedule: Option<String>,
    },
    Schedule {
        subcommand: ScheduleSubcommand,
    },
    Timeline {
        network: Option<String>,
        limit: usize,
    },
    Accounts,
    Help,
    Version,
}

/// Schedule subcommands
enum ScheduleSubcommand {
    List,
    Cancel { id: String },
    Run,
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

            // Parse flags
            let mut networks = Vec::new();
            let mut schedule = None;
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--to" | "-t" => {
                        if let Some(nets) = args.get(i + 1) {
                            networks.extend(nets.split(',').map(String::from));
                        }
                        i += 2;
                    }
                    "--schedule" | "-s" | "--at" => {
                        if let Some(time) = args.get(i + 1) {
                            schedule = Some(time.clone());
                        }
                        i += 2;
                    }
                    _ => i += 1,
                }
            }

            // Default to all configured networks
            if networks.is_empty() {
                networks = vec!["mastodon".to_string(), "bluesky".to_string()];
            }

            Ok(Command::Post {
                content,
                networks,
                schedule,
            })
        }

        "schedule" | "scheduled" => {
            let subcommand = match args.get(2).map(String::as_str) {
                Some("list" | "ls") | None => ScheduleSubcommand::List,
                Some("cancel" | "rm" | "delete") => {
                    let id = args
                        .get(3)
                        .ok_or_else(|| anyhow::anyhow!("Missing post ID to cancel"))?
                        .clone();
                    ScheduleSubcommand::Cancel { id }
                }
                Some("run" | "process") => ScheduleSubcommand::Run,
                Some(other) => {
                    return Err(anyhow::anyhow!(
                        "Unknown schedule subcommand: {}\nTry: list, cancel, run",
                        other
                    ));
                }
            };
            Ok(Command::Schedule { subcommand })
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
        -s, --schedule <time>          Schedule post for later
      Examples:
        perch post "Hello world!"
        perch post "Hello Fediverse!" --to mastodon
        perch post "Hello!" --to mastodon,bluesky
        perch post "Good morning!" --schedule "in 2h"
        perch post "Scheduled!" --schedule "2026-02-08 15:00"

    schedule [SUBCOMMAND]              Manage scheduled posts
      Subcommands:
        list                           List pending scheduled posts
        cancel <id>                    Cancel a scheduled post
        run                            Process due scheduled posts
      Examples:
        perch schedule list
        perch schedule cancel abc123
        perch schedule run

    timeline [network] [OPTIONS]       Show timeline
      Options:
        -l, --limit <n>                Number of posts (default: 20)
      Examples:
        perch timeline
        perch timeline mastodon --limit 50

    accounts                           List configured accounts

SCHEDULE TIME FORMATS:
    Relative:    "in 5m", "in 2h", "in 1d", "in 30 minutes"
    Time today:  "15:00", "3pm" (schedules for tomorrow if past)
    Date+time:   "2026-02-08 15:00", "2026-02-08T15:00"

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
            let client =
                perch::api::bluesky::BlueskyClient::login_with_pds(handle, password, pds_url)
                    .await?;
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

async fn post_cli(content: &str, networks: &[String], schedule: Option<&str>) -> Result<()> {
    let db = perch::Database::open()?;

    // Parse networks
    let parsed_networks: Vec<perch::Network> = networks
        .iter()
        .filter_map(|n| perch::Network::from_str(n))
        .collect();

    if parsed_networks.is_empty() {
        return Err(anyhow::anyhow!("No valid networks specified"));
    }

    // If scheduling, save to database instead of posting
    if let Some(schedule_time) = schedule {
        let scheduled_for = perch::schedule::parse_schedule_time(schedule_time)?;
        let scheduled_post =
            perch::ScheduledPost::new(content, parsed_networks.clone(), scheduled_for);

        db.save_scheduled_post(&scheduled_post)?;

        let networks_str = parsed_networks
            .iter()
            .map(perch::Network::name)
            .collect::<Vec<_>>()
            .join(", ");

        println!("📅 Post scheduled!");
        println!("   Networks: {}", networks_str);
        println!("   Time: {}", scheduled_post.scheduled_time_display());
        println!("   In: {}", scheduled_post.time_until());
        println!("\n   ID: {}", &scheduled_post.id.to_string()[..8]);
        println!("\n   Run 'perch schedule list' to see pending posts");
        println!("   Run 'perch schedule run' to process due posts");

        return Ok(());
    }

    // Post immediately
    for network in &parsed_networks {
        let account = db.get_default_account(*network)?.ok_or_else(|| {
            anyhow::anyhow!(
                "No {} account configured. Run: perch auth {}",
                network.name(),
                format!("{:?}", network).to_lowercase()
            )
        })?;

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

/// Handle schedule subcommands
async fn schedule_cli(subcommand: ScheduleSubcommand) -> Result<()> {
    let db = perch::Database::open()?;

    match subcommand {
        ScheduleSubcommand::List => {
            let posts = db.get_pending_scheduled_posts()?;

            if posts.is_empty() {
                println!("No scheduled posts.");
                println!("\nSchedule a post with:");
                println!("  perch post \"Your message\" --schedule \"in 2h\"");
                return Ok(());
            }

            println!("📅 Scheduled Posts\n");

            for post in posts {
                let networks_str = post
                    .networks
                    .iter()
                    .map(|n| format!("{} {}", n.emoji(), n.name()))
                    .collect::<Vec<_>>()
                    .join(", ");

                let id_short = &post.id.to_string()[..8];

                println!("{} [{}]", post.status.emoji(), id_short);
                println!("   \"{}\"", truncate_content(&post.content, 60));
                println!("   To: {}", networks_str);
                println!(
                    "   At: {} (in {})",
                    post.scheduled_time_display(),
                    post.time_until()
                );
                println!();
            }
        }

        ScheduleSubcommand::Cancel { id } => {
            // Find post by ID prefix
            let posts = db.get_pending_scheduled_posts()?;
            let matching: Vec<_> = posts
                .iter()
                .filter(|p| p.id.to_string().starts_with(&id))
                .collect();

            match matching.len() {
                0 => {
                    return Err(anyhow::anyhow!(
                        "No scheduled post found with ID starting with '{}'",
                        id
                    ));
                }
                1 => {
                    let post = matching[0];
                    db.cancel_scheduled_post(post.id)?;
                    println!("🚫 Cancelled scheduled post: {}", &post.id.to_string()[..8]);
                    println!("   \"{}\"", truncate_content(&post.content, 50));
                }
                _ => {
                    println!("Multiple posts match '{}'. Please be more specific:", id);
                    for post in matching {
                        println!(
                            "  {} - \"{}\"",
                            &post.id.to_string()[..8],
                            truncate_content(&post.content, 40)
                        );
                    }
                }
            }
        }

        ScheduleSubcommand::Run => {
            let due_posts = db.get_due_scheduled_posts()?;

            if due_posts.is_empty() {
                println!("No scheduled posts are due.");
                return Ok(());
            }

            println!("📤 Processing {} scheduled post(s)...\n", due_posts.len());

            for post in due_posts {
                let id_short = &post.id.to_string()[..8];
                println!(
                    "Processing [{}]: \"{}\"",
                    id_short,
                    truncate_content(&post.content, 40)
                );

                // Mark as posting
                db.update_scheduled_post_status(
                    post.id,
                    perch::ScheduledPostStatus::Posting,
                    None,
                )?;

                let mut success = true;
                let mut error_msg = String::new();

                for network in &post.networks {
                    let account = if let Some(a) = db.get_default_account(*network)? {
                        a
                    } else {
                        let msg = format!("No {} account configured", network.name());
                        println!("  ⚠️  {}", msg);
                        error_msg = msg;
                        success = false;
                        continue;
                    };

                    let token = if let Some(t) = perch::auth::get_credentials(&account)? {
                        t
                    } else {
                        let msg = format!("No credentials for {}", account.handle);
                        println!("  ⚠️  {}", msg);
                        error_msg = msg;
                        success = false;
                        continue;
                    };

                    match perch::api::get_client(&account, &token).await {
                        Ok(client) => match client.post(&post.content).await {
                            Ok(posted) => {
                                if let Some(url) = &posted.url {
                                    println!(
                                        "  {} ✓ Posted to {}: {}",
                                        network.emoji(),
                                        network.name(),
                                        url
                                    );
                                } else {
                                    println!(
                                        "  {} ✓ Posted to {}",
                                        network.emoji(),
                                        network.name()
                                    );
                                }
                            }
                            Err(e) => {
                                let msg = format!("Failed to post to {}: {}", network.name(), e);
                                println!("  {} ✗ {}", network.emoji(), msg);
                                error_msg = msg;
                                success = false;
                            }
                        },
                        Err(e) => {
                            let msg = format!("Failed to connect to {}: {}", network.name(), e);
                            println!("  {} ✗ {}", network.emoji(), msg);
                            error_msg = msg;
                            success = false;
                        }
                    }
                }

                // Update status
                if success {
                    db.update_scheduled_post_status(
                        post.id,
                        perch::ScheduledPostStatus::Posted,
                        None,
                    )?;
                    println!("  ✅ Done\n");
                } else {
                    db.update_scheduled_post_status(
                        post.id,
                        perch::ScheduledPostStatus::Failed,
                        Some(&error_msg),
                    )?;
                    println!("  ❌ Failed\n");
                }
            }
        }
    }

    Ok(())
}

/// Truncate content for display
fn truncate_content(content: &str, max_len: usize) -> String {
    let content = content.replace('\n', " ");
    if content.len() <= max_len {
        content
    } else {
        format!("{}...", &content[..max_len - 3])
    }
}

async fn timeline_cli(network: Option<&str>, limit: usize) -> Result<()> {
    let db = perch::Database::open()?;

    let networks: Vec<perch::Network> = if let Some(name) = network {
        vec![
            perch::Network::from_str(name)
                .ok_or_else(|| anyhow::anyhow!("Unknown network: {}", name))?,
        ]
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

        println!(
            "\n{} {} Timeline (@{})",
            network.emoji(),
            network.name(),
            account.handle
        );
        println!("{}", "─".repeat(60));

        let posts = client.timeline(limit).await?;

        for post in posts {
            println!("\n@{} · {}", post.author_handle, post.relative_time());
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
        let has_creds = perch::auth::has_credentials(&account);
        let cred_status = if has_creds {
            "✓"
        } else {
            "✗ no credentials"
        };

        println!(
            "  {} {} @{}{}\n    Server: {}\n    Auth: {} (key: {})",
            account.network.emoji(),
            account.display_name,
            account.handle,
            default_marker,
            account.server,
            cred_status,
            account.keyring_key()
        );
    }

    Ok(())
}
