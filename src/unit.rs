//! Stuff for convenient interaction with [`Unit`]s.
#![allow(missing_docs)]

use crate::{
	action::{Commander, Target},
	bot::{LockBool, LockOwned, LockU32, Locked, Reader, Rl, Rs, Rw},
	consts::{
		RaceValues, ANTI_ARMOR_BUFF, DAMAGE_BONUS_PER_UPGRADE, FRAMES_PER_SECOND, MISSED_WEAPONS,
		OFF_CREEP_SPEED_UPGRADES, SPEED_BUFFS, SPEED_ON_CREEP, SPEED_UPGRADES, WARPGATE_ABILITIES,
	},
	distance::Distance,
	game_data::{Attribute, Cost, GameData, TargetType, UnitTypeData, Weapon},
	game_state::Alliance,
	geometry::{Point2, Point3},
	ids::{AbilityId, BuffId, UnitTypeId, UpgradeId},
	pixel_map::{PixelMap, VisibilityMap},
	player::Race,
	units::Container,
	utils::CacheMap,
	FromProto,
};
use lazy_init::Lazy as LazyInit;
use num_traits::FromPrimitive;
use once_cell::sync::Lazy;
use rustc_hash::{FxHashMap, FxHashSet};
use sc2_proto::raw::{
	CloakState as ProtoCloakState, DisplayType as ProtoDisplayType, Unit as ProtoUnit,
	UnitOrder_oneof_target as ProtoTarget,
};
use std::cmp::Ordering;
use crate::consts::ON_CREEP_SPEED_UPGRADES;

#[derive(Default, Debug, Clone, Copy)]
pub struct WeaponStats {
	pub damage: u32,
	pub speed: f32,
	pub range: f32,
}

impl WeaponStats {
	pub fn dps(&self) -> f32 {
		self.damage as f32 / self.speed
	}
}

#[derive(Default, Clone)]
pub(crate) struct DataForUnit {
	pub commander: Rw<Commander>,
	pub game_data: Rs<GameData>,
	pub techlab_tags: Rw<FxHashSet<u64>>,
	pub reactor_tags: Rw<FxHashSet<u64>>,
	pub race_values: Rs<RaceValues>,
	pub max_cooldowns: Rw<FxHashMap<UnitTypeId, f32>>,
	pub last_units_hits: Rw<FxHashMap<u64, u32>>,
	pub last_units_seen: Rw<FxHashMap<u64, u32>>,
	pub abilities_units: Rw<FxHashMap<u64, FxHashSet<AbilityId>>>,
	pub upgrades: Rw<FxHashSet<UpgradeId>>,
	pub enemy_upgrades: Rw<FxHashSet<UpgradeId>>,
	pub creep: Rw<PixelMap>,
	pub game_step: Rs<LockU32>,
	pub game_loop: Rs<LockU32>,
	pub available_frames: Rw<FxHashMap<u64, u32>>,
}

pub(crate) struct UnitBase {
	pub display_type: Rl<DisplayType>,
	pub alliance: Alliance,
	pub tag: u64,
	pub type_id: Rl<UnitTypeId>,
	pub owner: u32,
	pub position: Point2,
	pub position3d: Point3,
	pub facing: f32,
	pub radius: f32,
	pub build_progress: f32,
	pub is_cloaked: LockBool,
	pub is_revealed: LockBool,
	pub buffs: FxHashSet<BuffId>,
	pub detect_range: f32,
	pub radar_range: f32,
	pub is_selected: bool,
	pub is_on_screen: bool,
	pub is_blip: bool,
	pub is_powered: bool,
	pub is_active: bool,
	pub attack_upgrade_level: u32,
	pub armor_upgrade_level: i32,
	pub shield_upgrade_level: i32,
	pub health: Option<u32>,
	pub health_max: Option<u32>,
	pub shield: Option<u32>,
	pub shield_max: Option<u32>,
	pub energy: Option<u32>,
	pub energy_max: Option<u32>,
	pub mineral_contents: Option<u32>,
	pub vespene_contents: Option<u32>,
	pub is_flying: bool,
	pub is_burrowed: LockBool,
	pub is_hallucination: LockBool,
	pub orders: Vec<UnitOrder>,
	pub addon_tag: Option<u64>,
	pub passengers: Vec<PassengerUnit>,
	pub cargo_space_taken: Option<u32>,
	pub cargo_space_max: Option<u32>,
	pub assigned_harvesters: Option<u32>,
	pub ideal_harvesters: Option<u32>,
	pub weapon_cooldown: Option<f32>,
	pub engaged_target_tag: Option<u64>,
	pub buff_duration_remain: Option<u32>,
	pub buff_duration_max: Option<u32>,
	pub rally_targets: Vec<RallyTarget>,

	// cache
	real_speed: LazyInit<f32>,
	on_creep_speed: LazyInit<f32>,
	off_creep_speed: LazyInit<f32>,
	real_weapon_vs: Lazy<CacheMap<u64, WeaponStats>>,
}

