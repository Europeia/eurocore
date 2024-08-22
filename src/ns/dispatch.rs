use crate::core::error::Error;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub(crate) enum FactbookCategory {
    Factbook(FactbookSubcategory), // 1
    Bulletin(BulletinSubcategory), // 3
    Account(AccountSubcategory),   // 5
    Meta(MetaSubcategory),         // 8
}

impl FactbookCategory {
    fn to_tuple(&self) -> (i16, i16) {
        match self {
            FactbookCategory::Factbook(subcategory) => match subcategory {
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
            },
            FactbookCategory::Bulletin(subcategory) => match subcategory {
                BulletinSubcategory::Policy => (3, 305),
                BulletinSubcategory::News => (3, 315),
                BulletinSubcategory::Opinion => (3, 325),
                BulletinSubcategory::Campaign => (3, 385),
            },
            FactbookCategory::Account(subcategory) => match subcategory {
                AccountSubcategory::Military => (5, 505),
                AccountSubcategory::Trade => (5, 515),
                AccountSubcategory::Sport => (5, 525),
                AccountSubcategory::Drama => (5, 535),
                AccountSubcategory::Diplomacy => (5, 545),
                AccountSubcategory::Science => (5, 555),
                AccountSubcategory::Culture => (5, 565),
                AccountSubcategory::Other => (5, 595),
            },
            FactbookCategory::Meta(subcategory) => match subcategory {
                MetaSubcategory::Gameplay => (8, 835),
                MetaSubcategory::Reference => (8, 845),
            },
        }
    }
}

impl TryFrom<(i16, i16)> for FactbookCategory {
    type Error = Error;

    fn try_from((category, subcategory): (i16, i16)) -> Result<Self, Self::Error> {
        match category {
            1 => match subcategory {
                100 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Overview)),
                101 => Ok(FactbookCategory::Factbook(FactbookSubcategory::History)),
                102 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Geography)),
                103 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Culture)),
                104 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Politics)),
                105 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Legislation)),
                106 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Religion)),
                107 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Military)),
                108 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Economy)),
                109 => Ok(FactbookCategory::Factbook(
                    FactbookSubcategory::International,
                )),
                110 => Ok(FactbookCategory::Factbook(FactbookSubcategory::Trivia)),
                111 => Ok(FactbookCategory::Factbook(
                    FactbookSubcategory::Miscellaneous,
                )),
                _ => Err(Error::InvalidFactbookCategory),
            },
            3 => match subcategory {
                305 => Ok(FactbookCategory::Bulletin(BulletinSubcategory::Policy)),
                315 => Ok(FactbookCategory::Bulletin(BulletinSubcategory::News)),
                325 => Ok(FactbookCategory::Bulletin(BulletinSubcategory::Opinion)),
                385 => Ok(FactbookCategory::Bulletin(BulletinSubcategory::Campaign)),
                _ => Err(Error::InvalidFactbookCategory),
            },
            5 => match subcategory {
                505 => Ok(FactbookCategory::Account(AccountSubcategory::Military)),
                515 => Ok(FactbookCategory::Account(AccountSubcategory::Trade)),
                525 => Ok(FactbookCategory::Account(AccountSubcategory::Sport)),
                535 => Ok(FactbookCategory::Account(AccountSubcategory::Drama)),
                545 => Ok(FactbookCategory::Account(AccountSubcategory::Diplomacy)),
                555 => Ok(FactbookCategory::Account(AccountSubcategory::Science)),
                565 => Ok(FactbookCategory::Account(AccountSubcategory::Culture)),
                595 => Ok(FactbookCategory::Account(AccountSubcategory::Other)),
                _ => Err(Error::InvalidFactbookCategory),
            },
            8 => match subcategory {
                835 => Ok(FactbookCategory::Meta(MetaSubcategory::Gameplay)),
                845 => Ok(FactbookCategory::Meta(MetaSubcategory::Reference)),
                _ => Err(Error::InvalidFactbookCategory),
            },
            _ => Err(Error::InvalidFactbookCategory),
        }
    }
}

