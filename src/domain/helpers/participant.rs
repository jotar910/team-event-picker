use crate::domain::entities::Participant;
use rand::Rng;

pub fn last_picked<'a, 'b>(picks: &'a Vec<Participant>) -> Option<&'a Participant>
where
    'a: 'b,
{
    let last_picked_at = 0i64;
    return picks.iter().find(|participant| {
        participant.picked_at.is_some() && participant.picked_at.unwrap() > last_picked_at
    });
}

pub fn replace_participant(picks: Vec<Participant>, participant: Participant) -> Vec<Participant> {
    let mut picks = picks.clone();
    if let Some(p) = picks
        .iter_mut()
        .find(|p| p.user == participant.user)
    {
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
