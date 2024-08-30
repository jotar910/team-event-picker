pub fn is_team_special(team_id: String) -> bool {
    std::env::var("SPECIAL_TEAM_ID")
        .inspect_err(|err| log::warn!("could not read special team id: {:?}", err))
        .map_or(false, |special| {
            log::trace!("create_event: special team id: {}", special);
            special == team_id
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_special_true() {
        std::env::set_var("SPECIAL_TEAM_ID", "special");
        assert_eq!(is_team_special("special".to_string()), true);
    }

    #[test]
    fn is_special_false() {
        std::env::set_var("SPECIAL_TEAM_ID", "special");
        assert_eq!(is_team_special("not_special".to_string()), false);
    }
}
