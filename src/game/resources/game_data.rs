use std::sync::Arc;

use crate::data::{
    AbilityValueCalculator, AiDatabase, CharacterCreator, DropTable, ItemDatabase, MotionDatabase,
    NpcDatabase, QuestDatabase, SkillDatabase, StatusEffectDatabase, ZoneDatabase,
};

pub struct GameData {
    pub character_creator: Box<dyn CharacterCreator + Send + Sync>,
    pub ability_value_calculator: Box<dyn AbilityValueCalculator + Send + Sync>,
    pub drop_table: Box<dyn DropTable + Send + Sync>,
    pub ai: Arc<AiDatabase>,
    pub items: Arc<ItemDatabase>,
    pub motions: Arc<MotionDatabase>,
    pub npcs: Arc<NpcDatabase>,
    pub quests: Arc<QuestDatabase>,
    pub skills: Arc<SkillDatabase>,
    pub status_effects: Arc<StatusEffectDatabase>,
    pub zones: Arc<ZoneDatabase>,
}
