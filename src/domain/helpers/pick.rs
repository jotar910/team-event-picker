use rand::Rng;

use crate::domain::entities::{Event, EventPick};

pub fn pick(event: &Event) -> (EventPick, u32) {
    return pick_participant(event, event.cur_pick, event.cur_pick);
}

pub fn repick(event: &Event) -> (EventPick, u32) {
    if event.cur_pick.count_ones() < event.participants.len() as u32 {
        return pick_participant(event, event.cur_pick, event.prev_pick);
    }
    return pick_participant(event, event.prev_pick, event.prev_pick);
}

fn pick_participant(event: &Event, pick_to_include: u32, pick_to_update: u32) -> (EventPick, u32) {
    let total_participants = event.participants.len();

    let mut not_picked: Vec<usize> = vec![];

    for i in 0..event.participants.len() {
        if pick_to_include & (1 << i) == 0 {
            not_picked.push(i);
        }
    }

    let new_pick_idx: usize;
    let new_pick: u32;

    if not_picked.len() == 0 {
        new_pick_idx = rand::thread_rng().gen_range(0..total_participants);
        new_pick = 1 << new_pick_idx;
    } else {
        new_pick_idx = not_picked[rand::thread_rng().gen_range(0..not_picked.len())];
        new_pick = pick_to_update | (1 << new_pick_idx);
    }

    (
        EventPick {
            event: event.id,
            prev_pick: pick_to_update,
            cur_pick: new_pick,
        },
        event.participants[total_participants - new_pick_idx - 1],
    )
}

#[cfg(test)]
mod test {
    use std::collections::hash_set;

    use crate::domain::{entities::RepeatPeriod, timezone::Timezone};

    use super::*;

    #[test]
    fn pick_concontrol() {
        let event = Event {
            id: 89,
            name: String::from("[Alpha] Daily"),
            timestamp: 1701248400,
            timezone: Timezone::UTC,
            repeat: RepeatPeriod::Daily,
            participants: vec![1, 12, 5, 10, 11, 14, 218, 216, 375, 217, 376, 377, 6],
            channel: 1,
            prev_pick: 6014,
            cur_pick: 6015,
            team_id: String::from("CONT"),
            deleted: false,
        };

        let (e, i) = pick(&event);
        assert!(e.cur_pick == 6143 || e.cur_pick == 8063);
        if e.cur_pick == 6143 {
            assert!(e.cur_pick & (1 << 11) == 0);
            assert!(i == 14);
        } else {
            assert!(e.cur_pick & (1 << 7) == 0);
            assert!(i == 12);
        }
    }

    #[test]
    fn pick_concontrol_fully() {
        let mut event = Event {
            id: 89,
            name: String::from("[Alpha] Daily"),
            timestamp: 1701248400,
            timezone: Timezone::UTC,
            repeat: RepeatPeriod::Daily,
            participants: vec![1, 12, 5, 10, 11, 14, 218, 216, 375, 217, 376, 377, 6],
            channel: 1,
            prev_pick: 0,
            cur_pick: 0,
            team_id: String::from("CONT"),
            deleted: false,
        };
        let mut picked = hash_set::HashSet::<u32>::new();
        loop {
            let (e, i) = pick(&event);
            if e.cur_pick == 8191 {
                break;
            }
            event.cur_pick = e.cur_pick;
            event.prev_pick = e.prev_pick;
            assert!(e.cur_pick > 0);
            assert!(!picked.contains(&i));
            picked.insert(i);
        }
        assert!(true);
    }
}
