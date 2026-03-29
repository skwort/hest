use crate::handler::{Action, Handler, Message};

pub struct EchoHandler;

impl Handler for EchoHandler {
    fn name(&self) -> &str {
        "EchoHandler"
    }

    fn wants(&self, _msg: &Message) -> bool {
        true
    }

    fn process(&self, msg: &Message) -> Vec<Action> {
        vec![Action::Reply(format!("echo: {}", msg.body))]
    }
}
