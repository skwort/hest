use crate::handler::{Action, Handler, Message};

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;
use std::fs::OpenOptions;
use std::io::Write as IoWrite;

use chrono::{DateTime, FixedOffset, Local, NaiveDate, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};

use rand::RngExt;
use rand::distr::Alphanumeric;

const REMINDER_HANDLER_PREFIX: &str = "@reminder";

/// Each reminder is either a true Reminder or a Tombstone (deleted reminder)
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReminderRecord {
    Reminder(Reminder),
    Tombstone { id: String, deleted: bool },
}

#[derive(Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub due: DateTime<FixedOffset>,
    pub to: String,
    pub message: String,
    pub cron: Option<String>,
}

impl Reminder {
    pub fn new(
        due: DateTime<FixedOffset>,
        message: String,
        to: String,
        cron: Option<String>,
    ) -> Result<Self, String> {
        let mut rng = rand::rng();
        let id: String = (0..7).map(|_| rng.sample(Alphanumeric) as char).collect();

        Ok(Self {
            id,
            due,
            message,
            to,
            cron,
        })
    }

    pub fn tombstone(&self) -> ReminderRecord {
        ReminderRecord::Tombstone {
            id: self.id.clone(),
            deleted: true,
        }
    }
}

struct ReminderStore {
    path: String,
    reminders: HashMap<String, Reminder>,
}

impl ReminderStore {
    pub fn load(path: &str) -> Self {
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => {
                return Self {
                    path: path.to_string(),
                    reminders: HashMap::new(),
                };
            }
        };

        let mut reminders: HashMap<String, Reminder> = HashMap::new();
        let mut count = 0;

        for line in contents.lines() {
            if let Ok(record) = serde_json::from_str::<ReminderRecord>(line) {
                match record {
                    ReminderRecord::Reminder(r) => {
                        reminders.insert(r.id.clone(), r);
                        count += 1;
                    }
                    ReminderRecord::Tombstone { id, .. } => {
                        reminders.remove(&id);
                        count -= 1;
                    }
                }
            }
        }

        log::info!("Loaded {} reminders from disk", count);

        Self {
            path: path.to_string(),
            reminders,
        }
    }

    pub fn add(&mut self, reminder: Reminder) -> Result<(), String> {
        if self.reminders.contains_key(&reminder.id) {
            return Err(format!("reminder for {} already exists", &reminder.id));
        }

        self.append(&reminder)?;
        log::info!("Reminder {} \"{}\" added", &reminder.id, &reminder.message);
        self.reminders.insert(reminder.id.clone(), reminder);
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        match self.reminders.get(id) {
            Some(reminder) => {
                self.append(&reminder.tombstone())?;
                log::info!(
                    "Reminder {} \"{}\" removed",
                    &reminder.id,
                    &reminder.message
                );
                self.reminders.remove(id);
                Ok(())
            }
            None => Err(format!("reminder for {} does not exist", id)),
        }
    }

    pub fn list(&self) -> impl Iterator<Item = &Reminder> {
        self.reminders.values()
    }

    fn append<T: Serialize>(&self, record: &T) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)
            .map_err(|e| e.to_string())?;

        let mut line = serde_json::to_string(record).map_err(|e| e.to_string())?;
        line.push('\n');
        file.write_all(line.as_bytes()).map_err(|e| e.to_string())?;

        Ok(())
    }
}

enum ReminderCommand {
    Add {
        due: DateTime<FixedOffset>,
        message: String,
    },
    List,
    Delete {
        id: String,
    },
}

pub struct ReminderHandler {
    // The ReminderHandler is what the main module sees. The ReminderHandler
    // needs to be able to mutate the ReminderStore, i.e. insert/remove
    // values from the HashMap. If we were to do this using a mutable store,
    // then the caller would need to use a mutable reminder handler.
    //
    // This is reasonable, but breaks encapsulation, and could be construed as
    // leaking implementation details.
    //
    // The idiomatic solution for this problem is to use interior mutability
    // via the RefCell abstraction. This lets us borrow through &self (i.e. a
    // reference) by moving the borrow checks to runtime.
    store: RefCell<ReminderStore>,
}

