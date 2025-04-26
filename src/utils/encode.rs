use htmlentity::entity::{CharacterSet, EncodeType, ICodedDataTrait};

pub(crate) fn encode(input: &str) -> String {
    input
        .chars()
        .map(|char| {
            if char.is_ascii() {
                char.to_string()
            } else {
                htmlentity::entity::encode(
                    char.encode_utf8(&mut [0; 4]).as_bytes(),
                    &EncodeType::Decimal,
                    &CharacterSet::All,
                )
                .to_string()
                .unwrap()
            }
        })
        .collect()
}