#[derive(Clone)]
pub(crate) enum FactbookSubcategory {
    Overview,      // 100
    History,       // 101
    Geography,     // 102
    Culture,       // 103
    Politics,      // 104
    Legislation,   // 105
    Religion,      // 106
    Military,      // 107
    Economy,       // 108
    International, // 109
    Trivia,        // 110
    Miscellaneous, // 111
}

#[derive(Clone)]
pub(crate) enum BulletinSubcategory {
    Policy,   // 305
    News,     // 315
    Opinion,  // 325
    Campaign, // 385
}

#[derive(Clone)]
pub(crate) enum AccountSubcategory {
    Military,  // 505
    Trade,     // 515
    Sport,     // 525
    Drama,     // 535
    Diplomacy, // 545
    Science,   // 555
    Culture,   // 565
    Other,     // 595
}

#[derive(Clone)]
pub(crate) enum MetaSubcategory {
    Gameplay,  // 835
    Reference, // 845
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

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct NewDispatch {
    pub(crate) nation: String,
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) category: i16,
    pub(crate) subcategory: i16,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct EditDispatch {
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) category: i16,
    pub(crate) subcategory: i16,
}

#[derive(Clone)]
pub(crate) enum Action {
    Add {
        nation: String,
        title: String,
        text: String,
        category: FactbookCategory,
    },
    Edit {
        id: i32,
        nation: String,
        title: String,
        text: String,
        category: FactbookCategory,
    },
    Remove {
        id: i32,
        nation: String,
    },
}

impl Action {
    pub(crate) fn add(params: NewDispatch) -> Result<Self, Error> {
        Ok(Action::Add {
            nation: params.nation,
            title: params.title,
            text: params.text,
            category: FactbookCategory::try_from((params.category, params.subcategory))?,
        })
    }

    pub(crate) fn edit(id: i32, nation: String, params: EditDispatch) -> Result<Self, Error> {
        Ok(Action::Edit {
            id,
            nation,
            title: params.title,
            text: params.text,
            category: FactbookCategory::try_from((params.category, params.subcategory))?,
        })
    }

    pub(crate) fn remove(id: i32, nation: String) -> Self {
        Action::Remove { id, nation }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Dispatch {
    #[serde(rename = "dispatchid", skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<i32>,
    pub(crate) nation: String,
    #[serde(rename = "c")]
    command: String,
    #[serde(rename = "dispatch")]
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) category: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) subcategory: Option<i16>,
    mode: Mode,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
}

impl Dispatch {
    pub(crate) fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    pub(crate) fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }
}

impl From<Action> for Dispatch {
    fn from(action: Action) -> Dispatch {
        match action {
            Action::Add {
                nation,
                title,
                text,
                category,
            } => Dispatch {
                id: None,
                nation,
                command: "dispatch".to_string(),
                action: "add".to_string(),
                title: Some(title),
                text: Some(text),
                category: Some(category.to_tuple().0),
                subcategory: Some(category.to_tuple().1),
                mode: Mode::Prepare,
                token: None,
            },
            Action::Edit {
                id,
                nation,
                title,
                text,
                category,
            } => Dispatch {
                id: Some(id),
                nation,
                command: "dispatch".to_string(),
                action: "edit".to_string(),
                title: Some(title),
                text: Some(text),
                category: Some(category.to_tuple().0),
                subcategory: Some(category.to_tuple().1),
                mode: Mode::Prepare,
                token: None,
            },
            Action::Remove { id, nation } => Dispatch {
                id: Some(id),
                nation,
                command: "dispatch".to_string(),
                action: "remove".to_string(),
                title: None,
                text: None,
                category: None,
                subcategory: None,
                mode: Mode::Prepare,
                token: None,
            },
        }
    }
}
