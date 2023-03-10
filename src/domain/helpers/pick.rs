use rand::Rng;

use crate::domain::entities::{Event, EventPick};

pub fn pick(event: &Event) -> (EventPick, u32) {
    let pick = event.cur_pick;
    let total_participants = event.participants.len();

    let mut not_picked: Vec<usize> = vec![];

    for i in 0..event.participants.len() {
        if pick & (1 << i) == 0 {
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
        new_pick = pick | (1 << new_pick_idx);
    }

    (
        EventPick {
            event: event.id,
            prev_pick: pick,
            cur_pick: new_pick,
        },
        event.participants[new_pick_idx],
    )
}
