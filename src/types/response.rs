use serde::Serialize;

#[derive(Default, Serialize)]
pub(crate) struct DispatchHeader {
    pub(crate) id: i32,
    pub(crate) nation: String,
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
