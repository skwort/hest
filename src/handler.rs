/// A message.
pub struct Message {
    pub body: String,
    pub from: String,
}

/// The requested action given a processed message.
pub enum Action {
    /// The handler wants to reply directly to the received message
    Reply(String),
}

// A trait is essentially an interface
pub trait Handler {
    /// Return the name of the handler. Useful for logging.
    fn name(&self) -> &str;

    /// Returns true if this handler wants to process this message
    fn wants(&self, msg: &Message) -> bool;

    /// Process this message into an Action
    fn process(&self, msg: &Message) -> Vec<Action>;
}

/// The message router
pub struct Router {
    /// The dispatch table.
    pub handlers: Vec<Box<dyn Handler>>,
    // The `Box<...>` here is saying that we want to heap allocate.
    // The `dyn Handler` is saying we have some implementer of the Handler
    // trait. This is essentially equivalent to List<IHandler> in C#.
}

impl Router {
    pub fn dispatch(&self, msg: &Message) -> Vec<Action> {
        for handler in &self.handlers {
            if handler.wants(msg) {
                // NOTE: First handler wins for the time being. If we decide
                //       to implement some kind of multicast pattern, then
                //       we could consider collecting Actions instead.
                log::debug!("{} wants message \"{}\"", handler.name(), msg.body);
                return handler.process(msg);
            }
        }
        // Empty return if no handler was available.
        vec![]
    }
}
