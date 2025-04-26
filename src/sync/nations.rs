use crate::core::error::Error;
use std::collections::HashMap;
use tokio::sync::{
    mpsc::{self, error::TryRecvError},
    oneshot,
};

struct Nation {
    name: String,
    password: String,
    pin: Option<String>,
}

enum Action {
    get_password { nation: String },
    get_pin { nation: String },
    set_pin { nation: String, pin: String },
}

struct Command {
    action: Action,
    tx: oneshot::Sender<Response>,
}

impl Command {
    fn new(action: Action, tx: oneshot::Sender<Response>) -> Self {
        Self { action, tx }
    }
}

#[derive(Debug)]
enum Response {
    Ok,
    Password { password: Option<String> },
    Pin { pin: Option<String> },
}

#[derive(Clone, Debug)]
pub(crate) struct Sender {
    tx: mpsc::Sender<Command>,
}

impl Sender {
    fn new(tx: mpsc::Sender<Command>) -> Self {
        Self { tx }
    }

    pub(crate) async fn get_password(&self, nation: &str) -> Result<String, Error> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self
            .tx
            .send(Command::new(
                Action::get_password {
                    nation: nation.to_owned(),
                },
                tx,
            ))
            .await
        {
            tracing::error!("failed to send message: {}", e);
            return Err(Error::Internal);
        };

        match rx.await {
            Ok(Response::Password { password }) => {
                if let Some(password) = password {
                    Ok(password)
                } else {
                    Err(Error::InvalidNation)
                }
            }
            Ok(_) => unreachable!(),
            Err(e) => {
                tracing::error!("failed to get password: {}", e);
                Err(Error::Internal)
            }
        }
    }

    pub(crate) async fn get_pin(&self, nation: &str) -> Result<Option<String>, Error> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self
            .tx
            .send(Command::new(
                Action::get_pin {
                    nation: nation.to_owned(),
                },
                tx,
            ))
            .await
        {
            tracing::error!("failed to send message: {}", e);
            return Err(Error::Internal);
        };

        match rx.await {
            Ok(Response::Pin { pin }) => Ok(pin),
            Ok(_) => unreachable!(),
            Err(e) => {
                tracing::error!("failed to get pin: {}", e);
                Err(Error::Internal)
            }
        }
    }

    pub(crate) async fn set_pin(&self, nation: &str, pin: &str) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self
            .tx
            .send(Command::new(
                Action::set_pin {
                    nation: nation.to_owned(),
                    pin: pin.to_owned(),
                },
                tx,
            ))
            .await
        {
            tracing::error!("failed to send message: {}", e);
            return Err(Error::Internal);
        };

        match rx.await {
            Ok(Response::Ok) => Ok(()),
            Ok(_) => unreachable!(),
            Err(e) => {
                tracing::error!("failed to set pin: {}", e);
                Err(Error::Internal)
            }
        }
    }
}

pub(crate) struct Receiver {
    rx: mpsc::Receiver<Command>,
    nations: HashMap<String, Nation>,
}

impl Receiver {
    fn new(rx: mpsc::Receiver<Command>) -> Self {
        Self {
            rx,
            nations: HashMap::new(),
        }
    }

    fn process(&mut self, command: Command) {
        let resp = match command.action {
            Action::get_password { nation } => {
                if let Some(nation) = self.nations.get(&nation) {
                    Response::Password {
                        password: Some(nation.password.clone()),
                    }
                } else {
                    Response::Password { password: None }
                }
            }
            Action::get_pin { nation } => {
                if let Some(nation) = self.nations.get(&nation) {
                    Response::Pin {
                        pin: nation.pin.clone(),
                    }
                } else {
                    Response::Pin { pin: None }
                }
            }
            Action::set_pin { nation, pin } => {
                if let Some(nation) = self.nations.get_mut(&nation) {
                    nation.pin = Some(pin);
                }

                Response::Ok
            }
        };

        command.tx.send(resp).unwrap();
    }

    fn run(&mut self) {
        loop {
            match self.rx.try_recv() {
                Err(e) => match e {
                    TryRecvError::Empty => (),
                    TryRecvError::Disconnected => {
                        tracing::warn!("nation manager disconnected, exiting");
                        break;
                    }
                },
                Ok(command) => {
                    tracing::info!("command received");
                }
            }
        }
    }
}

pub(crate) fn new() -> (Sender, Receiver) {
    let (tx, rx) = mpsc::channel(16);

    (Sender::new(tx), Receiver::new(rx))
}
