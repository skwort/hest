mod config;
mod handler;
mod handlers;

use config::Config;

use handler::{Action, Message, Router};
use handlers::{EchoHandler, ReminderHandler};

use xmpp::jid::BareJid;
use xmpp::message::send::MessageSettings;
use xmpp::muc::room::JoinRoomSettings;
use xmpp::{Agent, ClientBuilder, ClientType, Event, RoomNick};

use tokio::signal::ctrl_c;
use tokio::time::Duration;

use std::str::FromStr;

#[tokio::main]
async fn main() {
    env_logger::init();

    // Load the config
    let config = config::load().unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });

    let handler_config = &config.handler;

    // XMPP uses TLS; here we setup the crypto provider to enable TLS.
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    // Create the router
    let mut router = Router {
        handlers: Vec::new(),
    };

    // Box the handlers and push them to the router's dispatch table.
    router.handlers.push(Box::new(EchoHandler));

    if handler_config.reminder.enabled {
        log::info!("Creating reminder handler");
        router.handlers.push(Box::new(ReminderHandler::new(
            handler_config
                .reminder
                .data_dir
                .as_ref()
                .expect("data_dir should be defaulted in load()"),
        )));
    }

    // Create the XMPP client
    let jid = BareJid::from_str(&config.transport.xmpp.jid).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });

    let password = config::resolve_password(&config).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });

    let nick = RoomNick::from_str(&config.transport.xmpp.nick).unwrap();
    let mut client = ClientBuilder::new(jid, &password)
        .set_client(ClientType::Bot, &config.transport.xmpp.nick)
        .set_default_nick(nick)
        .build();

    log::info!("Connecting...");

    let mut tick = tokio::time::interval(Duration::from_secs(10));

    // TODO: We should abstract the transport away from main. There's no reason
    //       to couple directly to XMPP. We should instead couple to a Transport
    //       trait. Transports would take a mpsc tx handle on some start/run
    //       method. Then instead of client.wait_for_events(), we just call
    //       recv asgainst our mpsc rx handle. The send path would use a send()
    //       method on the trait.
    loop {
        tokio::select! {
            events = client.wait_for_events() => {
                for event in events {
                    handle_events(&config, &router, &mut client, event).await
                }
            },
            _ = tick.tick() => {
                log::debug!("Handler tick");
                for handler in &router.handlers {
                    for action in handler.tick() {
                        match action {
                            Action::Send { to, body } => {
                                match BareJid::from_str(&to) {
                                    Ok(jid) => {
                                        client.send_message(MessageSettings::new(jid, &body)).await;
                                    }
                                    Err(e) => {
                                        log::error!("Unable to Send message: {}", e);
                                    }
                                }
                            }
                            _ => panic!("Unsupported action returned from tick()"),
                        }
                    }
                }
            }
            _ = ctrl_c() => {
                log::info!("Disconnecting...");
                client.disconnect().await.unwrap();
                break;
            },
        }
    }
}

async fn handle_events(config: &Config, router: &Router, client: &mut Agent, event: Event) {
    match event {
        Event::Online => {
            log::info!("Online.");
            if let Some(rooms) = &config.transport.xmpp.rooms {
                for room in rooms {
                    match BareJid::from_str(room) {
                        Ok(jid) => {
                            client
                                .join_room(JoinRoomSettings {
                                    status: Some((
                                        "en",
                                        config.transport.xmpp.room_status.as_deref().unwrap_or(""),
                                    )),
                                    ..JoinRoomSettings::new(jid.clone())
                                })
                                .await;
                        }
                        Err(e) => {
                            log::error!("Requested room {} is not a valid JID: {e}", room);
                        }
                    }
                }
            }
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
            _ => panic!("Unsupported action returned from dispatch()"),
        }
    }
}
