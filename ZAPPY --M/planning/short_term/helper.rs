/// Selects an owner for a task deterministically.
#[must_use]
pub fn select_owner(idx: u16) -> String {
    match idx % 3 {
        0 => "ops".into(),
        1 => "engineering".into(),
        _ => "research".into(),
    }
}

/// Determines task count per phase based on resources and risk.
#[must_use]
pub fn task_count(resource_slots: usize, risk_multiplier: f32) -> u16 {
    let mut count = resource_slots.max(1) as u16;
    if risk_multiplier > 1.2 {
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_rotates() {
        assert_eq!(select_owner(0), "ops");
        assert_eq!(select_owner(1), "engineering");
    }

    #[test]
    fn task_count_increases_with_risk() {
        assert!(task_count(1, 1.3) > task_count(1, 1.0));
    }
}
