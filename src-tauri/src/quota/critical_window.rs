use std::cmp::Ordering;

use super::QuotaWindow;

pub fn select_critical_window(windows: &[QuotaWindow]) -> Option<&QuotaWindow> {
    windows
        .iter()
        .filter(|window| window.is_active)
        .filter(|window| window.remaining_percent.is_some())
        .min_by(|a, b| {
            let percent_order = a
                .remaining_percent
                .unwrap_or(100.0)
                .partial_cmp(&b.remaining_percent.unwrap_or(100.0))
                .unwrap_or(Ordering::Equal);

            if percent_order == Ordering::Equal {
                b.reset_at.cmp(&a.reset_at)
            } else {
                percent_order
            }
        })
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::quota::{PeriodType, QuotaUnit, QuotaWindow};

    use super::select_critical_window;

    fn window(id: &str, percent: Option<f64>, hours: i64) -> QuotaWindow {
        QuotaWindow {
            id: id.to_string(),
            name: id.to_string(),
            period_type: PeriodType::Rolling,
            period_seconds: None,
            total: None,
            used: None,
            remaining: None,
            remaining_percent: percent,
            unit: QuotaUnit::Percent,
            reset_at: Some(Utc::now() + Duration::hours(hours)),
            is_active: true,
            is_current_constraint: false,
            data_source: "test".to_string(),
        }
    }

    #[test]
    fn selects_lowest_percent() {
        let windows = vec![window("a", Some(70.0), 1), window("b", Some(20.0), 1)];
        assert_eq!(select_critical_window(&windows).unwrap().id, "b");
    }

    #[test]
    fn equal_percent_uses_later_reset() {
        let windows = vec![window("a", Some(20.0), 1), window("b", Some(20.0), 4)];
        assert_eq!(select_critical_window(&windows).unwrap().id, "b");
    }

    #[test]
    fn ignores_unknown_percent() {
        let windows = vec![window("a", None, 1), window("b", Some(35.0), 4)];
        assert_eq!(select_critical_window(&windows).unwrap().id, "b");
    }
}
