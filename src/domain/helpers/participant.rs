use crate::domain::entities::Participant;
use rand::Rng;

pub fn last_picked<'a, 'b>(picks: &'a Vec<Participant>) -> Option<&'a Participant>
where
    'a: 'b,
{
    let mut last_picked_at = 0i64;
    return picks.iter().fold(None, |acc, participant| {
        if participant.picked_at.is_some() && participant.picked_at.unwrap() > last_picked_at {
            last_picked_at = participant.picked_at.unwrap();
            return Some(participant);
        }
        return acc;
    });
}

pub fn replace_participant(picks: Vec<Participant>, participant: Participant) -> Vec<Participant> {
    let mut picks = picks.clone();
    if let Some(p) = picks.iter_mut().find(|p| p.user == participant.user) {
        *p = participant;
    }
    return picks;
}

pub fn pick_new<'a, 'b>(picks: &'a Vec<Participant>) -> Option<&'b Participant>
where
    'a: 'b,
{
    let unpicked = picks
        .iter()
        .filter(|participant| !participant.picked)
        .collect::<Vec<&Participant>>();
    if unpicked.len() == 0 {
        return None;
    }
    let random_index = rand::thread_rng().gen_range(0..unpicked.len());
    return Some(unpicked[random_index]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_last_picked() {
        let picks = vec![
            Participant {
                user: String::from("U04PGARU4K1"),
                picked: false,
                created_at: 1723822080,
                picked_at: None,
            },
            Participant {
                user: String::from("USLACKBOT"),
                picked: true,
                created_at: 1723822080,
                picked_at: Some(1724681700),
            },
            Participant {
                user: String::from("U0797QD5AJZ"),
                picked: true,
                created_at: 1723822080,
                picked_at: Some(1724681760),
            },
        ];
        let last_picked = last_picked(&picks);
        assert_eq!(last_picked.unwrap().user, "U0797QD5AJZ");
    }
}
