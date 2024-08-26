use std::{collections::HashMap, fmt::Display, sync::Arc, vec};

use tokio::{
    sync::{mpsc::Sender, Mutex},
    task::yield_now,
};

use super::{date::SchedulerDate, entities::EventSchedule, helpers};
use crate::{
    domain::events::pick_auto_participants,
    helpers::date::Date,
    repository::{auth, event},
};

struct DateRecords {
    events_per_minute: HashMap<i64, Vec<u32>>,
    saved_events_date: HashMap<u32, SchedulerDate>,
}

impl DateRecords {
    fn new() -> Self {
        Self {
            events_per_minute: HashMap::new(),
            saved_events_date: HashMap::new(),
        }
    }

    async fn check(
        &self,
        event_repo: Arc<dyn event::Repository>,
        auth_repo: Arc<dyn auth::Repository>,
        minute: i64,
    ) -> Vec<pick_auto_participants::Pick> {
        if let Some(events) = self.events_per_minute.get(&minute) {
            if let Some(response) = self.pick_for_events(event_repo, auth_repo, events).await {
                return response.picks.into_iter().map(|(_, picks)| picks).collect();
            }
        }
        vec![]
    }

    async fn pick_for_events(
        &self,
        event_repo: Arc<dyn event::Repository>,
        auth_repo: Arc<dyn auth::Repository>,
        events: &Vec<u32>,
    ) -> Option<pick_auto_participants::Response> {
        let req = pick_auto_participants::Request {
            events: events.clone(),
        };
        let res = match pick_auto_participants::execute(event_repo.clone(), auth_repo, req).await {
            Ok(res) => res,
            Err(err) => {
                log::error!("could not automatically pick participants: {:?}", err);
                return None;
            }
        };
        log::trace!(
            "automatically picked participants for events {:?}: {:?}",
            events,
            res
        );
        Some(res)
    }

    fn insert(&mut self, event: EventSchedule) {
        if self.saved_events_date.contains_key(&event.id) {
            log::trace!("removing saved event before adding the new event to scheduler");
            self.clear_event(event.id);
        }

        let date = SchedulerDate::new(event.timestamp, event.timezone.clone(), event.repeat);
        self.set_event_minutes(event.id, &date);
        self.saved_events_date.insert(event.id, date);
        let date_str = Date::new(event.timestamp)
            .with_timezone(event.timezone)
            .to_string();
        log::debug!(
            "added event to scheduler: {} at {} ({} secs)",
            event.id,
            date_str,
            event.timestamp
        );
    }

    fn remove(&mut self, event_id: u32) {
        if !self.saved_events_date.contains_key(&event_id) {
            log::trace!("trying to remove inexistent event from scheduler");
            return;
        }
        self.clear_event(event_id);
        log::debug!("removed event from scheduler: {}", event_id);
    }

    fn reset_minutes(&mut self) {
        self.events_per_minute = HashMap::new();

        let mut saved_events_date: HashMap<u32, SchedulerDate> = HashMap::new();
        for (&event_id, date) in self.saved_events_date.iter() {
            saved_events_date.insert(event_id, date.clone());
        }
        for (&event_id, date) in saved_events_date.iter() {
            self.set_event_minutes(event_id, date);
        }
    }

    fn set_event_minutes(&mut self, event_id: u32, date: &SchedulerDate) {
        let minutes = date.find_minutes();
        log::trace!(
            "calculated minutes for the event {}: {}",
            event_id,
            minutes
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join("|")
        );
        for minute in minutes.iter() {
            match self.events_per_minute.get_mut(&minute) {
                Some(events_per_minute) => {
                    events_per_minute.push(event_id);
                }
                None => {
                    self.events_per_minute.insert(*minute, vec![event_id]);
                }
            }
        }
    }

    fn clear_event(&mut self, event_id: u32) {
        let date = match self.saved_events_date.get(&event_id) {
            Some(date) => date,
            None => return,
        };
        for minute in date.find_minutes().into_iter() {
            let events = match self.events_per_minute.get_mut(&minute) {
                Some(events) => events,
                None => continue,
            };
            if let Some(index) = events.iter().position(|&event| event == event_id) {
                events.remove(index);
            }
        }
    }
}

impl Display for DateRecords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "total_events={}, total_minutes={}",
            self.saved_events_date.len(),
            self.events_per_minute.len()
        )
    }
}

pub struct Scheduler {
    pick_sender: Sender<Vec<pick_auto_participants::Pick>>,
    mutex: Mutex<DateRecords>,
}

impl Scheduler {
    pub fn new(pick_tx: Sender<Vec<pick_auto_participants::Pick>>) -> Self {
        Self {
            pick_sender: pick_tx,
            mutex: Mutex::new(DateRecords::new()),
        }
    }

    pub async fn start(
        &self,
        event_repo: Arc<dyn event::Repository>,
        auth_repo: Arc<dyn auth::Repository>,
    ) {
        loop {
            helpers::sleep_until_next_minute();

            let current_minute = helpers::find_current_minute();
            let ending_minute = helpers::find_ending_minute();
            for minute in current_minute..ending_minute {
                {
                    let records = self.mutex.lock().await;
                    if minute % 20 == 0 {
                        log::trace!("scheduler state: minute={}, {}", minute, records);
                    }
                    let picks = records
                        .check(event_repo.clone(), auth_repo.clone(), minute)
                        .await;
                    if let Err(err) = self.pick_sender.send(picks).await {
                        log::error!("failed to notify pick results: {}", err);
                    }
                    yield_now().await;
                }
                helpers::sleep_until_next_minute();
            }

            {
                log::trace!("finished year round: inserting a new round of events");
                let mut records = self.mutex.lock().await;
                records.reset_minutes();
                yield_now().await;
            }
        }
    }

    pub async fn insert(&self, event: EventSchedule) {
        let mut records = self.mutex.lock().await;
        records.insert(event);
    }

    pub async fn remove(&self, event_id: u32) {
        let mut records = self.mutex.lock().await;
        records.remove(event_id);
    }
}
