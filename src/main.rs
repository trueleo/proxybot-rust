use std::{
    env::{self, current_dir},
    future::Future,
    net::SocketAddr,
    sync::{Arc, OnceLock},
};

use dotenvy::dotenv;
use sqlite::{Connection, ConnectionThreadSafe};
use tgbot::{
    api::Client,
    handler::{UpdateHandler, WebhookServer},
    types::{
        AllowedUpdate, ChatPeerId, Command, CopyMessage, ForwardMessage, Message,
        MessageReactionUpdated, ReplyTo, SendMessage, SetMessageReaction, SetWebhook, TextEntity,
        TextEntityPosition, Update, UpdateType,
    },
};

mod db;

fn group_id() -> i64 {
    static GROUP_ID: OnceLock<i64> = OnceLock::new();
    *GROUP_ID.get_or_init(|| {
        env::var("GROUP_ID")
            .expect("GROUP_ID is set")
            .parse::<i64>()
            .expect("GROUP_ID is an i64")
    })
}

async fn start(bot: &Client, message: &Message) -> Result<(), anyhow::Error> {
    let user = &message
        .sender
        .get_user()
        .map(|user| user.get_full_name())
        .unwrap_or_default();
    let reply = vec![
        "Hi!",
        user,
        ", Conversation with this bot is send to our admins.",
    ];
    let bold_text = TextEntity::bold(TextEntityPosition {
        offset: reply.iter().take(1).map(|&x| x.len() as u32).sum(),
        length: reply[1].len() as u32,
    });
    let message = SendMessage::new(message.chat.get_id(), reply.into_iter().collect::<String>())
        .with_entities(Some(bold_text));
    bot.execute(message).await?;
    Ok(())
}

async fn help(bot: &Client, message: &Message) -> Result<(), anyhow::Error> {
    let message = SendMessage::new(
        message.chat.get_id(),
        format!("Help! {}", message.chat.get_id()),
    );
    bot.execute(message).await?;
    Ok(())
}

async fn forward_reaction(
    bot: &Client,
    db: &sqlite::ConnectionThreadSafe,
    message: MessageReactionUpdated,
) -> Result<(), anyhow::Error> {
    let Some((user_id, dm_message_id)) = db::get_from_message_id(db, message.message_id)? else {
        return Ok(());
    };
    bot.execute(
        SetMessageReaction::new(user_id, dm_message_id).with_reaction(message.new_reaction),
    )
    .await?;
    Ok(())
}

async fn group_forward(
    bot: &Client,
    db: &sqlite::ConnectionThreadSafe,
    message: Message,
) -> Result<(), anyhow::Error> {
    let Some(ReplyTo::Message(reply_to)) = message.reply_to else {
        return Ok(());
    };
    if message.sender.get_user().is_some_and(|user| user.is_bot) {
        return Ok(());
    }
    let Some((user_id, _)) = db::get_from_message_id(db, reply_to.id)? else {
        return Ok(());
    };
    bot.execute(CopyMessage::new(user_id, group_id(), message.id))
        .await?;
    Ok(())
}

async fn user_forward(
    bot: &Client,
    db: &sqlite::ConnectionThreadSafe,
    message: Message,
) -> Result<(), anyhow::Error> {
    let chat_id = message.chat.get_id();
    let dm_message_id = message.id;

    let ingroup_message = bot
        .execute(ForwardMessage::new(group_id(), chat_id, message.id))
        .await?;

    db::insert_into(
        db,
        db::InsertValues {
            message_id: ingroup_message.id,
            user_id: chat_id.into(),
            dm_message_id,
        },
    )
}

struct Handler {
    client: Arc<Client>,
    db: Arc<sqlite::ConnectionThreadSafe>,
}

impl UpdateHandler for Handler {
    fn handle(&self, update: Update) -> impl Future<Output = ()> + Send {
        let client = Arc::clone(&self.client);
        let db = Arc::clone(&self.db);
        async {
            let res = handle_updates(client, db, update).await;
            if let Err(err) = res {
                log::error!("{}", err.to_string())
            }
        }
    }
}

async fn handle_updates(
    client: Arc<Client>,
    db: Arc<ConnectionThreadSafe>,
    update: Update,
) -> Result<(), anyhow::Error> {
    match update.update_type {
        UpdateType::Message(message) => {
            let chatid = message.chat.get_id();
            if let Ok(command) = Command::try_from(message.clone()) {
                match command.get_name() {
                    "/start" => start(&client, &message).await?,
                    "/help" => help(&client, &message).await?,
                    _ => (),
                }
            } else if chatid == group_id() {
                group_forward(&client, &db, message).await?;
            } else if chatid > ChatPeerId::from(0) {
                user_forward(&client, &db, message).await?;
            }
        }
        UpdateType::MessageReaction(message) => forward_reaction(&client, &db, message).await?,
        _ => (),
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();

    let sqlite = Connection::open_thread_safe(current_dir().unwrap().join("db.db")).unwrap();
    sqlite.execute(db::CREATE_STATEMENT).unwrap();

    let token = env::var("TGBOT_TOKEN").expect("TGBOT_TOKEN is not set");
    let client = Client::new(token.clone()).expect("Failed to create API");

    let webhook_addr = env::var("WEBHOOK_ADDR")
        .expect("WEBHOOK_ADDR is set")
        .parse::<String>()
        .expect("WEBHOOK_ADDR an String");

    let mut webhook = SetWebhook::new(webhook_addr)
        .with_secret_token(token)
        .with_allowed_updates([AllowedUpdate::Message, AllowedUpdate::MessageReaction].into())
        .with_drop_pending_updates(true);

    if let Some(webhook_ip) = env::var("WEBHOOK_IP")
        .ok()
        .map(|value| value.parse::<String>().expect("WEBHOOK_IP an String"))
    {
        webhook = webhook.with_ip_address(webhook_ip)
    }

    if let Some(cert) = env::var("TLS_CERT")
        .ok()
        .map(|value| value.parse::<String>().expect("TLS_CERT an String"))
    {
        webhook = webhook.with_certificate(cert)
    }

    client.execute(webhook).await.unwrap();

    WebhookServer::new(
        "/",
        Handler {
            client: client.into(),
            db: sqlite.into(),
        },
    )
    .run("127.0.0.1:8080".parse::<SocketAddr>().unwrap())
    .await
    .unwrap();
}
