use std::collections::HashMap;

pub struct PickUpdateHelper {
    picks: HashMap<u32, bool>,
}

impl PickUpdateHelper {
    pub fn new(participants: &Vec<u32>, cur_pick: u32) -> Self {
        let mut i = 0;
        let mut picks: HashMap<u32, bool> = HashMap::new();
        for &participant in participants.iter() {
            picks.insert(participant, cur_pick & (1 << i) > 0);
            i += 1;
        }
        PickUpdateHelper { picks }
    }

    pub fn new_pick(&self, participants: &Vec<u32>) -> u32 {
        let mut i = 0;
        let mut pick = 0;
        for participant in participants.iter() {
            let mut value = 0;
            if self.picks.contains_key(participant) {
                value = 1;
            }

            pick = pick | (value << i);
            i += 1;
        }
        pick
    }
}
