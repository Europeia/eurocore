use std::collections::HashMap;
use std::fmt::Formatter;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::error::Error;

pub(crate) struct Nation {
    name: String,
    password: String,
    pin: Option<String>,
}

impl Nation {
    pub(crate) fn new(name: String, password: String) -> Self {
        Self {
            name,
            password,
            pin: None,
        }
    }
}

impl std::fmt::Debug for Nation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Nation")
            .field("name", &self.name)
            .field("pin", &self.pin)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct NationList(Arc<RwLock<HashMap<String, Nation>>>);

impl NationList {
    pub(crate) fn new(nations: HashMap<String, Nation>) -> Self {
        Self(Arc::new(RwLock::new(nations)))
    }

    pub(crate) async fn contains_nation(&self, name: &str) -> bool {
        self.0.read().await.get(name).is_some()
    }

    pub(crate) async fn get_password(&self, name: &str) -> Result<String, Error> {
        match self.0.read().await.get(name) {
            Some(nation) => Ok(nation.password.clone()),
            None => Err(Error::InvalidNation),
        }
    }

    /// Return the nation's pin, if it exists, or an empty string if it does not
    /// This is a secondary method of authentication, so it being empty is fine.
    pub(crate) async fn get_pin(&self, name: &str) -> Result<String, Error> {
        let nations = self.0.read().await;

        let nation = nations.get(name).ok_or(Error::InvalidNation)?;

        Ok(nation.pin.clone().unwrap_or_default())
    }

    pub(crate) async fn set_pin(&self, name: &str, pin: &str) -> Result<(), Error> {
        if let Some(nation) = self.0.write().await.get_mut(name) {
            nation.pin = Some(pin.to_string());

            Ok(())
        } else {
            Err(Error::InvalidNation)
        }
    }
}

pub(crate) fn create_nations_map(nations: &str) -> HashMap<String, Nation> {
    nations
        .split(',')
        .map(|nation| {
            let mut split = nation.split(':');
            let name = split.next().unwrap().to_string();
            let password = split.next().unwrap().to_string();
            (name.clone(), Nation::new(name, password))
        })
        .collect()
}
