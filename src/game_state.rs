//! Information updated every step stored here.

use crate::{
	action::{Action, ActionError},
	bot::{Bot, LockOwned, LockU32, Locked, Rs, Rw},
	geometry::Point2,
	ids::*,
	pixel_map::{PixelMap, VisibilityMap},
	score::Score,
	unit::Unit,
	units::Units,
	Event, FromProto, Player, SC2Result,
};
use num_traits::FromPrimitive;
use rustc_hash::FxHashSet;
use sc2_proto::{
	query::RequestQueryAvailableAbilities,
	raw::{Alliance as ProtoAlliance, PowerSource as ProtoPowerSource},
	sc2api::{Alert as ProtoAlert, Request, ResponseObservation},
};
use std::ops::{Deref, DerefMut};

/// Information about current state on current step.
///
/// Can be accessed through [`state`](crate::bot::Bot::state) field.
#[derive(Default, Clone)]
pub struct GameState {
	/// Actions executed on previous step.
	pub actions: Vec<Action>,
	/// Results on actions from previous step.
	pub action_errors: Vec<ActionError>,
	/// Bot's observation here.
	pub observation: Observation,
	// player_result,
	/// Messeges in game chat.
	pub chat: Vec<ChatMessage>,
}

pub(crate) fn update_state<B>(
	bot: &mut B,
	response_observation: &ResponseObservation,
) -> SC2Result<Vec<Event>>
where
	B: Player + DerefMut<Target = Bot> + Deref<Target = Bot>,
{
	// Game state
	let state = &mut bot.state;

	// let player_result = response_observation.get_player_result();
	state.actions = response_observation
		.get_actions()
		.iter()
		.filter_map(Option::<Action>::from_proto)
		.collect();
	state.action_errors = response_observation
		.get_action_errors()
		.iter()
		.map(ActionError::from_proto)
		.collect();
	state.chat = response_observation
		.get_chat()
		.iter()
		.map(|m| ChatMessage {
			player_id: m.get_player_id(),
			message: m.get_message().to_string(),
		})
		.collect();

	// Observation
	let obs = &mut state.observation;
	let res_obs = response_observation.get_observation();

	obs.game_loop.set_locked(res_obs.get_game_loop());
	obs.alerts = res_obs
		.get_alerts()
		.iter()
		.map(|a| Alert::from_proto(*a))
		.collect();
	obs.abilities = res_obs
		.get_abilities()
		.iter()
		.map(|a| AvailableAbility {
			id: {
				let id = a.get_ability_id();
				AbilityId::from_i32(id).unwrap_or_else(|| panic!("There's no `AbilityId` with value {}", id))
			},
			requires_point: a.get_requires_point(),
		})
		.collect();
	obs.score = Score::from_proto(res_obs.get_score());

	// Common
	let common = res_obs.get_player_common();
	obs.common = Common {
		player_id: common.get_player_id(),
		minerals: common.get_minerals(),
		vespene: common.get_vespene(),
		food_cap: common.get_food_cap(),
		food_used: common.get_food_used(),
		food_army: common.get_food_army(),
		food_workers: common.get_food_workers(),
		idle_worker_count: common.get_idle_worker_count(),
		army_count: common.get_army_count(),
		warp_gate_count: common.get_warp_gate_count(),
		larva_count: common.get_larva_count(),
	};

	// Raw
	let raw = &mut obs.raw;
	let res_raw = res_obs.get_raw_data();

	let raw_player = res_raw.get_player();
	raw.psionic_matrix = raw_player
		.get_power_sources()
		.iter()
		.map(PsionicMatrix::from_proto)
		.collect();
	raw.camera = Point2::from_proto(raw_player.get_camera());
	raw.effects = res_raw
		.get_effects()
		.iter()
		.filter(|e| EffectId::from_u32(e.get_effect_id()).is_some())
		.map(|e| Effect {
			id: {
				let id = e.get_effect_id();
				EffectId::from_u32(id).unwrap_or_else(|| panic!("There's no `EffectId` with value {}", id))
			},
			positions: e.get_pos().iter().map(Point2::from_proto).collect(),
			alliance: Alliance::from_proto(e.get_alliance()),
			owner: e.get_owner() as u32,
			radius: e.get_radius(),
		})
		.collect();
	raw.radars = res_raw
		.get_radar()
		.iter()
		.map(|r| Radar {
			pos: Point2::from_proto(r.get_pos()),
			radius: r.get_radius(),
		})
		.collect();

	let mut events = vec![];
	// Dead units
	let dead_units = res_raw.get_event().get_dead_units().to_vec();

	#[cfg(feature = "enemies_cache")]
	let enemy_is_terran = bot.enemy_race.is_terran();

	for u in &dead_units {
		let alliance = if bot.owned_tags.remove(u) {
			bot.available_frames.write_lock().remove(u);
			bot.under_construction.remove(u);
			bot.last_units_hits.write_lock().remove(u);
			bot.last_units_seen.write_lock().remove(u);
			Some(Alliance::Own)
		} else {
			let removed = bot.saved_hallucinations.remove(u);

			#[cfg(feature = "enemies_cache")]
			let removed = {
				let cache = &mut bot.units.cached;
				removed
					| cache.all.remove(*u).is_some()
					| cache.units.remove(*u).is_some()
					| cache.workers.remove(*u).is_some()
					| if enemy_is_terran {
						cache.structures.remove(*u).is_some() | cache.townhalls.remove(*u).is_some()
					} else {
						false
					}
			};

			if removed {
				Some(Alliance::Enemy)
			} else if bot.expansions.iter_mut().any(|exp| exp.minerals.remove(u)) {
				Some(Alliance::Neutral)
			} else {
				None
			}
		};

		events.push(Event::UnitDestroyed(*u, alliance));
	}

	let raw = &mut bot.state.observation.raw;
	raw.dead_units = dead_units;

	// Upgrades
	*raw.upgrades.write_lock() = raw_player
		.get_upgrade_ids()
		.iter()
		.filter(|&u| UpgradeId::from_u32(*u).is_some())
		.map(|u| UpgradeId::from_u32(*u).unwrap_or_else(|| panic!("There's no `UpgradeId` with value {}", u)))
		.collect::<FxHashSet<_>>();

	// Map
	let map_state = res_raw.get_map_state();
	// Creep
	*raw.creep.write_lock() = PixelMap::from_proto(map_state.get_creep());

	// Available abilities
	let mut req = Request::new();
	let req_query_abilities = req.mut_query().mut_abilities();
	for u in res_raw.get_units() {
		if matches!(u.get_alliance(), ProtoAlliance::value_Self) {
			let mut req_unit = RequestQueryAvailableAbilities::new();
			req_unit.set_unit_tag(u.get_tag());
			req_query_abilities.push(req_unit);
		}
	}

	let res = bot.api().send(req)?;
	*bot.abilities_units.write_lock() = res
		.get_query()
		.get_abilities()
		.iter()
		.map(|a| {
			(
				a.get_unit_tag(),
				a.get_abilities()
					.iter()
					.filter_map(|ab| AbilityId::from_i32(ab.get_ability_id()))
					.collect(),
			)
		})
		.collect();

	// Get visiblity
	let visibility = VisibilityMap::from_proto(map_state.get_visibility());
	// Get units
	let units = res_raw
		.get_units()
		.iter()
		.map(|u| Unit::from_proto(Rs::clone(&bot.data_for_unit), &visibility, u))
		.collect::<Units>();

	// Set visiblity
	bot.state.observation.raw.visibility = visibility;

	// Updating units
	bot.update_units(units);

	// Events
	let mut owned_tags = vec![];
	let mut under_construction = vec![];
	let mut construction_complete = vec![];
	for (tag, u) in bot.units.my.all.pairs() {
		if !bot.owned_tags.contains(tag) {
			owned_tags.push(*tag);
			if u.is_structure() {
				if !(u.is_placeholder() || u.type_id() == UnitTypeId::KD8Charge) {
					if u.is_ready() {
						events.push(Event::ConstructionComplete(*tag));
					} else {
						events.push(Event::ConstructionStarted(*tag));
						under_construction.push(*tag);
					}
				}
			} else {
				events.push(Event::UnitCreated(*tag));
			}
		} else if bot.under_construction.contains(tag) && u.is_ready() {
			construction_complete.push(*tag);
			events.push(Event::ConstructionComplete(*tag));
		}
	}
	for tag in owned_tags {
		bot.owned_tags.insert(tag);
	}
	for tag in under_construction {
		bot.under_construction.insert(tag);
	}
	for tag in construction_complete {
		bot.under_construction.remove(&tag);
	}

	if bot.enemy_race.is_random() {
		if let Some(race) = bot
			.units
			.enemy
			.all
			.iter()
			.map(|u| u.race())
			.find(|r| !r.is_random())
		{
			events.push(Event::RandomRaceDetected(race));
			bot.enemy_race = race;
		}
	}

	Ok(events)
}

