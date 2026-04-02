mod handler;
mod handlers;

use handler::{Action, Message, Router};
use handlers::EchoHandler;

use xmpp::jid::BareJid;
use xmpp::message::send::MessageSettings;
use xmpp::muc::room::JoinRoomSettings;
use xmpp::{Agent, ClientBuilder, ClientType, Event, RoomNick};

use tokio::signal::ctrl_c;

use std::str::FromStr;

#[tokio::main]
async fn main() {
    env_logger::init();

    // XMPP uses TLS; here we setup the crypto provider to enable TLS.
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    // Create the router
    let mut router = Router {
        handlers: Vec::new(),
    };

    // Create the handler
    let eh = EchoHandler;

    // Box the handler and push it to the router's dispatch table.
    // This takes ownership of eh.
    router.handlers.push(Box::new(eh));

    // Create the XMPP client
    let jid = BareJid::from_str("hest@xmpp.skwort.dev").unwrap();
    let password = "default";
    let nick = RoomNick::from_str("hest").unwrap();
    let mut client = ClientBuilder::new(jid, password)
        .set_client(ClientType::Bot, "hest")
        .set_default_nick(nick)
        .build();

    log::info!("Connecting...");

    loop {
        tokio::select! {
            events = client.wait_for_events() => {
                for event in events {
                    handle_events(&router, &mut client, event).await
                }
            },
            _ = ctrl_c() => {
                log::info!("Disconnecting...");
                client.disconnect().await.unwrap();
                break;
            },
        }
    }
}

async fn handle_events(router: &Router, client: &mut Agent, event: Event) {
    match event {
        Event::Online => {
            log::info!("Online.");
            log::info!("Joining Home");
            let room = BareJid::from_str("home@conference.xmpp.skwort.dev").unwrap();
            client
                .join_room(JoinRoomSettings {
                    status: Some(("en", "At your behest.")),
                    ..JoinRoomSettings::new(room.clone())
                })
                .await;
        }
        Event::Disconnected(e) => {
            log::info!("Disconnected: {}.", e);
        }
        Event::ChatMessage(_id, jid, body, time_info) => {
            log::info!(
                "{} {}: {}",
                time_info.received.time().format("%H:%M"),
                jid,
                body
            );

            // We can skip replayed messages using:
            // if !time_info.delays.is_empty() {
            //    return;
            // }
            handle_message(router, client, body, jid).await;
        }
        Event::RoomMessage(_id, jid, nick, body, time_info) => {
            // Skip any replayed messages
            if !time_info.delays.is_empty() {
                return;
            }
            println!(
                "Message in room {} from {} at {}: {}",
                jid, nick, time_info.received, body
            );
        }
        _ => {
            log::debug!("Unimplemented event:\n{:#?}", event);
        }
    }
}

async fn handle_message(router: &Router, client: &mut Agent, body: String, from: BareJid) {
    let msg = Message {
        body,
        from: from.to_string(),
    };
    for actions in router.dispatch(&msg) {
        match actions {
            Action::Reply(text) => {
                client
                    .send_message(MessageSettings::new(from.clone(), &text))
                    .await
            }
        };
    }
}
