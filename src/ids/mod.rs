//! Auto generated with `generate_ids.py` script from `stableid.json`
//! ids of units, ablities, upgrades, buffs and effects.
#![allow(missing_docs)]

mod ability_id;
mod buff_id;
mod effect_id;
mod unit_typeid;
mod upgrade_id;

pub use ability_id::AbilityId;
pub use buff_id::BuffId;
pub use effect_id::EffectId;
pub use unit_typeid::UnitTypeId;
pub use upgrade_id::UpgradeId;

mod impls;