/// Messege in game chat.
#[derive(Clone)]
pub struct ChatMessage {
	/// Id of player who sent that message.
	pub player_id: u32,
	/// Actual message.
	pub message: String,
}

/// Bot's observation stored here.
/// Can be accessed through [`state.observation`](GameState::observation).
#[derive(Default, Clone)]
pub struct Observation {
	pub(crate) game_loop: Rs<LockU32>,
	/// Common information from the observation.
	pub common: Common,
	/// Alerts appearing when some kind of things happen.
	pub alerts: Vec<Alert>,
	pub abilities: Vec<AvailableAbility>,
	/// SC2 Score data.
	pub score: Score,
	/// Data of raw interface.
	pub raw: RawData,
}
impl Observation {
	/// Current game tick (frame).
	pub fn game_loop(&self) -> u32 {
		self.game_loop.get_locked()
	}
}

/// Bot's observation stored here.
/// Can be accessed through [`state.observation.raw`](Observation::raw).
#[derive(Default, Clone)]
pub struct RawData {
	/// Protoss power from pylons.
	pub psionic_matrix: Vec<PsionicMatrix>,
	/// Current camera position
	pub camera: Point2,
	/// Bot's ready upgrades.
	pub upgrades: Rw<FxHashSet<UpgradeId>>,
	/// Bot's visibility map.
	pub visibility: VisibilityMap,
	/// Creep on the map.
	pub creep: Rw<PixelMap>,
	/// Tags of units which died last step.
	pub dead_units: Vec<u64>,
	/// Current effects on the map.
	pub effects: Vec<Effect>,
	/// Terran radars on the map.
	pub radars: Vec<Radar>,
}

