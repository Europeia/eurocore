use crate::controllers::{dispatch, rmbpost, telegram, user};

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    pub(crate) user_controller: user::Controller,
    pub(crate) dispatch_controller: dispatch::Controller,
    pub(crate) rmbpost_controller: rmbpost::Controller,
    pub(crate) telegram_controller: telegram::Controller,
}

impl AppState {
    pub(crate) fn new(
        user_controller: user::Controller,
        dispatch_controller: dispatch::Controller,
        rmbpost_controller: rmbpost::Controller,
        telegram_controller: telegram::Controller,
    ) -> Self {
        AppState {
            user_controller,
            dispatch_controller,
            rmbpost_controller,
            telegram_controller,
        }
    }
}
