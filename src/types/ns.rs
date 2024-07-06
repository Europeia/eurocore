use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt::Display;
use crate::core::error::Error;

#[derive(Clone)]
pub(crate) enum FactbookCategory {
    Factbook(FactbookSubcategory), // 1
    Bulletin(BulletinSubcategory), // 3
    Account(AccountSubcategory), // 5
    Meta(MetaSubcategory), // 8
}

impl FactbookCategory {
    fn to_tuple(&self) -> (u32, u32) {
        match self {
            FactbookCategory::Factbook(subcategory) => {
                match subcategory {
                    FactbookSubcategory::Overview => (1, 100),
                    FactbookSubcategory::History => (1, 101),
                    FactbookSubcategory::Geography => (1, 102),
                    FactbookSubcategory::Culture => (1, 103),
                    FactbookSubcategory::Politics => (1, 104),
                    FactbookSubcategory::Legislation => (1, 105),
                    FactbookSubcategory::Religion => (1, 106),
                    FactbookSubcategory::Military => (1, 107),
                    FactbookSubcategory::Economy => (1, 108),
                    FactbookSubcategory::International => (1, 109),
                    FactbookSubcategory::Trivia => (1, 110),
                    FactbookSubcategory::Miscellaneous => (1, 111),
                }
            }
            FactbookCategory::Bulletin(subcategory) => {
                match subcategory {
                    BulletinSubcategory::Policy => (3, 305),
                    BulletinSubcategory::News => (3, 315),
                    BulletinSubcategory::Opinion => (3, 325),
                    BulletinSubcategory::Campaign => (3, 385),
                }
            }
            FactbookCategory::Account(subcategory) => {
                match subcategory {
                    AccountSubcategory::Military => (5, 505),
                    AccountSubcategory::Trade => (5, 515),
                    AccountSubcategory::Sport => (5, 525),
                    AccountSubcategory::Drama => (5, 535),
                    AccountSubcategory::Diplomacy => (5, 545),
                    AccountSubcategory::Science => (5, 555),
                    AccountSubcategory::Culture => (5, 565),
                    AccountSubcategory::Other => (5, 595),
                }
            }
            FactbookCategory::Meta(subcategory) => {
                match subcategory {
                    MetaSubcategory::Gameplay => (8, 835),
                    MetaSubcategory::Reference => (8, 845),
                }
            }
        }
    }
}

impl TryFrom<(u32, u32)> for FactbookCategory {
    type Error = Error;

    fn try_from((category, subcategory): (u32, u32)) -> Result<Self, Self::Error> {
        match category {
            1 => {
                match subcategory {
                    100 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Overview)),
                    101 => Ok(FactbookCategory::Factbook(FactbookSubcategory::History)),
                    102 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Geography)),
                    103 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Culture)),
                    104 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Politics)),
                    105 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Legislation)),
                    106 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Religion)),
                    107 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Military)),
                    108 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Economy)),
                    109 => Ok(FactbookCategory::Factbook(FactbookSubcategory::International)),
                    110 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Trivia)),
                    111 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Miscellaneous)),
                    _ => Err(Error::InvalidFactbookCategory),
                }
            }
            3 => {
                match subcategory {
                    305 => Ok(FactbookCategory::Bulletin(BulletinSubcategory::Policy)),
                    315 => Ok(FactbookCategory::Bulletin(BulletinSubcategory::News)),
                    325 => Ok(FactbookCategory::Bulletin(BulletinSubcategory::Opinion)),
                    385 => Ok(FactbookCategory::Bulletin(BulletinSubcategory::Campaign)),
                    _ => Err(Error::InvalidFactbookCategory),
                }
            }
            5 => {
                match subcategory {
                    505 => Ok(FactbookCategory::Account(AccountSubcategory::Military)),
                    515 => Ok(FactbookCategory::Account(AccountSubcategory::Trade)),
                    525 => Ok(FactbookCategory::Account(AccountSubcategory::Sport)),
                    535 => Ok(FactbookCategory::Account(AccountSubcategory::Drama)),
                    545 => Ok(FactbookCategory::Account(AccountSubcategory::Diplomacy)),
                    555 => Ok(FactbookCategory::Account(AccountSubcategory::Science)),
                    565 => Ok(FactbookCategory::Account(AccountSubcategory::Culture)),
                    595 => Ok(FactbookCategory::Account(AccountSubcategory::Other)),
                    _ => Err(Error::InvalidFactbookCategory),
                }
            }
            8 => {
                match subcategory {
                    835 => Ok(FactbookCategory::Meta(MetaSubcategory::Gameplay)),
                    845 => Ok(FactbookCategory::Meta(MetaSubcategory::Reference)),
                    _ => Err(Error::InvalidFactbookCategory),
                }
            }
            _ => Err(Error::InvalidFactbookCategory),
        }
    }
}

