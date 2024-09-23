use std::time::Duration;

pub(crate) mod dispatch;
pub(crate) mod rmb;
pub(crate) mod telegram;

const PERIOD: Duration = Duration::from_secs(2);
