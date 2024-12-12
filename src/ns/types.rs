use crate::core::error::Error;
use serde::Serialize;
use std::marker::PhantomData;

enum Command {
    Dispatch,
    RmbPost,
}

impl Serialize for Command {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            Command::Dispatch => serializer.serialize_str("dispatch"),
            Command::RmbPost => serializer.serialize_str("rmbpost"),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Mode {
    Prepare,
    Execute,
}

impl Serialize for Mode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            Mode::Prepare => serializer.serialize_str("prepare"),
            Mode::Execute => serializer.serialize_str("execute"),
        }
    }
}

/// Marker trait for  types that can be used as data for an NS command.
pub(crate) trait PrivateCommand {}

#[derive(Serialize)]
pub(crate) struct Unprepared;
impl PrivateCommand for Unprepared {}

#[derive(Serialize)]
pub(crate) struct Prepared;
impl PrivateCommand for Prepared {}