#[derive(Clone)]
pub(crate) enum FactbookSubcategory {
    Overview, // 100
    History, // 101
    Geography, // 102
    Culture, // 103
    Politics, // 104
    Legislation, // 105
    Religion, // 106
    Military, // 107
    Economy, // 108
    International, // 109
    Trivia, // 110
    Miscellaneous, // 111
}

#[derive(Clone)]
pub(crate) enum BulletinSubcategory {
    Policy, // 305
    News, // 315
    Opinion, // 325
    Campaign, // 385
}

#[derive(Clone)]
pub(crate) enum AccountSubcategory {
    Military, // 505
    Trade, // 515
    Sport, // 525
    Drama, // 535
    Diplomacy, // 545
    Science, // 555
    Culture, // 565
    Other, // 595
}

#[derive(Clone)]
pub(crate) enum MetaSubcategory {
    Gameplay, // 835
    Reference, // 845
}

#[derive(Clone)]
pub(crate) enum Command {
    Dispatch
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Command::Dispatch => "dispatch".to_string(),
        })
    }
}

#[derive(Clone)]
pub(crate) enum DispatchAction {
    Add,
    Edit(u32),
    Remove(u32),
}

impl Display for DispatchAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            DispatchAction::Add => "add".to_string(),
            DispatchAction::Edit(_) => "edit".to_string(),
            DispatchAction::Remove(_) => "remove".to_string(),
        })
    }
}

#[derive(Clone)]
pub(crate) enum Mode {
    Prepare,
    Execute,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Mode::Prepare => "prepare".to_string(),
            Mode::Execute => "execute".to_string(),
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Dispatch {
    #[serde(rename="dispatchid", skip_serializing_if="Option::is_none")]
    id: Option<u32>,
    nation: String,
    #[serde(rename="c")]
    command: String,
    #[serde(rename="dispatch")]
    action: String,
    #[serde(skip_serializing_if="Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    category: Option<u32>,
    #[serde(skip_serializing_if="Option::is_none")]
    subcategory: Option<u32>,
    mode: String,
    #[serde(skip_serializing_if="Option::is_none")]
    token: Option<String>,
}

impl Dispatch {
    fn new(
        nation: String,
        command: Command,
        action: DispatchAction,
        title: Option<String>,
        text: Option<String>,
        factbook_category: Option<FactbookCategory>
    ) -> Self {
        let id = match action {
            DispatchAction::Edit(id) => Some(id),
            DispatchAction::Remove(id) => Some(id),
            _ => None,
        };

        let (category, subcategory) = match factbook_category {
            Some(category) => {
                let (category, subcategory) = category.to_tuple();

                (Some(category), Some(subcategory))
            },
            None => (None, None),
        };

        Self {
            id,
            nation,
            command: command.to_string(),
            action: action.to_string(),
            title,
            text,
            category,
            subcategory,
            mode: Mode::Prepare.to_string(),
            token: None,
        }
    }

    pub(crate) fn set_mode(&mut self, mode: Mode) {
        self.mode = mode.to_string();
    }

    pub(crate) fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    pub(crate) fn try_from_new_params(params: NewDispatchParams, nation: &str) -> Result<Self, Error> {
        Ok(Dispatch::new(
            nation.to_string(),
            Command::Dispatch,
            DispatchAction::Add,
            Some(params.title),
            Some(params.text),
            Some(FactbookCategory::try_from(
                (params.category, params.subcategory)
            )?)
        ))
    }

    pub(crate) fn try_from_edit_params(params: EditDispatchParams, nation: &str) -> Result<Self, Error> {
        Ok(Dispatch::new(
            nation.to_string(),
            Command::Dispatch,
            DispatchAction::Edit(params.id),
            Some(params.title),
            Some(params.text),
            Some(FactbookCategory::try_from(
                (params.category, params.subcategory)
            )?)
        ))
    }

    pub(crate) fn from_remove_params(params: RemoveDispatchParams, nation: &str) -> Self {
        Dispatch::new(
            nation.to_string(),
            Command::Dispatch,
            DispatchAction::Remove(params.id),
            None,
            None,
            None
        )
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct NewDispatchParams {
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) category: u32,
    pub(crate) subcategory: u32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EditDispatchParams {
    pub(crate) id: u32,
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) category: u32,
    pub(crate) subcategory: u32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RemoveDispatchParams {
    pub(crate) id: u32,
}