/// Weapon target used in [`calculate_weapon_stats`](Unit::calculate_weapon_stats).
pub enum CalcTarget<'a> {
	/// Specific unit.
	Unit(&'a Unit),
	/// Abstract target with given type and attributes.
	Abstract(TargetType, &'a [Attribute]),
}

pub(crate) type SharedUnitData = Rs<DataForUnit>;

/// Unit structure contains some raw data, helper methods for it's analysis
/// and some methods for actions execution.
#[derive(Clone)]
pub struct Unit {
	data: SharedUnitData,
	pub(crate) base: Rs<UnitBase>,
}

impl Unit {
	/////////////////////////////////////////////////
	// Fields are populated based on type/alliance //
	/////////////////////////////////////////////////
	/// How unit is displayed (i.e. visibility of unit).
	#[inline]
	pub fn display_type(&self) -> DisplayType {
		*self.base.display_type.read_lock()
	}
	/// Unit is owned, enemy or just neutral.
	#[inline]
	pub fn alliance(&self) -> Alliance {
		self.base.alliance
	}

	/// Unique and constant for each unit tag. Used to find exactly the same unit in bunch of [`Units`].
	/// See also [`get`], [`get_mut`] and [`find_tags`].
	///
	/// [`Units`]: crate::units::Units
	/// [`get`]: crate::units::Units::get
	/// [`get_mut`]: crate::units::Units::get_mut
	/// [`find_tags`]: crate::units::Units::find_tags
	#[inline]
	pub fn tag(&self) -> u64 {
		self.base.tag
	}
	/// The type of unit.
	#[inline]
	pub fn type_id(&self) -> UnitTypeId {
		*self.base.type_id.read_lock()
	}
	/// Player id of the owner. Normally it should match your [`player_id`] for owned units
	/// and [`enemy_player_id`] for opponent's units.
	///
	/// [`player_id`]: crate::bot::Bot::player_id
	/// [`enemy_player_id`]: crate::bot::Bot::enemy_player_id
	#[inline]
	pub fn owner(&self) -> u32 {
		self.base.owner
	}
	/// Position on 2D grid.
	#[inline]
	pub fn position(&self) -> Point2 {
		self.base.position
	}
	/// Position in 3D world space.
	#[inline]
	pub fn position3d(&self) -> Point3 {
		self.base.position3d
	}
	/// Unit rotation angle (i.e. the direction unit is facing).
	/// Value in range `[0, 2π)`.
	#[inline]
	pub fn facing(&self) -> f32 {
		self.base.facing
	}
	/// Radius of the unit.
	#[inline]
	pub fn radius(&self) -> f32 {
		self.base.radius
	}
	/// The progress of building construction. Value from `0` to `1`.
	#[inline]
	pub fn build_progress(&self) -> f32 {
		self.base.build_progress
	}
	/// `true` when unit is burrowed or has cloak field turned on.
	#[inline]
	pub fn is_cloaked(&self) -> bool {
		self.base.is_cloaked.get_locked()
	}
	/// `true` when unit is detected.
	#[inline]
	pub fn is_revealed(&self) -> bool {
		self.base.is_revealed.get_locked()
	}
	/// Set of buffs unit has.
	#[inline]
	pub fn buffs(&self) -> &FxHashSet<BuffId> {
		&self.base.buffs
	}
	/// Detection range of detector or `0` if unit is not detector.
	/// See also [`is_detector`](Self::is_detector).
	#[inline]
	pub fn detect_range(&self) -> f32 {
		self.base.detect_range
	}
	/// Range of terran's sensor tower.
	#[inline]
	pub fn radar_range(&self) -> f32 {
		self.base.radar_range
	}
	/// Unit is selected.
	#[inline]
	pub fn is_selected(&self) -> bool {
		self.base.is_selected
	}
	/// Unit is visible in game window.
	#[inline]
	pub fn is_on_screen(&self) -> bool {
		self.base.is_on_screen
	}
	/// Enemies detected by sensor tower.
	#[inline]
	pub fn is_blip(&self) -> bool {
		self.base.is_blip
	}
	/// Protoss structure is powered by pylon.
	#[inline]
	pub fn is_powered(&self) -> bool {
		self.base.is_powered
	}
	/// Building is training/researching (i.e. animated).
	#[inline]
	pub fn is_active(&self) -> bool {
		self.base.is_active
	}
	/// General attack upgrade level without considering buffs and special upgrades.
	#[inline]
	pub fn attack_upgrade_level(&self) -> u32 {
		self.base.attack_upgrade_level
	}
	/// General armor upgrade level without considering buffs and special upgrades.
	#[inline]
	pub fn armor_upgrade_level(&self) -> i32 {
		self.base.armor_upgrade_level
	}
	/// General shield upgrade level without considering buffs and special upgrades.
	#[inline]
	pub fn shield_upgrade_level(&self) -> i32 {
		self.base.shield_upgrade_level
	}

	/////////////////////////////////
	// Not populated for snapshots //
	/////////////////////////////////
	/// Current health of unit.
	///
	/// Note: Not populated for snapshots.
	#[inline]
	pub fn health(&self) -> Option<u32> {
		self.base.health
	}
	/// Maximum health of unit.
	///
	/// Note: Not populated for snapshots.
	#[inline]
	pub fn health_max(&self) -> Option<u32> {
		self.base.health_max
	}
	/// Current shield of protoss unit.
	///
	/// Note: Not populated for snapshots.
	#[inline]
	pub fn shield(&self) -> Option<u32> {
		self.base.shield
	}
	/// Maximum shield of protoss unit.
	///
	/// Note: Not populated for snapshots.
	#[inline]
	pub fn shield_max(&self) -> Option<u32> {
		self.base.shield_max
	}
	/// Current energy of caster unit.
	///
	/// Note: Not populated for snapshots.
	#[inline]
	pub fn energy(&self) -> Option<u32> {
		self.base.energy
	}
	/// Maximum energy of caster unit.
	///
	/// Note: Not populated for snapshots.
	#[inline]
	pub fn energy_max(&self) -> Option<u32> {
		self.base.energy_max
	}
	/// Amount of minerals left in mineral field.
	///
	/// Note: Not populated for snapshots.
	#[inline]
	pub fn mineral_contents(&self) -> Option<u32> {
		self.base.mineral_contents
	}
	/// Amount of vespene gas left in vespene geyser.
	///
	/// Note: Not populated for snapshots.
	#[inline]
	pub fn vespene_contents(&self) -> Option<u32> {
		self.base.vespene_contents
	}
	/// Unit is flying.
	///
	/// Note: Not populated for snapshots.
	#[inline]
	pub fn is_flying(&self) -> bool {
		self.base.is_flying
	}
	/// Zerg unit is burrowed.
	///
	/// Note: Not populated for snapshots.
	#[inline]
	pub fn is_burrowed(&self) -> bool {
		self.base.is_burrowed.get_locked()
	}
	/// Is hallucination created by protoss sentry.
	///
	/// Note: Not populated for snapshots.
	#[inline]
	pub fn is_hallucination(&self) -> bool {
		self.base.is_hallucination.get_locked()
	}

	///////////////////////////////
	// Not populated for enemies //
	///////////////////////////////
	/// Current orders of unit.
	///
	/// Note: Not populated for enemies.
	#[inline]
	pub fn orders(&self) -> &[UnitOrder] {
		&self.base.orders
	}
	/// Tag of addon if any.
	///
	/// Note: Not populated for enemies.
	#[inline]
	pub fn addon_tag(&self) -> Option<u64> {
		self.base.addon_tag
	}
	/// Units inside transport or bunker.
	///
	/// Note: Not populated for enemies.
	#[inline]
	pub fn passengers(&self) -> &[PassengerUnit] {
		&self.base.passengers
	}
	/// Used space of transport or bunker.
	///
	/// Note: Not populated for enemies.
	#[inline]
	pub fn cargo_space_taken(&self) -> Option<u32> {
		self.base.cargo_space_taken
	}
	/// Maximum space of transport or bunker.
	///
	/// Note: Not populated for enemies.
	#[inline]
	pub fn cargo_space_max(&self) -> Option<u32> {
		self.base.cargo_space_max
	}
	/// Current number of workers on gas or base.
	///
	/// Note: Not populated for enemies.
	#[inline]
	pub fn assigned_harvesters(&self) -> Option<u32> {
		self.base.assigned_harvesters
	}
	/// Ideal number of workers on gas or base.
	///
	/// Note: Not populated for enemies.
	#[inline]
	pub fn ideal_harvesters(&self) -> Option<u32> {
		self.base.ideal_harvesters
	}
	/// Frames left until weapon will be ready to shot.
	///
	/// Note: Not populated for enemies.
	#[inline]
	pub fn weapon_cooldown(&self) -> Option<f32> {
		self.base.weapon_cooldown
	}
	#[inline]
	pub fn engaged_target_tag(&self) -> Option<u64> {
		self.base.engaged_target_tag
	}
	/// How long a buff or unit is still around (e.g. mule, broodling, chronoboost).
	///
	/// Note: Not populated for enemies.
	#[inline]
	pub fn buff_duration_remain(&self) -> Option<u32> {
		self.base.buff_duration_remain
	}
	/// How long the maximum duration of buff or unit (e.g. mule, broodling, chronoboost).
	///
	/// Note: Not populated for enemies.
	#[inline]
	pub fn buff_duration_max(&self) -> Option<u32> {
		self.base.buff_duration_max
	}
	/// All rally points of structure.
	///
	/// Note: Not populated for enemies.
	#[inline]
	pub fn rally_targets(&self) -> &[RallyTarget] {
		&self.base.rally_targets
	}

	fn type_data(&self) -> Option<&UnitTypeData> {
		self.data.game_data.units.get(&self.type_id())
	}
	pub fn upgrades(&self) -> Reader<FxHashSet<UpgradeId>> {
		if self.is_mine() {
			self.data.upgrades.read_lock()
		} else {
			self.data.enemy_upgrades.read_lock()
		}
	}
	/// Name of the unit
	pub fn name(&self) -> &str {
		self.type_data().map_or("", |data| &data.name)
	}
	/// Checks if unit is worker.
	pub fn is_worker(&self) -> bool {
		self.type_id().is_worker()
	}
	/// Checks if it's townhall.
	pub fn is_townhall(&self) -> bool {
		self.type_id().is_townhall()
	}
	/// Checks if it's addon.
	pub fn is_addon(&self) -> bool {
		self.type_id().is_addon()
	}
	/// Checks if unit is melee attacker.
	pub fn is_melee(&self) -> bool {
		self.type_id().is_melee()
	}
	/// Checks if it's mineral field.
	pub fn is_mineral(&self) -> bool {
		self.type_data().map_or(false, |data| data.has_minerals)
	}
	/// Checks if it's vespene geyser.
	pub fn is_geyser(&self) -> bool {
		self.type_data().map_or(false, |data| data.has_vespene)
	}
	/// Checks if unit is detector.
	#[rustfmt::skip::macros(matches)]
	pub fn is_detector(&self) -> bool {
		matches!(
			self.type_id(),
			UnitTypeId::Observer
				| UnitTypeId::ObserverSiegeMode
				| UnitTypeId::Raven
				| UnitTypeId::Overseer
				| UnitTypeId::OverseerSiegeMode
		) || (self.is_almost_ready()
			&& (matches!(
				self.type_id(),
				UnitTypeId::MissileTurret | UnitTypeId::SporeCrawler
			) || (matches!(self.type_id(), UnitTypeId::PhotonCannon) && self.is_powered())))
	}
	/// Building construction is complete.
	pub fn is_ready(&self) -> bool {
		(self.build_progress() - 1.0).abs() < f32::EPSILON
	}
	/// Building construction is more than 95% complete.
	pub fn is_almost_ready(&self) -> bool {
		self.build_progress() >= 0.95
	}
	/// Terran building has addon.
	pub fn has_addon(&self) -> bool {
		self.addon_tag().is_some()
	}
	/// Terran building's addon is techlab if any.
	pub fn has_techlab(&self) -> bool {
		let techlab_tags = self.data.techlab_tags.read_lock();
		self.addon_tag().map_or(false, |tag| techlab_tags.contains(&tag))
	}
	/// Terran building's addon is reactor if any.
	pub fn has_reactor(&self) -> bool {
		let reactor_tags = self.data.reactor_tags.read_lock();
		self.addon_tag().map_or(false, |tag| reactor_tags.contains(&tag))
	}
	/// Unit was attacked on last step.
	pub fn is_attacked(&self) -> bool {
		self.hits() < self.data.last_units_hits.read_lock().get(&self.tag()).copied()
	}
	/// The damage was taken by unit if it was attacked, otherwise it's `0`.
	pub fn damage_taken(&self) -> u32 {
		let hits = match self.hits() {
			Some(hits) => hits,
			None => return 0,
		};
		let last_hits = match self.data.last_units_hits.read_lock().get(&self.tag()).copied() {
			Some(hits) => hits,
			None => return 0,
		};
		last_hits.saturating_sub(hits)
	}
	/// Unit was attacked on last step.
	pub fn time_alive(&self) -> u32 {
		if let Some(initially_seen) = self.data.last_units_seen.read_lock().get(&self.tag()).copied() {
			self.data.game_loop.get_locked().saturating_sub(initially_seen)
		} else {
			0
		}
	}
	/// Abilities available for unit to use.
	///
	/// Ability won't be available if it's on cooldown, unit
	/// is out of energy or bot doesn't have enough resources.
	pub fn abilities(&self) -> Option<FxHashSet<AbilityId>> {
		self.data.abilities_units.read_lock().get(&self.tag()).cloned()
	}
	/// Checks if ability is available for unit.
	///
	/// Ability won't be available if it's on cooldown, unit
	/// is out of energy or bot doesn't have enough resources.
	pub fn has_ability(&self, ability: AbilityId) -> bool {
		self.data
			.abilities_units
			.read_lock()
			.get(&self.tag())
			.map_or(false, |abilities| abilities.contains(&ability))
	}
	/// Race of unit, dependent on it's type.
	pub fn race(&self) -> Race {
		self.type_data().map_or(Race::Random, |data| data.race)
	}
	/// There're some units inside transport or bunker.
	pub fn has_cargo(&self) -> bool {
		self.cargo_space_taken().map_or(false, |taken| taken > 0)
	}
	/// Free space left in transport or bunker.
	pub fn cargo_left(&self) -> Option<u32> {
		Some(self.cargo_space_max()? - self.cargo_space_taken()?)
	}
	/// Half of [`building_size`](Self::building_size), but `2.5` for addons.
	pub fn footprint_radius(&self) -> Option<f32> {
		self.type_data().and_then(|data| {
			data.ability.and_then(|ability| {
				self.data
					.game_data
					.abilities
					.get(&ability)
					.and_then(|ability_data| ability_data.footprint_radius)
			})
		})
	}
	/// Correct building size in tiles
	/// (e.g. `2` for supply and addons, `3` for barracks, `5` for command center).
	pub fn building_size(&self) -> Option<usize> {
		if self.is_addon() {
			Some(2)
		} else {
			self.footprint_radius().map(|radius| (radius * 2.0) as usize)
		}
	}
	/// How long a unit takes to build.
	pub fn build_time(&self) -> f32 {
		self.type_data().map_or(0.0, |data| data.build_time)
	}
	/// Space that unit takes in transports and bunkers.
	pub fn cargo_size(&self) -> u32 {
		self.type_data().map_or(0, |data| data.cargo_size)
	}
	/// How far unit can see.
	pub fn sight_range(&self) -> f32 {
		self.type_data().map_or(0.0, |data| data.sight_range)
	}
	/// Initial armor of unit without considering upgrades and buffs.
	pub fn armor(&self) -> i32 {
		self.type_data().map_or(0, |data| data.armor)
	}
	/// Returns point with given offset towards unit face direction.
	pub fn towards_facing(&self, offset: f32) -> Point2 {
		self.position()
			.offset(offset * self.facing().cos(), offset * self.facing().sin())
	}
	/// Checks if unit is fully visible.
	pub fn is_visible(&self) -> bool {
		self.display_type().is_visible()
	}
	/// Checks if unit is snapshot (i.e. hidden in fog of war or on high ground).
	pub fn is_snapshot(&self) -> bool {
		self.display_type().is_snapshot()
	}
	/// Checks if unit is fully hidden.
	pub fn is_hidden(&self) -> bool {
		self.display_type().is_hidden()
	}
	/// Checks if unit is building placeholder.
	pub fn is_placeholder(&self) -> bool {
		self.display_type().is_placeholder()
	}
	/// Checks if unit is owned.
	pub fn is_mine(&self) -> bool {
		self.alliance().is_mine()
	}
	/// Checks if unit is enemy.
	pub fn is_enemy(&self) -> bool {
		self.alliance().is_enemy()
	}
	/// Checks if unit is neutral.
	pub fn is_neutral(&self) -> bool {
		self.alliance().is_neutral()
	}
	/// Checks if unit is allied, but not owned.
	pub fn is_ally(&self) -> bool {
		self.alliance().is_ally()
	}

	/// Checks if unit is detected or not even cloaked.
	#[inline]
	pub fn can_be_attacked(&self) -> bool {
		self.is_revealed() || !self.is_cloaked()
	}
	/// Checks if unit is burrowed or cloaked, and not detected (i.e. must be detected to be attacked).
	#[inline]
	pub fn is_invisible(&self) -> bool {
		self.is_cloaked() && !self.is_revealed()
	}

	/// Returns how much supply this unit uses.
	pub fn supply_cost(&self) -> f32 {
		self.type_data().map_or(0.0, |data| data.food_required)
	}
	/// Returns how much supply this unit uses.
	pub fn supply_provided(&self) -> f32 {
		self.type_data().map_or(0.0, |data| data.food_provided)
	}
	/// Returns cost of unit.
	pub fn cost(&self) -> Cost {
		self.type_data().map_or(Cost::default(), |data| data.cost())
	}
	/// Returns health percentage (current health divided by max health).
	/// Value in range from `0` to `1`.
	pub fn health_percentage(&self) -> Option<f32> {
		let current = self.health()?;
		let max = self.health_max()?;
		if max == 0 {
			return None;
		}
		Some(current as f32 / max as f32)
	}
	/// Returns shield percentage (current shield divided by max shield).
	/// Value in range from `0` to `1`.
	pub fn shield_percentage(&self) -> Option<f32> {
		let current = self.shield()?;
		let max = self.shield_max()?;
		if max == 0 {
			return None;
		}
		Some(current as f32 / max as f32)
	}
	/// Returns energy percentage (current energy divided by max energy).
	/// Value in range from `0` to `1`.
	pub fn energy_percentage(&self) -> Option<f32> {
		let current = self.energy()?;
		let max = self.energy_max()?;
		if max == 0 {
			return None;
		}
		Some(current as f32 / max as f32)
	}
	/// Returns summed health and shield.
	///
	/// Not populated for snapshots.
	pub fn hits(&self) -> Option<u32> {
		let extra_shield = if self.has_buff(BuffId::ImmortalShield) {
			100
		} else {
			0
		};
		match (self.health(), self.shield()) {
			(Some(health), Some(shield)) => Some(health + shield + extra_shield),
			(Some(health), None) => Some(health + extra_shield),
			(None, Some(shield)) => Some(shield + extra_shield),
			(None, None) => None,
		}
	}
	/// Returns summed max health and max shield.
	///
	/// Not populated for snapshots.
	pub fn hits_max(&self) -> Option<u32> {
		match (self.health_max(), self.shield_max()) {
			(Some(health), Some(shield)) => Some(health + shield),
			(Some(health), None) => Some(health),
			(None, Some(shield)) => Some(shield),
			(None, None) => None,
		}
	}
	/// Returns percentage of summed health and shield (current hits divided by max hits).
	/// Value in range from `0` to `1`.
	///
	/// Not populated for snapshots.
	pub fn hits_percentage(&self) -> Option<f32> {
		let current = self.hits()?;
		let max = self.hits_max()?;
		if max == 0 {
			return None;
		}
		Some(current as f32 / max as f32)
	}
	/// Basic speed of the unit without considering buffs and upgrades.
	///
	/// Use [`real_speed`](Self::real_speed) to get speed including buffs and upgrades.
	pub fn speed(&self) -> f32 {
		self.type_data().map_or(0.0, |data| data.movement_speed)
	}
	pub fn is_unit_on_creep(&self) -> bool {
		self.data.creep.read_lock()[self.position()].is_empty()
	}
	pub fn on_creep_speed(&self) -> f32 {
		*self.base.on_creep_speed.get_or_create(|| {
			let unit_type = self.type_id();
			let mut base_speed = self.base_real_speed();
			let upgrades = self.upgrades();
			// Hydralisks speed upgrade bonus is lower on creep
			if let Some((upgrade_id, increase)) = ON_CREEP_SPEED_UPGRADES.get(&unit_type) {
				if upgrades.contains(upgrade_id) {
					base_speed *= increase;
				}
			}
			if let Some(increase) = SPEED_ON_CREEP.get(&unit_type) {
				return base_speed * increase;
			}
			base_speed
		})
	}
	pub fn off_creep_speed(&self) -> f32 {
		*self.base.off_creep_speed.get_or_create(|| {
			let unit_type = self.type_id();

			// Off creep upgrades
			let upgrades = self.upgrades();
			let base_speed = self.base_real_speed();
			if let Some((upgrade_id, increase)) = OFF_CREEP_SPEED_UPGRADES.get(&unit_type) {
				if upgrades.contains(upgrade_id) {
					return base_speed * increase;
				}
			}
			base_speed
		})
	}
	/// Returns actual speed of the unit calculated including buffs and upgrades.
	pub fn base_real_speed(&self) -> f32 {
		*self.base.real_speed.get_or_create(|| {
			let mut speed = self.speed();
			let unit_type = self.type_id();

			// ---- Buffs ----
			// Ultralisk has passive ability "Frenzied" which makes it immune to speed altering buffs
			if unit_type != UnitTypeId::Ultralisk {
				for buff in self.buffs() {
					match buff {
						BuffId::MedivacSpeedBoost => return speed * 1.7,
						BuffId::VoidRaySwarmDamageBoost => return speed * 0.75,
						_ => {
							if let Some(increase) = SPEED_BUFFS.get(buff) {
								speed *= increase;
							}
						}
					}
				}
			}

			// ---- Upgrades ----
			let upgrades = self.upgrades();
			if let Some((upgrade_id, increase)) = SPEED_UPGRADES.get(&unit_type) {
				if upgrades.contains(upgrade_id) {
					speed *= increase;
				}
			}

			speed
		})
	}
	/// Returns actual speed of the unit calculated including buffs and upgrades.
	pub fn real_speed(&self) -> f32 {
		if self.is_unit_on_creep() {
			self.on_creep_speed()
		} else {
			self.off_creep_speed()
		}
	}
	/// Distance unit can travel per one step.
	pub fn distance_per_step(&self) -> f32 {
		self.real_speed() / FRAMES_PER_SECOND * self.data.game_step.get_locked() as f32
	}
	/// Distance unit can travel until weapons be ready to fire.
	pub fn distance_to_weapon_ready(&self) -> f32 {
		self.real_speed() / FRAMES_PER_SECOND * self.weapon_cooldown().unwrap_or(0.0)
	}
	/// Attributes of unit, dependent on it's type.
	pub fn attributes(&self) -> &[Attribute] {
		self.type_data().map_or(&[], |data| data.attributes.as_slice())
	}
	/// Checks if unit has given attribute.
	pub fn has_attribute(&self, attribute: Attribute) -> bool {
		self.type_data()
			.map_or(false, |data| data.attributes.contains(&attribute))
	}
	/// Checks if unit has `Light` attribute.
	pub fn is_light(&self) -> bool {
		self.has_attribute(Attribute::Light)
	}
	/// Checks if unit has `Armored` attribute.
	pub fn is_armored(&self) -> bool {
		self.has_attribute(Attribute::Armored)
	}
	/// Checks if unit has `Biological` attribute.
	pub fn is_biological(&self) -> bool {
		self.has_attribute(Attribute::Biological)
	}
	/// Checks if unit has `Mechanical` attribute.
	pub fn is_mechanical(&self) -> bool {
		self.has_attribute(Attribute::Mechanical)
	}
	/// Checks if unit has `Robotic` attribute.
	pub fn is_robotic(&self) -> bool {
		self.has_attribute(Attribute::Robotic)
	}
	/// Checks if unit has `Psionic` attribute.
	pub fn is_psionic(&self) -> bool {
		self.has_attribute(Attribute::Psionic)
	}
	/// Checks if unit has `Massive` attribute.
	pub fn is_massive(&self) -> bool {
		self.has_attribute(Attribute::Massive)
	}
	/// Checks if unit has `Structure` attribute.
	pub fn is_structure(&self) -> bool {
		self.has_attribute(Attribute::Structure)
	}
	/// Checks if unit has `Hover` attribute.
	pub fn is_hover(&self) -> bool {
		self.has_attribute(Attribute::Hover)
	}
	/// Checks if unit has `Heroic` attribute.
	pub fn is_heroic(&self) -> bool {
		self.has_attribute(Attribute::Heroic)
	}
	/// Checks if unit has `Summoned` attribute.
	pub fn is_summoned(&self) -> bool {
		self.has_attribute(Attribute::Summoned)
	}
	/// Checks if unit has given buff.
	pub fn has_buff(&self, buff: BuffId) -> bool {
		self.buffs().contains(&buff)
	}
	/// Checks if unit has any from given buffs.
	pub fn has_any_buff<'a, B: IntoIterator<Item = &'a BuffId>>(&self, buffs: B) -> bool {
		buffs.into_iter().any(|b| self.buffs().contains(b))
	}
	/// Checks if worker is carrying minerals.
	pub fn is_carrying_minerals(&self) -> bool {
		self.has_any_buff(&[
			BuffId::CarryMineralFieldMinerals,
			BuffId::CarryHighYieldMineralFieldMinerals,
		])
	}
	/// Checks if worker is carrying vespene gas
	/// (Currently not works if worker is carrying gas from rich vespene geyeser,
	/// because SC2 API is not providing this information).
	pub fn is_carrying_vespene(&self) -> bool {
		self.has_any_buff(&[
			BuffId::CarryHarvestableVespeneGeyserGas,
			BuffId::CarryHarvestableVespeneGeyserGasProtoss,
			BuffId::CarryHarvestableVespeneGeyserGasZerg,
		])
	}
	/// Checks if worker is carrying any resource
	/// (Currently not works if worker is carrying gas from rich vespene geyeser,
	/// because SC2 API is not providing this information)
	pub fn is_carrying_resource(&self) -> bool {
		self.is_carrying_minerals() || self.is_carrying_vespene()
	}

	#[inline]
	pub fn weapons(&self) -> &[Weapon] {
		match self.type_id() {
			UnitTypeId::Changeling
			| UnitTypeId::ChangelingZealot
			| UnitTypeId::ChangelingMarineShield
			| UnitTypeId::ChangelingMarine
			| UnitTypeId::ChangelingZerglingWings
			| UnitTypeId::ChangelingZergling => &[],
			UnitTypeId::Baneling | UnitTypeId::BanelingBurrowed | UnitTypeId::BanelingCocoon => {
				&MISSED_WEAPONS[&UnitTypeId::Baneling]
			}
			UnitTypeId::RavagerCocoon => self
				.data
				.game_data
				.units
				.get(&UnitTypeId::Ravager)
				.map_or(&[], |data| data.weapons.as_slice()),
			unit_type => self
				.type_data()
				.map(|data| data.weapons.as_slice())
				.filter(|weapons| !weapons.is_empty())
				.or_else(|| MISSED_WEAPONS.get(&unit_type).map(|ws| ws.as_slice()))
				.unwrap_or_default(),
		}
	}
	/// Targets unit can attack if it has weapon.
	pub fn weapon_target(&self) -> Option<TargetType> {
		let weapons = self.weapons();
		if weapons.is_empty() {
			return None;
		}

		let mut ground = false;
		let mut air = false;
		if weapons.iter().any(|w| match w.target {
			TargetType::Ground => {
				ground = true;
				ground && air
			}
			TargetType::Air => {
				air = true;
				ground && air
			}
			_ => true,
		}) || (ground && air)
		{
			Some(TargetType::Any)
		} else if ground {
			Some(TargetType::Ground)
		} else if air {
			Some(TargetType::Air)
		} else {
			None
		}
	}
	/// Checks if unit can attack at all (i.e. has weapons).
	pub fn can_attack(&self) -> bool {
		!self.weapons().is_empty()
	}
	/// Checks if unit can attack both air and ground targets.
	pub fn can_attack_both(&self) -> bool {
		let weapons = self.weapons();
		if weapons.is_empty() {
			return false;
		}

		let mut ground = false;
		let mut air = false;
		weapons.iter().any(|w| match w.target {
			TargetType::Ground => {
				ground = true;
				ground && air
			}
			TargetType::Air => {
				air = true;
				ground && air
			}
			_ => true,
		}) || (ground && air)
	}
	/// Checks if unit can attack ground targets.
	pub fn can_attack_ground(&self) -> bool {
		self.weapons().iter().any(|w| !w.target.is_air())
	}
	/// Checks if unit can attack air targets.
	pub fn can_attack_air(&self) -> bool {
		self.weapons().iter().any(|w| !w.target.is_ground())
	}
	/// Checks if unit can attack given target.
	pub fn can_attack_unit(&self, target: &Unit) -> bool {
		let weapons = self.weapons();
		if weapons.is_empty() {
			return false;
		}

		if target.type_id() == UnitTypeId::Colossus {
			!weapons.is_empty()
		} else {
			let not_target = {
				if target.is_flying() {
					TargetType::Ground
				} else {
					TargetType::Air
				}
			};
			weapons.iter().any(|w| w.target != not_target)
		}
	}
	/// Checks if unit's weapon is on cooldown.
	pub fn on_cooldown(&self) -> bool {
		self.weapon_cooldown().map_or(false, |cool| cool > f32::EPSILON)
	}
	/// Returns max cooldown in frames for unit's weapon.
	pub fn max_cooldown(&self) -> Option<f32> {
		self.data.max_cooldowns.read_lock().get(&self.type_id()).copied()
	}
	/// Returns weapon cooldown percentage (current cooldown divided by max cooldown).
	/// Value in range from `0` to `1`.
	pub fn cooldown_percentage(&self) -> Option<f32> {
		let current = self.weapon_cooldown()?;
		let max = self.max_cooldown()?;
		if max == 0.0 {
			return None;
		}
		Some(current / max)
	}
	/// Returns ground range of unit's weapon without considering upgrades.
	/// Use [`real_ground_range`](Self::real_ground_range) to get range including upgrades.
	pub fn ground_range(&self) -> f32 {
		self.weapons()
			.iter()
			.find(|w| !w.target.is_air())
			.map_or(0.0, |w| w.range)
	}
	/// Returns air range of unit's weapon without considering upgrades.
	/// Use [`real_air_range`](Self::real_air_range) to get range including upgrades.
	pub fn air_range(&self) -> f32 {
		self.weapons()
			.iter()
			.find(|w| !w.target.is_ground())
			.map_or(0.0, |w| w.range)
	}
	/// Returns range of unit's weapon vs given target if unit can it, otherwise returns `0`.
	/// Doesn't consider upgrades, use [`real_range_vs`](Self::real_range_vs)
	/// instead to get range including upgrades.
	pub fn range_vs(&self, target: &Unit) -> f32 {
		let weapons = self.weapons();
		if weapons.is_empty() {
			return 0.0;
		}

		if target.type_id() == UnitTypeId::Colossus {
			weapons
				.iter()
				.map(|w| w.range)
				.max_by(|r1, r2| r1.partial_cmp(r2).unwrap_or(Ordering::Equal))
				.unwrap_or(0.0)
		} else {
			let not_target = {
				if target.is_flying() {
					TargetType::Ground
				} else {
					TargetType::Air
				}
			};
			weapons
				.iter()
				.find(|w| w.target != not_target)
				.map_or(0.0, |w| w.range)
		}
	}
	/// Returns actual ground range of unit's weapon including upgrades.
	pub fn real_ground_range(&self) -> f32 {
		self.weapons()
			.iter()
			.find(|w| !w.target.is_air())
			.map_or(0.0, |w| {
				let upgrades = self.upgrades();
				match self.type_id() {
					UnitTypeId::Hydralisk => {
						if upgrades.contains(&UpgradeId::EvolveGroovedSpines) {
							return w.range + 1.0;
						}
					}
					UnitTypeId::Phoenix => {
						if upgrades.contains(&UpgradeId::PhoenixRangeUpgrade) {
							return w.range + 2.0;
						}
					}
					UnitTypeId::PlanetaryFortress | UnitTypeId::MissileTurret | UnitTypeId::AutoTurret => {
						if upgrades.contains(&UpgradeId::HiSecAutoTracking) {
							return w.range + 1.0;
						}
					}
					_ => {}
				}
				w.range
			})
	}
	/// Returns actual air range of unit's weapon including upgrades.
	pub fn real_air_range(&self) -> f32 {
		self.weapons()
			.iter()
			.find(|w| !w.target.is_ground())
			.map_or(0.0, |w| {
				let upgrades = self.upgrades();
				match self.type_id() {
					UnitTypeId::Hydralisk => {
						if upgrades.contains(&UpgradeId::EvolveGroovedSpines) {
							return w.range + 1.0;
						}
					}
					UnitTypeId::Phoenix => {
						if upgrades.contains(&UpgradeId::PhoenixRangeUpgrade) {
							return w.range + 2.0;
						}
					}
					UnitTypeId::PlanetaryFortress | UnitTypeId::MissileTurret | UnitTypeId::AutoTurret => {
						if upgrades.contains(&UpgradeId::HiSecAutoTracking) {
							return w.range + 1.0;
						}
					}
					_ => {}
				}
				w.range
			})
	}
	/// Returns actual range of unit's weapon vs given target if unit can attack it, otherwise returs `0`.
	/// Takes upgrades into account.
	pub fn real_range_vs(&self, target: &Unit) -> f32 {
		let weapons = self.weapons();
		if weapons.is_empty() {
			return 0.0;
		}

		let extract_range = |w: &Weapon| {
			let upgrades = self.upgrades();
			match self.type_id() {
				UnitTypeId::Hydralisk => {
					if upgrades.contains(&UpgradeId::EvolveGroovedSpines) {
						return w.range + 1.0;
					}
				}
				UnitTypeId::Phoenix => {
					if upgrades.contains(&UpgradeId::PhoenixRangeUpgrade) {
						return w.range + 2f32;
					}
				}
				UnitTypeId::PlanetaryFortress | UnitTypeId::MissileTurret | UnitTypeId::AutoTurret => {
					if upgrades.contains(&UpgradeId::HiSecAutoTracking) {
						return w.range + 1f32;
					}
				}
				UnitTypeId::Colossus => {
					if upgrades.contains(&UpgradeId::ExtendedThermalLance) {
						return w.range + 2f32;
					}
				}
				UnitTypeId::Ghost => {
					// TODO: Is it possible to get energy cost from Ability data?
					let ability = self
						.data
						.game_data
						.abilities
						.get(&AbilityId::EffectGhostSnipe)
						.unwrap();
					return if self.has_buff(BuffId::ChannelSnipeCombat) {
						ability.cast_range.unwrap_or_default() + 4f32
					} else if self.energy().unwrap_or_default() >= 50 {
						ability.cast_range.unwrap_or_default()
					} else {
						w.range
					};
				}
				_ => {}
			}
			w.range
		};

		if target.type_id() == UnitTypeId::Colossus {
			weapons
				.iter()
				.map(extract_range)
				.max_by(|r1, r2| r1.partial_cmp(r2).unwrap_or(Ordering::Equal))
				.unwrap_or(0.0)
		} else {
			let not_target = {
				if target.is_flying() {
					TargetType::Ground
				} else {
					TargetType::Air
				}
			};
			weapons
				.iter()
				.find(|w| w.target != not_target)
				.map_or(0.0, extract_range)
		}
	}
	/// Returns ground dps of unit's weapon without considering upgrades.
	/// Use [`real_ground_weapon`](Self::real_ground_weapon) to get dps including upgrades.
	pub fn ground_dps(&self) -> f32 {
		self.weapons()
			.iter()
			.find(|w| !w.target.is_air())
			.map_or(0.0, |w| w.damage as f32 * (w.attacks as f32) / w.speed)
	}
	/// Returns air dps of unit's weapon without considering upgrades.
	/// Use [`real_air_weapon`](Self::real_air_weapon) to get dps including upgrades.
	pub fn air_dps(&self) -> f32 {
		self.weapons()
			.iter()
			.find(|w| !w.target.is_ground())
			.map_or(0.0, |w| w.damage as f32 * (w.attacks as f32) / w.speed)
	}
	/// Returns dps of unit's weapon vs given target if unit can it, otherwise returns `0`.
	/// Doesn't consider upgrades, use [`real_weapon_vs`](Self::real_weapon_vs)
	/// instead to get dps including upgrades.
	pub fn dps_vs(&self, target: &Unit) -> f32 {
		let weapons = self.weapons();
		if weapons.is_empty() {
			return 0.0;
		}

		let extract_dps = |w: &Weapon| w.damage as f32 * (w.attacks as f32) / w.speed;

		if target.type_id() == UnitTypeId::Colossus {
			weapons
				.iter()
				.map(extract_dps)
				.max_by(|d1, d2| d1.partial_cmp(d2).unwrap_or(Ordering::Equal))
				.unwrap_or(0.0)
		} else {
			let not_target = {
				if target.is_flying() {
					TargetType::Ground
				} else {
					TargetType::Air
				}
			};
			weapons
				.iter()
				.find(|w| w.target != not_target)
				.map_or(0.0, extract_dps)
		}
	}

	/// Returns (dps, range) of first unit's weapon including bonuses from buffs and upgrades.
	///
	/// If you need to get only real range of unit, use [`real_ground_range`], [`real_air_range`]
	/// or [`real_range_vs`] instead, because they're generally faster.
	///
	/// [`real_range_vs`]: Self::real_range_vs
	/// [`real_ground_range`]: Self::real_ground_range
	/// [`real_air_range`]: Self::real_air_range
	pub fn real_weapon(&self, attributes: &[Attribute]) -> WeaponStats {
		self.calculate_weapon_stats(CalcTarget::Abstract(TargetType::Any, attributes))
	}
	/// Returns (dps, range) of unit's ground weapon including bonuses from buffs and upgrades.
	///
	/// If you need to get only real range of unit, use [`real_ground_range`](Self::real_ground_range)
	/// instead, because it's generally faster.
	pub fn real_ground_weapon(&self, attributes: &[Attribute]) -> WeaponStats {
		self.calculate_weapon_stats(CalcTarget::Abstract(TargetType::Ground, attributes))
	}
	/// Returns (dps, range) of unit's air weapon including bonuses from buffs and upgrades.
	///
	/// If you need to get only real range of unit, use [`real_air_range`](Self::real_air_range)
	/// instead, because it's generally faster.
	pub fn real_air_weapon(&self, attributes: &[Attribute]) -> WeaponStats {
		self.calculate_weapon_stats(CalcTarget::Abstract(TargetType::Air, attributes))
	}
	/// Returns (dps, range) of unit's weapon vs given target if unit can attack it, otherwise returs `(0, 0)`.
	/// Takes buffs and upgrades into account.
	///
	/// If you need to get only real range of unit, use [`real_range_vs`](Self::real_range_vs)
	/// instead, because it's generally faster.
	pub fn real_weapon_vs(&self, target: &Unit) -> WeaponStats {
		self.base.real_weapon_vs.get_or_create(&target.tag(), || {
			self.calculate_weapon_stats(CalcTarget::Unit(target))
		})
	}

	/// Returns (dps, range) of unit's weapon vs given abstract target
	/// if unit can attack it, otherwise returs `(0, 0)`.
	/// Abstract target is described by it's type (air or ground) and attributes (e.g. light, armored, ...).
	///
	/// If you need to get only real range of unit, use [`real_ground_range`], [`real_air_range`]
	/// or [`real_range_vs`] instead, because they're generally faster.
	///
	/// [`real_range_vs`]: Self::real_range_vs
	/// [`real_ground_range`]: Self::real_ground_range
	/// [`real_air_range`]: Self::real_air_range
	pub fn calculate_weapon_abstract(
		&self,
		target_type: TargetType,
		attributes: &[Attribute],
	) -> WeaponStats {
		self.calculate_weapon_stats(CalcTarget::Abstract(target_type, attributes))
	}

	/// Returns (dps, range) of unit's weapon vs given target (can be unit or abstract)
	/// if unit can attack it, otherwise returs `(0, 0)`.
	///
	/// If you need to get only real range of unit, use [`real_ground_range`], [`real_air_range`]
	/// or [`real_range_vs`] instead, because they're generally faster.
	///
	/// [`real_range_vs`]: Self::real_range_vs
	/// [`real_ground_range`]: Self::real_ground_range
	/// [`real_air_range`]: Self::real_air_range
	#[allow(clippy::mut_range_bound)]
	pub fn calculate_weapon_stats(&self, target: CalcTarget) -> WeaponStats {
		let (upgrades, target_upgrades) = {
			let my_upgrades = self.data.upgrades.read_lock();
			let enemy_upgrades = self.data.enemy_upgrades.read_lock();
			if self.is_mine() {
				(my_upgrades, enemy_upgrades)
			} else {
				(enemy_upgrades, my_upgrades)
			}
		};
		if matches!(self.type_id(), UnitTypeId::Oracle) && !self.has_buff(BuffId::OracleWeapon) {
			return WeaponStats {
				damage: 0,
				speed: 0f32,
				range: 0f32,
			};
		}

		let (not_target, attributes, target_unit) = match target {
			CalcTarget::Unit(target) => {
				let mut enemy_armor = target.armor() + target.armor_upgrade_level();
				let mut enemy_shield_armor = target.shield_upgrade_level();

				let mut target_has_guardian_shield = false;

				for buff in target.buffs() {
					match buff {
						BuffId::GuardianShield => target_has_guardian_shield = true,
						_ => {
							if *buff == ANTI_ARMOR_BUFF {
								enemy_armor -= 3;
								enemy_shield_armor -= 3;
							}
						}
					}
				}

				if !target_upgrades.is_empty() {
					if target.race().is_terran() {
						if target.is_structure() && target_upgrades.contains(&UpgradeId::TerranBuildingArmor)
						{
							enemy_armor += 2;
						}
					} else if matches!(
						target.type_id(),
						UnitTypeId::Ultralisk | UnitTypeId::UltraliskBurrowed
					) && target_upgrades.contains(&UpgradeId::ChitinousPlating)
					{
						enemy_armor += 2;
					}
				}

				(
					if matches!(target.type_id(), UnitTypeId::Colossus) {
						TargetType::Any
					} else if target.is_flying() {
						TargetType::Ground
					} else {
						TargetType::Air
					},
					target.attributes(),
					Some((
						target,
						enemy_armor,
						enemy_shield_armor,
						target_has_guardian_shield,
					)),
				)
			}
			CalcTarget::Abstract(target_type, attributes) => (
				match target_type {
					TargetType::Any => TargetType::Any,
					TargetType::Ground => TargetType::Air,
					TargetType::Air => TargetType::Ground,
				},
				attributes,
				None,
			),
		};

		let weapons = self.weapons();
		if weapons.is_empty() {
			return WeaponStats {
				damage: 0,
				speed: 0f32,
				range: 0f32,
			};
		}

		let mut speed_modifier = 1.0;
		let mut range_modifier = 0.0;

		for buff in self.buffs() {
			match buff {
				BuffId::Stimpack | BuffId::StimpackMarauder => speed_modifier /= 1.5,
				BuffId::TimeWarpProduction => speed_modifier *= 2.0,
				_ => {}
			}
		}

		if !upgrades.is_empty() {
			match self.type_id() {
				UnitTypeId::Zergling => {
					if upgrades.contains(&UpgradeId::Zerglingattackspeed) {
						speed_modifier /= 1.4;
					}
				}
				UnitTypeId::Adept => {
					if upgrades.contains(&UpgradeId::AdeptPiercingAttack) {
						speed_modifier /= 1.45;
					}
				}
				UnitTypeId::Hydralisk => {
					if upgrades.contains(&UpgradeId::EvolveGroovedSpines) {
						range_modifier += 1.0;
					}
				}
				UnitTypeId::Phoenix => {
					if upgrades.contains(&UpgradeId::PhoenixRangeUpgrade) {
						range_modifier += 2.0;
					}
				}
				UnitTypeId::LurkerMPBurrowed => {
					if upgrades.contains(&UpgradeId::LurkerRange) {
						range_modifier += 2.0;
					}
				}
				UnitTypeId::PlanetaryFortress | UnitTypeId::MissileTurret | UnitTypeId::AutoTurret => {
					if upgrades.contains(&UpgradeId::HiSecAutoTracking) {
						range_modifier += 1.0;
					}
				}
				_ => {}
			}
		}

		let damage_bonus_per_upgrade = DAMAGE_BONUS_PER_UPGRADE.get(&self.type_id());
		let extract_weapon_stats = |w: &Weapon| {
			let damage_bonus_per_upgrade = damage_bonus_per_upgrade.and_then(|bonus| bonus.get(&w.target));

			let mut damage = w.damage
				+ (self.attack_upgrade_level()
					* damage_bonus_per_upgrade.and_then(|bonus| bonus.0).unwrap_or(1));
			let speed = w.speed * speed_modifier;
			let range = w.range + range_modifier;

			// Bonus damage
			if let Some(bonus) = w
				.damage_bonus
				.iter()
				.filter_map(|(attribute, bonus)| {
					if attributes.contains(attribute) {
						let mut damage_bonus_per_upgrade = damage_bonus_per_upgrade
							.and_then(|bonus| bonus.1.get(attribute))
							.copied()
							.unwrap_or(0);

						if let Attribute::Light = attribute {
							if upgrades.contains(&UpgradeId::HighCapacityBarrels) {
								match self.type_id() {
									UnitTypeId::Hellion => damage_bonus_per_upgrade += 5,
									UnitTypeId::HellionTank => damage_bonus_per_upgrade += 12,
									_ => {}
								}
							}
						}

						let mut bonus_damage =
							bonus + (self.attack_upgrade_level() * damage_bonus_per_upgrade);

						if let Attribute::Armored = attribute {
							if self.has_buff(BuffId::VoidRaySwarmDamageBoost) {
								bonus_damage += 6;
							}
						}

						Some(bonus_damage)
					} else {
						None
					}
				})
				.max_by(|b1, b2| b1.partial_cmp(b2).unwrap_or(Ordering::Equal))
			{
				damage += bonus;
			}

			// Subtract damage
			match target_unit {
				Some((target, enemy_armor, enemy_shield_armor, target_has_guardian_shield)) => {
					let mut attacks = w.attacks;
					let mut shield_damage = 0;
					let mut health_damage = 0;

					if let Some(enemy_shield) = target.shield().filter(|shield| shield > &0) {
						let enemy_shield_armor = if target_has_guardian_shield && range >= 2.0 {
							enemy_shield_armor + 2
						} else {
							enemy_shield_armor
						};
						let exact_damage = 1.max(damage as i32 - enemy_shield_armor) as u32;

						for _ in 0..attacks {
							if shield_damage >= enemy_shield {
								health_damage = shield_damage - enemy_shield;
								break;
							}
							shield_damage += exact_damage;
							attacks -= 1;
						}
					}

					if let Some(enemy_health) = target.health().filter(|health| health > &0) {
						let enemy_armor = if target_has_guardian_shield && range >= 2.0 {
							enemy_armor + 2
						} else {
							enemy_armor
						};
						let exact_damage = 1.max(damage as i32 - enemy_armor) as u32;

						for _ in 0..attacks {
							if health_damage >= enemy_health {
								break;
							}
							health_damage += exact_damage;
						}
					}

					(shield_damage + health_damage, speed, range)
				}
				None => (damage * w.attacks, speed, range),
			}
		};
		let (damage, speed, range) = if not_target.is_any() {
			weapons
				.iter()
				.map(extract_weapon_stats)
				.max_by_key(|k| k.0)
				.unwrap_or((0, 0.0, 0.0))
		} else {
			weapons
				.iter()
				.filter(|w| w.target != not_target)
				.map(extract_weapon_stats)
				.max_by_key(|k| k.0)
				.unwrap_or((0, 0.0, 0.0))
		};
		WeaponStats {
			damage: if speed == 0f32 { 0 } else { damage },
			speed,
			range,
		}
	}

	/// Checks if unit is close enough to attack given target.
	///
	/// See also [`in_real_range`](Self::in_real_range) which uses actual range of unit for calculations.
	pub fn in_range(&self, target: &Unit, gap: f32) -> bool {
		let range = {
			if matches!(target.type_id(), UnitTypeId::Colossus) {
				match self
					.weapons()
					.iter()
					.map(|w| w.range)
					.max_by(|r1, r2| r1.partial_cmp(r2).unwrap_or(Ordering::Equal))
				{
					Some(max_range) => max_range,
					None => return false,
				}
			} else {
				let range = if target.is_flying() {
					self.air_range()
				} else {
					self.ground_range()
				};
				if range < f32::EPSILON {
					return false;
				}
				range
			}
		};
		let distance = self.distance_squared(target);

		// Takes into account that Sieged Tank has a minimum range of 2
		(self.type_id() != UnitTypeId::SiegeTankSieged || distance > 4.0)
			&& distance <= (self.radius() + target.radius() + range + gap).powi(2)
	}
	/// Checks if unit is close enough to be attacked by given threat.
	/// This `unit.in_range_of(threat, gap)` is equivalent to `threat.in_range(unit, gap)`.
	///
	/// See also [`in_real_range_of`](Self::in_real_range_of) which uses actual range of unit for calculation.
	pub fn in_range_of(&self, threat: &Unit, gap: f32) -> bool {
		threat.in_range(self, gap)
	}
	/// Checks if unit is close enough to attack given target.
	///
	/// Uses actual range from [`real_range_vs`](Self::real_range_vs) in it's calculations.
	pub fn in_real_range(&self, target: &Unit, gap: f32) -> bool {
		let range = self.real_range_vs(target);
		if range < f32::EPSILON {
			return false;
		}
		let distance = self.distance_squared(target);

		// Takes into account that Sieged Tank has a minimum range of 2
		(self.type_id() != UnitTypeId::SiegeTankSieged || distance > 4.0)
			&& distance <= (self.radius() + target.radius() + range + gap).powi(2)
	}
	/// Checks if unit is close enough to be attacked by given threat.
	/// This `unit.in_real_range_of(threat, gap)` is equivalent to `threat.in_real_range(unit, gap)`.
	///
	/// Uses actual range from [`real_range_vs`](Self::real_range_vs) in it's calculations.
	pub fn in_real_range_of(&self, threat: &Unit, gap: f32) -> bool {
		threat.in_real_range(self, gap)
	}
	/// Checks if unit is close enough to use given ability on target.
	pub fn in_ability_cast_range<A>(&self, ability_id: AbilityId, target: A, gap: f32) -> bool
	where
		A: Into<Point2> + Radius,
	{
		if let Some(data) = self.data.game_data.abilities.get(&ability_id) {
			if let Some(cast_range) = data.cast_range {
				return (cast_range + self.radius() + target.radius() + gap).powi(2)
					>= self.distance_squared(target);
			}
		}
		false
	}
	/// Returns (attribute, bonus damage) for first unit's weapon if any.
	pub fn damage_bonus(&self) -> Option<(Attribute, u32)> {
		self.weapons()
			.iter()
			.find_map(|w| w.damage_bonus.first())
			.copied()
	}
	/// Returns (ability, target, progress) of the current unit order or `None` if it's idle.
	pub fn order(&self) -> Option<(AbilityId, Target, f32)> {
		self.orders()
			.first()
			.map(|order| (order.ability, order.target, order.progress))
	}
	/// Returns target of first unit's order.
	pub fn target(&self) -> Target {
		self.orders().first().map_or(Target::None, |order| order.target)
	}
	/// Returns target point of unit's order if any.
	pub fn target_pos(&self) -> Option<Point2> {
		match self.target() {
			Target::Pos(pos) => Some(pos),
			_ => None,
		}
	}
	/// Returns target tag of unit's order if any.
	pub fn target_tag(&self) -> Option<u64> {
		match self.target() {
			Target::Tag(tag) => Some(tag),
			_ => None,
		}
	}
	/// Returns ability of first unit's order.
	pub fn ordered_ability(&self) -> Option<AbilityId> {
		self.orders().first().map(|order| order.ability)
	}
	/// Checks if unit don't have any orders currently.
	pub fn is_idle(&self) -> bool {
		self.orders().is_empty()
	}
	/// Checks if unit don't have any orders currently or it's order is more than 95% complete.
	pub fn is_almost_idle(&self) -> bool {
		self.is_idle() || (self.orders().len() == 1 && self.orders()[0].progress >= 0.95)
	}
	/// Checks if production building with reactor don't have any orders currently.
	pub fn is_unused(&self) -> bool {
		if self.has_reactor() {
			self.orders().len() < 2
		} else {
			self.is_idle()
		}
	}
	/// Checks if production building with reactor don't have any orders currently
	/// or it's order is more than 95% complete.
	pub fn is_almost_unused(&self) -> bool {
		if self.has_reactor() {
			self.orders().len() < 2
				|| (self.orders().len() == 2 && self.orders().iter().any(|order| order.progress >= 0.95))
		} else {
			self.is_almost_idle()
		}
	}
	/// Checks if unit is using given ability.
	///
	/// Doesn't work with enemies.
	pub fn is_using(&self, ability: AbilityId) -> bool {
		self.ordered_ability() == Some(ability)
	}
	/// Checks if unit is using any of given abilities.
	///
	/// Doesn't work with enemies.
	pub fn is_using_any<A: Container<AbilityId>>(&self, abilities: &A) -> bool {
		self.ordered_ability().map_or(false, |a| abilities.contains(&a))
	}
	/// Checks if unit is currently attacking.
	///
	/// Doesn't work with enemies.
	#[rustfmt::skip::macros(matches)]
	pub fn is_attacking(&self) -> bool {
		self.is_using_any(&vec![
			AbilityId::Attack,
			AbilityId::AttackAttack,
			AbilityId::AttackAttackTowards,
			AbilityId::AttackAttackBarrage,
			AbilityId::ScanMove,
		])
	}
	/// Checks if unit is currently moving.
	///
	/// Doesn't work with enemies.
	pub fn is_moving(&self) -> bool {
		self.is_using(AbilityId::MoveMove)
	}
	/// Checks if unit is currently patrolling.
	///
	/// Doesn't work with enemies.
	pub fn is_patrolling(&self) -> bool {
		self.is_using(AbilityId::Patrol)
	}
	/// Checks if SCV or MULE is currently repairing.
	///
	/// Doesn't work with enemies.
	pub fn is_repairing(&self) -> bool {
		self.is_using_any(&vec![AbilityId::EffectRepairSCV, AbilityId::EffectRepairMule])
	}
	/// Checks if worker is currently gathering resource.
	///
	/// Doesn't work with enemies.
	pub fn is_gathering(&self) -> bool {
		self.is_using_any(&vec![
			AbilityId::HarvestGatherSCV,
			AbilityId::HarvestGatherMule,
			AbilityId::HarvestGatherDrone,
			AbilityId::HarvestGatherProbe,
		])
	}
	/// Checks if worker is currently returning resource closest base.
	///
	/// Doesn't work with enemies.
	pub fn is_returning(&self) -> bool {
		self.is_using_any(&vec![
			AbilityId::HarvestReturnSCV,
			AbilityId::HarvestReturnMule,
			AbilityId::HarvestReturnDrone,
			AbilityId::HarvestReturnProbe,
		])
	}
	/// Checks if worker is currently gathering or returning resources.
	///
	/// Doesn't work with enemies.
	pub fn is_collecting(&self) -> bool {
		self.orders().first().map_or(false, |order| match self.type_id() {
			UnitTypeId::SCV => matches!(
				order.ability,
				AbilityId::HarvestGatherSCV | AbilityId::HarvestReturnSCV
			),
			UnitTypeId::MULE => matches!(
				order.ability,
				AbilityId::HarvestGatherMule | AbilityId::HarvestReturnMule
			),
			UnitTypeId::Drone => matches!(
				order.ability,
				AbilityId::HarvestGatherDrone | AbilityId::HarvestReturnDrone
			),
			UnitTypeId::Probe => matches!(
				order.ability,
				AbilityId::HarvestGatherProbe | AbilityId::HarvestReturnProbe
			),
			_ => false,
		})
	}
	/// Checks if worker is currently constructing a building.
	///
	/// Doesn't work with enemies.
	pub fn is_constructing(&self) -> bool {
		self.orders().first().map_or(false, |order| match self.type_id() {
			UnitTypeId::SCV => order.ability.is_constructing_scv(),
			UnitTypeId::Drone => order.ability.is_constructing_drone(),
			UnitTypeId::Probe => order.ability.is_constructing_probe(),
			_ => false,
		})
	}

	/// Checks if worker is currently constructing a specific building.
	///
	/// Doesn't work with enemies.
	pub fn is_constructing_any(&self, unit_types: &Vec<UnitTypeId>) -> bool {
		unit_types
			.iter()
			.map(|t| self.data.game_data.units.get(t).and_then(|data| data.ability))
			.any(|a| a.is_some() && self.is_using(a.unwrap()))
	}

	/// Checks if terran building is currently making addon.
	///
	/// Doesn't work with enemies.
	pub fn is_making_addon(&self) -> bool {
		self.orders().first().map_or(false, |order| match self.type_id() {
			UnitTypeId::Barracks => matches!(
				order.ability,
				AbilityId::BuildTechLabBarracks | AbilityId::BuildReactorBarracks
			),
			UnitTypeId::Factory => matches!(
				order.ability,
				AbilityId::BuildTechLabFactory | AbilityId::BuildReactorFactory
			),
			UnitTypeId::Starport => matches!(
				order.ability,
				AbilityId::BuildTechLabStarport | AbilityId::BuildReactorStarport
			),
			_ => false,
		})
	}
	/// Checks if terran building is currently building techlab.
	///
	/// Doesn't work with enemies.
	pub fn is_making_techlab(&self) -> bool {
		self.is_using_any(&vec![
			AbilityId::BuildTechLabBarracks,
			AbilityId::BuildTechLabFactory,
			AbilityId::BuildTechLabStarport,
		])
	}
	/// Checks if terran building is currently building reactor.
	///
	/// Doesn't work with enemies.
	pub fn is_making_reactor(&self) -> bool {
		self.is_using_any(&vec![
			AbilityId::BuildReactorBarracks,
			AbilityId::BuildReactorFactory,
			AbilityId::BuildReactorStarport,
		])
	}

	/// Checks if unit is doing something important
	/// and it's bad idea to interrupt it,
	/// so you can skip evaluating it anyway.
	///
	/// Use with [`sleep`](Self::sleep)
	/// to skip evaluating units executing durable commands.
	pub fn is_sleeping(&self) -> bool {
		self.data
			.available_frames
			.read_lock()
			.get(&self.tag())
			.map_or(false, |frame| self.data.game_loop.get_locked() < *frame)
	}
	/// Makes unit ignore all your commands for given amount of frames.
	///
	/// Use with [`is_sleeping`](Self::is_sleeping)
	/// to skip evaluating units executing durable commands.
	pub fn sleep(&self, duration: u32) {
		self.data
			.available_frames
			.write_lock()
			.insert(self.tag(), self.data.game_loop.get_locked() + duration);
	}

	// Actions

	/// Toggles autocast on given ability.
	pub fn toggle_autocast(&self, ability: AbilityId) {
		self.data
			.commander
			.write_lock()
			.autocast
			.entry(ability)
			.or_default()
			.push(self.tag());
	}
	/// Orders unit to execute given command.
	pub fn command(&self, ability: AbilityId, target: Target, queue: bool) {
		self.data
			.commander
			.write_lock()
			.commands
			.entry((ability, target, queue))
			.or_default()
			.push(self.tag());
	}
	/// Orders unit to use given ability (This is equivalent of `unit.command(ability, Target::None, queue)`).
	pub fn use_ability(&self, ability: AbilityId, queue: bool) {
		self.command(ability, Target::None, queue)
	}
	/// Orders unit a `Smart` ability (This is equivalent of right click).
	pub fn smart(&self, target: Target, queue: bool) {
		self.command(AbilityId::Smart, target, queue)
	}
	/// Orders unit to attack given target.
	pub fn attack(&self, target: Target, queue: bool) {
		self.command(AbilityId::Attack, target, queue)
	}
	/// Orders unit to move to given target.
	pub fn move_to(&self, target: Target, queue: bool) {
		self.command(AbilityId::MoveMove, target, queue)
	}
	/// Orders unit to hold position.
	pub fn hold_position(&self, queue: bool) {
		self.command(AbilityId::HoldPosition, Target::None, queue)
	}
	/// Orders worker to gather given resource.
	pub fn gather(&self, target: u64, queue: bool) {
		self.command(AbilityId::HarvestGather, Target::Tag(target), queue)
	}
	/// Orders worker to return resource to closest base.
	pub fn return_resource(&self, queue: bool) {
		self.command(AbilityId::HarvestReturn, Target::None, queue)
	}
	/// Orders unit to stop actions.
	pub fn stop(&self, queue: bool) {
		self.command(AbilityId::Stop, Target::None, queue)
	}
	/// Orders unit to patrol.
	pub fn patrol(&self, target: Target, queue: bool) {
		self.command(AbilityId::Patrol, target, queue)
	}
	/// Orders SCV or MULE to repair given structure or mechanical unit.
	pub fn repair(&self, target: u64, queue: bool) {
		self.command(AbilityId::EffectRepair, Target::Tag(target), queue)
	}
	/// Orders building which is in progress to cancel construction.
	pub fn cancel_building(&self, queue: bool) {
		self.command(AbilityId::CancelBuildInProgress, Target::None, queue)
	}
	/// Orders production building to cancel last unit in train queue.
	pub fn cancel_queue(&self, queue: bool) {
		self.command(
			if self.is_townhall() {
				AbilityId::CancelQueueCancelToSelection
			} else {
				AbilityId::CancelQueue5
			},
			Target::None,
			queue,
		)
	}
	/// Orders worker to build race gas building on given geyser.
	pub fn build_gas(&self, target: u64, queue: bool) {
		self.command(
			self.data.game_data.units[&self.data.race_values.gas]
				.ability
				.unwrap(),
			Target::Tag(target),
			queue,
		)
	}
	/// Orders worker to build something on given position.
	pub fn build(&self, unit: UnitTypeId, target: Point2, queue: bool) {
		if let Some(type_data) = self.data.game_data.units.get(&unit) {
			if let Some(ability) = type_data.ability {
				self.command(ability, Target::Pos(target), queue);
			}
		}
	}
	/// Orders production building to train given unit.
	///
	/// This also works for morphing units and building addons.
	pub fn train(&self, unit: UnitTypeId, queue: bool) {
		if let Some(type_data) = self.data.game_data.units.get(&unit) {
			if let Some(ability) = type_data.ability {
				self.command(ability, Target::None, queue);
			}
		}
	}
	/// Orders building to research given upgrade.
	pub fn research(&self, upgrade: UpgradeId, queue: bool) {
		match upgrade {
			UpgradeId::TerranVehicleAndShipArmorsLevel1
			| UpgradeId::TerranVehicleAndShipArmorsLevel2
			| UpgradeId::TerranVehicleAndShipArmorsLevel3 => self.command(
				AbilityId::ResearchTerranVehicleAndShipPlating,
				Target::None,
				queue,
			),
			_ => {
				if let Some(type_data) = self.data.game_data.upgrades.get(&upgrade) {
					self.command(type_data.ability, Target::None, queue);
				}
			}
		}
	}
	/// Orders protoss warp gate to warp unit on given position.
	pub fn warp_in(&self, unit: UnitTypeId, target: Point2) {
		if let Some(ability) = WARPGATE_ABILITIES.get(&unit) {
			self.command(*ability, Target::Pos(target), false);
		}
	}
	/// Orders terran building to lift in the air.
	pub fn lift(&self, queue: bool) {
		self.command(AbilityId::Lift, Target::None, queue);
	}
	/// Orders flying terran building to land on given position.
	pub fn land(&self, target: Point2, queue: bool) {
		self.command(AbilityId::Land, Target::Pos(target), queue);
	}
}

impl From<&Unit> for Point2 {
	#[inline]
	fn from(u: &Unit) -> Self {
		u.position()
	}
}
impl From<Unit> for Point2 {
	#[inline]
	fn from(u: Unit) -> Self {
		u.position()
	}
}

impl Unit {
	pub(crate) fn from_proto(data: SharedUnitData, visibility: &VisibilityMap, u: &ProtoUnit) -> Self {
		let pos = u.get_pos();
		let position = Point2::from_proto(pos);
		let type_id = {
			let id = u.get_unit_type();
			UnitTypeId::from_u32(id).unwrap_or_else(|| panic!("There's no `UnitTypeId` with value {}", id))
		};
		let is_burrowed = u.get_is_burrowed();
		let (is_cloaked, is_revealed) = if is_burrowed {
			(true, false)
		} else {
			match u.get_cloak() {
				ProtoCloakState::CloakedUnknown | ProtoCloakState::NotCloaked => (false, false),
				ProtoCloakState::Cloaked | ProtoCloakState::CloakedAllied => (true, false),
				ProtoCloakState::CloakedDetected => (true, true),
			}
		};
		Self {
			data,
			base: Rs::new(UnitBase {
				display_type: Rl::new(match DisplayType::from_proto(u.get_display_type()) {
					DisplayType::Visible => {
						if visibility
							.get(<(usize, usize)>::from(position))
							.map_or(false, |p| p.is_visible())
						{
							DisplayType::Visible
						} else {
							DisplayType::Snapshot
						}
					}
					x => x,
				}),
				alliance: Alliance::from_proto(u.get_alliance()),
				tag: u.get_tag(),
				type_id: Rl::new(type_id),
				owner: u.get_owner() as u32,
				position,
				position3d: Point3::from_proto(pos),
				facing: u.get_facing(),
				radius: u.get_radius(),
				build_progress: u.get_build_progress(),
				is_cloaked: LockBool::new(is_cloaked),
				is_revealed: LockBool::new(is_revealed),
				buffs: u
					.get_buff_ids()
					.iter()
					.filter(|&b| BuffId::from_u32(*b).is_some())
					.map(|b| {
						BuffId::from_u32(*b).unwrap_or_else(|| panic!("There's no `BuffId` with value {}", b))
					})
					.collect(),
				detect_range: match type_id {
					UnitTypeId::Observer => 11.0,
					UnitTypeId::ObserverSiegeMode => 13.75,
					_ => u.get_detect_range(),
				},
				radar_range: u.get_radar_range(),
				is_selected: u.get_is_selected(),
				is_on_screen: u.get_is_on_screen(),
				is_blip: u.get_is_blip(),
				is_powered: u.get_is_powered(),
				is_active: u.get_is_active(),
				attack_upgrade_level: u.get_attack_upgrade_level() as u32,
				armor_upgrade_level: u.get_armor_upgrade_level(),
				shield_upgrade_level: u.get_shield_upgrade_level(),
				// Not populated for snapshots
				health: u.health.map(|x| x as u32),
				health_max: u.health_max.map(|x| x as u32),
				shield: u.shield.map(|x| x as u32),
				shield_max: u.shield_max.map(|x| x as u32),
				energy: u.energy.map(|x| x as u32),
				energy_max: u.energy_max.map(|x| x as u32),
				mineral_contents: u.mineral_contents.map(|x| x as u32),
				vespene_contents: u.vespene_contents.map(|x| x as u32),
				is_flying: u.get_is_flying(),
				is_burrowed: LockBool::new(is_burrowed),
				is_hallucination: LockBool::new(u.get_is_hallucination()),
				// Not populated for enemies
				orders: u
					.get_orders()
					.iter()
					.filter(|order| AbilityId::from_u32(order.get_ability_id()).is_some())
					.map(|order| UnitOrder {
						ability: {
							let id = order.get_ability_id();
							AbilityId::from_u32(id)
								.unwrap_or_else(|| panic!("There's no `AbilityId` with value {}", id))
						},
						target: match &order.target {
							Some(ProtoTarget::target_world_space_pos(pos)) => {
								Target::Pos(Point2::from_proto(pos))
							}
							Some(ProtoTarget::target_unit_tag(tag)) => Target::Tag(*tag),
							None => Target::None,
						},
						progress: order.get_progress(),
					})
					.collect(),
				addon_tag: u.add_on_tag,
				passengers: u
					.get_passengers()
					.iter()
					.map(|p| PassengerUnit {
						tag: p.get_tag(),
						health: p.get_health(),
						health_max: p.get_health_max(),
						shield: p.get_shield(),
						shield_max: p.get_shield_max(),
						energy: p.get_energy(),
						energy_max: p.get_energy_max(),
						type_id: {
							let id = p.get_unit_type();
							UnitTypeId::from_u32(id)
								.unwrap_or_else(|| panic!("There's no `UnitTypeId` with value {}", id))
						},
					})
					.collect(),
				cargo_space_taken: u.cargo_space_taken.map(|x| x as u32),
				cargo_space_max: u.cargo_space_max.map(|x| x as u32),
				assigned_harvesters: u.assigned_harvesters.map(|x| x as u32),
				ideal_harvesters: u.ideal_harvesters.map(|x| x as u32),
				weapon_cooldown: u.weapon_cooldown,
				engaged_target_tag: u.engaged_target_tag,
				buff_duration_remain: u.buff_duration_remain.map(|x| x as u32),
				buff_duration_max: u.buff_duration_max.map(|x| x as u32),
				rally_targets: u
					.get_rally_targets()
					.iter()
					.map(|t| RallyTarget {
						point: Point2::from_proto(t.get_point()),
						tag: t.tag,
					})
					.collect(),

				// cache
				real_speed: Default::default(),
				on_creep_speed: Default::default(),
				off_creep_speed: Default::default(),
				real_weapon_vs: Default::default(),
			}),
		}
	}
}

/// The display type of [`Unit`].
/// Can be accessed through [`display_type`](Unit::display_type) field.
#[variant_checkers]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DisplayType {
	/// Fully visible.
	Visible,
	/// Dimmed version of unit left behind after entering fog of war.
	Snapshot,
	/// Fully hidden.
	Hidden,
	/// Building that hasn't started construction.
	Placeholder,
}