/// Power matrix from the pylon or warp prism, used to give power to buildings and warp units on it.
#[derive(Clone)]
pub struct PsionicMatrix {
	/// Position of psionic matrix source.
	pub pos: Point2,
	/// Radius of the matrix: shold be `6.5` for pylon and `3.75` for warp prism in phsing mode.
	pub radius: f32,
	/// Tag of unit that is source of power matrix.
	pub tag: u64,
}
impl FromProto<&ProtoPowerSource> for PsionicMatrix {
	fn from_proto(ps: &ProtoPowerSource) -> Self {
		Self {
			pos: Point2::from_proto(ps.get_pos()),
			radius: ps.get_radius(),
			tag: ps.get_tag(),
		}
	}
}

/// There are different effects in SC2, some of them can harm your units,
/// so take them into account when microing.
///
/// All effects stored in [state.observation.raw.effects](RawData::effects).
#[derive(Clone)]
pub struct Effect {
	/// Type of the effect.
	pub id: EffectId,
	/// Positions covered by this effect.
	pub positions: Vec<Point2>,
	/// Is this effect yours or opponent's.
	pub alliance: Alliance,
	/// Player id of effect's owner.
	pub owner: u32,
	/// Additional radius covered by effect around every it's position.
	pub radius: f32,
}

/// The alliance of unit or effect to your bot.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Alliance {
	/// Your own objects.
	Own,
	/// Owned by your allias (in 2v2, 3v3 an 4v4 games).
	Ally,
	/// Just neutral object.
	Neutral,
	/// Owned by your opponent.
	Enemy,
}
impl Alliance {
	pub fn is_mine(self) -> bool {
		matches!(self, Alliance::Own)
	}
	pub fn is_enemy(self) -> bool {
		matches!(self, Alliance::Enemy)
	}
	pub fn is_neutral(self) -> bool {
		matches!(self, Alliance::Neutral)
	}
	pub fn is_ally(self) -> bool {
		matches!(self, Alliance::Ally)
	}
}
impl FromProto<ProtoAlliance> for Alliance {
	fn from_proto(alliance: ProtoAlliance) -> Self {
		match alliance {
			ProtoAlliance::value_Self => Alliance::Own,
			ProtoAlliance::Ally => Alliance::Ally,
			ProtoAlliance::Neutral => Alliance::Neutral,
			ProtoAlliance::Enemy => Alliance::Enemy,
		}
	}
}

