use std::{fs, io::Write, path::Path};

use anyhow::{Context, Result};
use blog_client::{BlogClient, Transport};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "blog-cli", about = "Клиент учебного блога")]
struct Cli {
    #[arg(long, global = true)]
    grpc: bool,
    #[arg(long, global = true)]
    server: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Register {
        #[arg(long)]
        username: String,
        #[arg(long)]
        email: String,
        #[arg(long)]
        password: String,
    },
    Login {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: String,
    },
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        content: String,
    },
    Get {
        #[arg(long)]
        id: i64,
    },
    Update {
        #[arg(long)]
        id: i64,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        content: Option<String>,
    },
    Delete {
        #[arg(long)]
        id: i64,
    },
    List {
        #[arg(long, default_value_t = 10)]
        limit: i64,
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let address = args.server.unwrap_or_else(|| {
        if args.grpc {
            "http://127.0.0.1:50051".into()
        } else {
            "http://127.0.0.1:8080".into()
        }
    });
    let transport = if args.grpc {
        Transport::Grpc(address)
    } else {
        Transport::Http(address)
    };
    let mut client = BlogClient::new(transport).await?;
    let token_path = Path::new(".blog_token");
    if let Ok(token) = fs::read_to_string(token_path) {
        let token = token.trim();
        if !token.is_empty() {
            client.set_token(token.to_string());
        }
    }

    match args.command {
        Commands::Register {
            username,
            email,
            password,
        } => {
            let result = client.register(&username, &email, &password).await?;
            save_token(token_path, &result.token)?;
            println!(
                "Зарегистрирован пользователь {} (id={})",
                result.user.username, result.user.id
            );
        }
        Commands::Login { username, password } => {
            let result = client.login(&username, &password).await?;
            save_token(token_path, &result.token)?;
            println!(
                "Вход выполнен: {} (id={})",
                result.user.username, result.user.id
            );
        }
        Commands::Create { title, content } => {
            let post = client.create_post(&title, &content).await?;
            print_post(&post);
        }
        Commands::Get { id } => print_post(&client.get_post(id).await?),
        Commands::Update { id, title, content } => {
            let existing = client.get_post(id).await?;
            let title = title.unwrap_or(existing.title);
            let content = content.unwrap_or(existing.content);
            print_post(&client.update_post(id, &title, &content).await?)
        }
        Commands::Delete { id } => {
            client.delete_post(id).await?;
            println!("Пост {id} удалён");
        }
        Commands::List { limit, offset } => {
            let list = client.list_posts(limit, offset).await?;
            println!(
                "Всего постов: {} (limit={}, offset={})",
                list.total, list.limit, list.offset
            );
            for post in &list.posts {
                print_post(post);
            }
        }
    }
    Ok(())
}

fn save_token(path: &Path, token: &str) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("не удалось открыть {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(token.as_bytes())
        .with_context(|| format!("не удалось сохранить токен в {}", path.display()))?;
    Ok(())
}

fn print_post(post: &blog_client::Post) {
    println!(
        "#{}: {} (автор {}, {})",
        post.id, post.title, post.author_id, post.created_at
    );
    println!("{}\n", post.content);
}
