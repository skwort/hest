use chrono::{DateTime, FixedOffset};
use serde::{Serialize, Deserialize};

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
    pub fn tombstone(&self) -> ReminderRecord::Tombstone {
        ReminderRecord::Tombstone{ id: self.id.clone(), deleted: true };
    }
}

pub fn load(path: &str) -> HashMap<String, Reminder> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut reminders: HashMap<String, Reminder> = HashMap::new();

    for line in contents.lines() {
        if let Ok(record) = serde_json::from_str::<ReminderRecord>(line) {
            match record {
                ReminderRecord::Reminder(r) => { reminders.insert(r.id.clone(), r); }
                ReminderRecord::Tombstone { id, .. } => { reminders.remove(&id); }
            }
        }
    }

    reminders
}

fn append<T: Serialize>(path: &str, record: &T) -> Result<(), String> {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|e| e.to_string())?;

    let mut line = serde_json::to_string(record).map_err(|e| e.to_string())?;
    line.push('\n');
    file.write_all(line.as_bytes()).map_err(|e| e.to_string())?;

    Ok(())
}