impl FromProto<ProtoDisplayType> for DisplayType {
	fn from_proto(display_type: ProtoDisplayType) -> Self {
		match display_type {
			ProtoDisplayType::Visible => DisplayType::Visible,
			ProtoDisplayType::Snapshot => DisplayType::Snapshot,
			ProtoDisplayType::Hidden => DisplayType::Hidden,
			ProtoDisplayType::Placeholder => DisplayType::Placeholder,
		}
	}
}

/// Order given to unit. All current orders of unit stored in [`orders`](Unit::orders) field.
#[derive(Clone, Debug)]
pub struct UnitOrder {
	/// Ability unit is using.
	pub ability: AbilityId,
	/// Target of unit's ability.
	pub target: Target,
	/// Progress of train abilities. Value in range from `0` to `1`.
	pub progress: f32,
}

/// Unit inside transport or bunker. All passengers stored in [`passengers`](Unit::passengers) field.
#[derive(Clone, Debug)]
pub struct PassengerUnit {
	pub tag: u64,
	pub health: f32,
	pub health_max: f32,
	pub shield: f32,
	pub shield_max: f32,
	pub energy: f32,
	pub energy_max: f32,
	pub type_id: UnitTypeId,
}

/// Rally point of production building.
/// All rally points stored in [`rally_targets`](Unit::rally_targets) field.
#[derive(Clone)]
pub struct RallyTarget {
	/// Rally point. Position building rallied on.
	pub point: Point2,
	/// Filled if building is rallied on unit.
	pub tag: Option<u64>,
}

/// Trait for radius
pub trait Radius {
	/// Radius for struct
	fn radius(&self) -> f32 {
		0.0
	}
}

impl Radius for &Unit {
	fn radius(&self) -> f32 {
		Unit::radius(self)
	}
}
impl Radius for Unit {
	fn radius(&self) -> f32 {
		self.radius()
	}
}
