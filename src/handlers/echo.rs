use crate::handler::{Action, Handler, Message};

const ECHO_HANDLER_PREFIX: &str = "@echo";

pub struct EchoHandler;

impl Handler for EchoHandler {
    fn name(&self) -> &str {
        "EchoHandler"
    }

    fn wants(&self, msg: &Message) -> bool {
        msg.body.starts_with(ECHO_HANDLER_PREFIX)
    }

    fn process(&self, msg: &Message) -> Vec<Action> {
        vec![Action::Reply(format!(
            "echo: {}",
            msg.body.strip_prefix(ECHO_HANDLER_PREFIX).unwrap().trim()
        ))]
    }
}
