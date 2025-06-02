use crate::core::error::{ConfigError, Error};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot};

struct Nation {
    name: String,
    password: String,
    pin: Option<String>,
}

impl Nation {
    fn new(name: &str, password: &str) -> Self {
        Self {
            name: name.into(),
            password: password.into(),
            pin: None,
        }
    }
}

enum Action {
    ListNations,
    GetPassword { nation: String },
    GetPin { nation: String },
    SetPin { nation: String, pin: String },
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
    List { nations: Vec<String> },
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

    #[tracing::instrument(skip_all)]
    pub(crate) async fn list_nations(&self) -> Result<Vec<String>, Error> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self.tx.send(Command::new(Action::ListNations, tx)).await {
            tracing::error!("failed to send message: {}", e);
            return Err(Error::Internal);
        };

        match rx.await {
            Ok(Response::List { nations }) => Ok(nations),
            Ok(_) => unreachable!(),
            Err(e) => {
                tracing::error!("failed to get nations: {}", e);
                Err(Error::Internal)
            }
        }
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn get_password(&self, nation: &str) -> Result<String, Error> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self
            .tx
            .send(Command::new(
                Action::GetPassword {
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

    #[tracing::instrument(skip_all)]
    pub(crate) async fn get_pin(&self, nation: &str) -> Result<Option<String>, Error> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self
            .tx
            .send(Command::new(
                Action::GetPin {
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

    #[tracing::instrument(skip_all)]
    pub(crate) async fn set_pin(&self, nation: &str, pin: &str) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self
            .tx
            .send(Command::new(
                Action::SetPin {
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
    fn new(rx: mpsc::Receiver<Command>, nations: HashMap<String, Nation>) -> Self {
        Self { rx, nations }
    }

    #[tracing::instrument(skip_all)]
    fn process(&mut self, command: Command) {
        let resp = match command.action {
            Action::ListNations => {
                tracing::debug!("listing nations");
                let nations = self.nations.keys().cloned().collect::<Vec<String>>();

                Response::List { nations }
            }
            Action::GetPassword { nation } => {
                tracing::debug!("retrieving password for nation: {}", &nation);
                if let Some(nation) = self.nations.get(&nation) {
                    Response::Password {
                        password: Some(nation.password.clone()),
                    }
                } else {
                    Response::Password { password: None }
                }
            }
            Action::GetPin { nation } => {
                tracing::debug!("retrieving pin for nation: {}", &nation);
                if let Some(nation) = self.nations.get(&nation) {
                    Response::Pin {
                        pin: nation.pin.clone(),
                    }
                } else {
                    Response::Pin { pin: None }
                }
            }
            Action::SetPin { nation, pin } => {
                tracing::debug!("setting pin for nation: {}", &nation);
                if let Some(nation) = self.nations.get_mut(&nation) {
                    nation.pin = Some(pin);
                }

                Response::Ok
            }
        };

        if command.tx.send(resp).is_err() {
            tracing::error!("failed to send response");
        }
    }

    #[tracing::instrument(skip_all)]
    async fn run(&mut self) {
        loop {
            match self.rx.recv().await {
                None => {
                    tracing::warn!("channel is closed");
                    break;
                }
                Some(command) => {
                    tracing::info!("command received");
                    self.process(command);
                }
            }
        }
    }
}

pub(crate) enum Source {
    Env(String),
    File(PathBuf),
    Str(String),
}

pub(crate) fn new(source: Source) -> Result<Sender, ConfigError> {
    let nations = match source {
        Source::Str(var) => var,
        Source::Env(var) => env::var(var)?,
        Source::File(path) => fs::read_to_string(&path)?,
    };

    let nations = parse_nations(&nations)?;

    let (tx, rx) = mpsc::channel(16);

    let mut receiver = Receiver::new(rx, nations);

    let sender = Sender::new(tx);

    tokio::task::spawn(async move {
        receiver.run().await;
    });

    Ok(sender)
}

fn parse_nations(nations: &str) -> Result<HashMap<String, Nation>, ConfigError> {
    nations
        .split(",")
        .map(parse_nation)
        .collect::<Result<HashMap<String, Nation>, ConfigError>>()
}

fn parse_nation(value: &str) -> Result<(String, Nation), ConfigError> {
    let mut split = value.splitn(2, ":");

    let nation = split
        .next()
        .ok_or(ConfigError::Nations(value.to_string()))?
        .trim();

    let password = split
        .next()
        .ok_or(ConfigError::Nations(value.to_string()))?;

    Ok((nation.to_string(), Nation::new(nation, password)))
}
