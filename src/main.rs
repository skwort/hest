mod handler;
mod handlers;

use handler::{Action, Message, Router};
use handlers::EchoHandler;

use std::io::{self, BufRead};

fn main() {
    // Create the router
    let mut router = Router {
        handlers: Vec::new(),
    };

    // Create the handler
    let eh = EchoHandler;

    // Box the handler and push it to the router's dispatch table.
    // This takes ownership of eh.
    router.handlers.push(Box::new(eh));

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(text) => text,
            Err(e) => {
                println!("error reading: {}", e);
                continue;
            }
        };

        let msg = Message {
            body: line,
            from: String::from("stdin"),
        };
        for actions in router.dispatch(&msg) {
            match actions {
                Action::Reply(text) => println!("reply: {}", text),
            }
        }
    }
}
