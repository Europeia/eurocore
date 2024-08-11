use serde::Serialize;

#[derive(Default, Serialize)]
pub(crate) struct DispatchHeader {
    pub(crate) id: i32,
    pub(crate) nation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) category: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) subcategory: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created_by: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct Dispatch {
    pub(crate) id: i32,
    pub(crate) nation: String,
    pub(crate) category: i16,
    pub(crate) subcategory: i16,
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) created_by: String,
}