/// Radar point on the map.
#[derive(Clone)]
pub struct Radar {
	/// Position where radar is.
	pub pos: Point2,
	/// Radius covered by radar (Pretty sure it's `12`).
	pub radius: f32,
}

/// Common information of player.
#[derive(Default, Clone)]
pub struct Common {
	/// In-game player id.
	pub player_id: u32,
	/// Amount of minerals bot currently has.
	pub minerals: u32,
	/// Amount of vespene gas bot currently has.
	pub vespene: u32,
	/// Supply capacity.
	pub food_cap: u32,
	/// Supply used in total.
	pub food_used: u32,
	/// Supply used by combat units.
	pub food_army: u32,
	/// Supply used by workers (workers count).
	pub food_workers: u32,
	/// The count of your idle workers.
	pub idle_worker_count: u32,
	/// The count of your combat units.
	pub army_count: u32,
	/// The count of your free warp gates.
	pub warp_gate_count: u32,
	/// The count of your larva.
	pub larva_count: u32,
}

/// Different kinds of alert that can happen.
/// All alerts stored in [`state.observation.alerts`](Observation::alerts).
#[allow(missing_docs)]
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
pub enum Alert {
	AlertError,
	AddOnComplete,
	BuildingComplete,
	BuildingUnderAttack,
	LarvaHatched,
	MergeComplete,
	MineralsExhausted,
	MorphComplete,
	MothershipComplete,
	MULEExpired,
	NuclearLaunchDetected,
	NukeComplete,
	NydusWormDetected,
	ResearchComplete,
	TrainError,
	TrainUnitComplete,
	TrainWorkerComplete,
	TransformationComplete,
	UnitUnderAttack,
	UpgradeComplete,
	VespeneExhausted,
	WarpInComplete,
}
impl FromProto<ProtoAlert> for Alert {
	fn from_proto(alert: ProtoAlert) -> Self {
		match alert {
			ProtoAlert::AlertError => Alert::AlertError,
			ProtoAlert::AddOnComplete => Alert::AddOnComplete,
			ProtoAlert::BuildingComplete => Alert::BuildingComplete,
			ProtoAlert::BuildingUnderAttack => Alert::BuildingUnderAttack,
			ProtoAlert::LarvaHatched => Alert::LarvaHatched,
			ProtoAlert::MergeComplete => Alert::MergeComplete,
			ProtoAlert::MineralsExhausted => Alert::MineralsExhausted,
			ProtoAlert::MorphComplete => Alert::MorphComplete,
			ProtoAlert::MothershipComplete => Alert::MothershipComplete,
			ProtoAlert::MULEExpired => Alert::MULEExpired,
			ProtoAlert::NuclearLaunchDetected => Alert::NuclearLaunchDetected,
			ProtoAlert::NukeComplete => Alert::NukeComplete,
			ProtoAlert::NydusWormDetected => Alert::NydusWormDetected,
			ProtoAlert::ResearchComplete => Alert::ResearchComplete,
			ProtoAlert::TrainError => Alert::TrainError,
			ProtoAlert::TrainUnitComplete => Alert::TrainUnitComplete,
			ProtoAlert::TrainWorkerComplete => Alert::TrainWorkerComplete,
			ProtoAlert::TransformationComplete => Alert::TransformationComplete,
			ProtoAlert::UnitUnderAttack => Alert::UnitUnderAttack,
			ProtoAlert::UpgradeComplete => Alert::UpgradeComplete,
			ProtoAlert::VespeneExhausted => Alert::VespeneExhausted,
			ProtoAlert::WarpInComplete => Alert::WarpInComplete,
		}
	}
}

#[derive(Clone)]
pub struct AvailableAbility {
	pub id: AbilityId,
	pub requires_point: bool,
}
