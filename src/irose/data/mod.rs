mod ability_values;
mod ai_database;
mod character_creator;
mod item_database;
mod motion_database;
mod npc_database;
mod skill_database;
mod zone_database;

use crate::data::ItemReference;
use num_traits::FromPrimitive;

enum DecodeItemReferenceError {
    Empty,
    InvalidItemType,
    InvalidItemNumber,
}

fn decode_item_reference(value: u32) -> Result<ItemReference, DecodeItemReferenceError> {
    if value == 0 {
        Err(DecodeItemReferenceError::Empty)
    } else {
        let item_type = FromPrimitive::from_u32(value / 1000)
            .ok_or(DecodeItemReferenceError::InvalidItemType)?;
        let item_number = value % 1000;
        if item_number == 0 {
            Err(DecodeItemReferenceError::InvalidItemNumber)
        } else {
            Ok(ItemReference::new(item_type, item_number as usize))
        }
    }
}

pub use ability_values::get_ability_value_calculator;
pub use ai_database::get_ai_database;
pub use character_creator::get_character_creator;
pub use item_database::get_item_database;
pub use motion_database::get_motion_database;
pub use npc_database::get_npc_database;
pub use skill_database::get_skill_database;
pub use zone_database::get_zone_database;
