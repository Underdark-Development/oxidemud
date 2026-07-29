use crate::components::Gender;

pub fn interpolate(
    template: &str,
    actor_name: &str,
    actor_gender: &Gender,
    target_name: Option<&str>,
    target_gender: Option<&Gender>,
) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            match chars.next() {
                Some('n') => result.push_str(actor_name),
                Some('N') => result.push_str(target_name.unwrap_or("someone")),
                Some('m') => result.push_str(&actor_gender.pronoun_possessive),
                Some('M') => result.push_str(
                    &target_gender.map_or("their".into(), |g| g.pronoun_possessive.clone()),
                ),
                Some('s') => result.push_str(&actor_gender.pronoun_subject),
                Some('S') => result
                    .push_str(&target_gender.map_or("they".into(), |g| g.pronoun_subject.clone())),
                Some('o') => result.push_str(&actor_gender.pronoun_object),
                Some('O') => result
                    .push_str(&target_gender.map_or("them".into(), |g| g.pronoun_object.clone())),
                Some(c) => {
                    result.push('$');
                    result.push(c);
                }
                None => {
                    result.push('$');
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}