impl ReminderHandler {
    pub fn new(data_dir: &str) -> Self {
        let path = std::path::PathBuf::from(data_dir).join("reminders.jsonl");
        Self {
            store: RefCell::new(ReminderStore::load(&path.to_string_lossy())),
        }
    }
}

impl Handler for ReminderHandler {
    fn name(&self) -> &str {
        "ReminderHandler"
    }

    fn wants(&self, msg: &Message) -> bool {
        msg.body.starts_with(REMINDER_HANDLER_PREFIX)
    }

    fn process(&self, msg: &Message) -> Vec<Action> {
        // commands come in as:
        // - @reminder add <date> <time> <msg>
        // - @reminder delete <id>
        // - @reminder list

        let args = msg
            .body
            .strip_prefix(REMINDER_HANDLER_PREFIX)
            .expect("wants() guarantees prefix is present")
            .trim();

        // NOTE: The match nesting is very deep here. We could refactor this by
        // pulling out a helper method that returns a Result<Vec<Action>, string>,
        // then use ? to bubble the err/ok up to the caller.

        match parse_command(args) {
            Ok(ReminderCommand::List) => {
                if self.store.borrow().reminders.is_empty() {
                    return vec![Action::Reply("no reminders".to_string())];
                }

                // Preallocate with a large capacity to avoid reallocations
                let mut reply = String::with_capacity(64 * self.store.borrow().reminders.len());
                for reminder in self.store.borrow().list() {
                    // A naive approach here would be push_str(format!()), but
                    // that would result in intermediate allocations for the
                    // formatted strings. Using write! lets us avoid the extra
                    // allocations.
                    writeln!(
                        reply,
                        "- {} {} {}\n",
                        reminder.id,
                        reminder.message,
                        reminder.due.format("%Y-%m-%d %H:%M")
                    )
                    .unwrap();
                }
                vec![Action::Reply(reply)]
            }
            Ok(ReminderCommand::Add { due, message }) => {
                match Reminder::new(due, message, msg.from.clone(), None) {
                    Ok(reminder) => {
                        // Need to create the happy path action first as add moves reminder
                        let created = Action::Reply(format!("Reminder {} created.", &reminder.id));
                        match self.store.borrow_mut().add(reminder) {
                            Ok(()) => vec![created],
                            Err(e) => vec![Action::Reply(e)],
                        }
                    }
                    Err(e) => vec![Action::Reply(e)],
                }
            }
            Ok(ReminderCommand::Delete { id }) => match self.store.borrow_mut().remove(&id) {
                Ok(()) => vec![Action::Reply(format!("Reminder {} deleted.", id))],
                Err(e) => vec![Action::Reply(e)],
            },
            Err(e) => vec![Action::Reply(e)],
        }
    }
}

fn parse_command(args: &str) -> Result<ReminderCommand, String> {
    let mut parts = args.splitn(2, ' ');

    match parts.next() {
        Some("add") => {
            let remainder = parts.next().ok_or("add requires <date> <time> <message>")?;
            let mut subparts = remainder.splitn(3, ' ');
            let date = subparts.next().ok_or("missing date")?;
            let time = subparts.next().ok_or("missing time")?;
            let message = subparts.next().ok_or("missing message")?;

            log::info!(
                "New reminder: date={} time={} message={}",
                date,
                time,
                message
            );

            let d = NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map_err(|e| format!("invalid date: {}", e))?;

            let t = NaiveTime::parse_from_str(time, "%H:%M")
                .map_err(|e| format!("invalid time: {}", e))?;

            let dt = d.and_time(t);
            let due = Local
                .from_local_datetime(&dt)
                .single()
                .ok_or("ambiguous local time".to_string())?
                .fixed_offset();

            Ok(ReminderCommand::Add {
                due,
                message: message.to_string(),
            })
        }
        Some("list") => Ok(ReminderCommand::List),
        Some("delete") => {
            let id = parts.next().ok_or("missing id")?;
            Ok(ReminderCommand::Delete { id: id.to_string() })
        }
        Some(&_) => Err("Invalid argument".to_string()),
        // TODO: Replace with real help message
        None => Err("help_message".to_string()),
    }
}

// TODO: Unit tests
