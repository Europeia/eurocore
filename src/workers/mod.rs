use std::time::Duration;

pub(crate) mod dispatch;
pub(crate) mod rmbpost;
pub(crate) mod telegram;

const PERIOD: Duration = Duration::from_millis(250);
