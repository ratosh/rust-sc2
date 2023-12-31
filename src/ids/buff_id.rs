#![allow(deprecated)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, FromPrimitive, ToPrimitive, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BuffId {
	Null = 0,
	Radar25 = 1,
	Tauntb = 2,
	DisableAbils = 3,
	TransientMorph = 4,
	GravitonBeam = 5,
	GhostCloak = 6,
	BansheeCloak = 7,
	PowerUserWarpable = 8,
	VortexBehaviorEnemy = 9,
	Corruption = 10,
	QueenSpawnLarvaTimer = 11,
	GhostHoldFire = 12,
	GhostHoldFireB = 13,
	Leech = 14,
	LeechDisableAbilities = 15,
	EMPDecloak = 16,
	FungalGrowth = 17,
	GuardianShield = 18,
	SeekerMissileTimeout = 19,
	TimeWarpProduction = 20,
	Ethereal = 21,
	NeuralParasite = 22,
	NeuralParasiteWait = 23,
	StimpackMarauder = 24,
	SupplyDrop = 25,
	_250mmStrikeCannons = 26,
	Stimpack = 27,
	PsiStorm = 28,
	CloakFieldEffect = 29,
	Charging = 30,
	AIDangerBuff = 31,
	VortexBehavior = 32,
	Slow = 33,
	TemporalRiftUnit = 34,
	SheepBusy = 35,
	Contaminated = 36,
	TimeScaleConversionBehavior = 37,
	BlindingCloudStructure = 38,
	CollapsibleRockTowerConjoinedSearch = 39,
	CollapsibleRockTowerRampDiagonalConjoinedSearch = 40,
	CollapsibleTerranTowerConjoinedSearch = 41,
	CollapsibleTerranTowerRampDiagonalConjoinedSearch = 42,
	DigesterCreepSprayVision = 43,
	InvulnerabilityShield = 44,
	MineDroneCountdown = 45,
	MothershipStasis = 46,
	MothershipStasisCaster = 47,
	MothershipCoreEnergizeVisual = 48,
	OracleRevelation = 49,
	GhostSnipeDoT = 50,
	NexusPhaseShift = 51,
	NexusInvulnerability = 52,
	RoughTerrainSearch = 53,
	RoughTerrainSlow = 54,
	OracleCloakField = 55,
	OracleCloakFieldEffect = 56,
	ScryerFriendly = 57,
	SpectreShield = 58,
	ViperConsumeStructure = 59,
	RestoreShields = 60,
	MercenaryCycloneMissiles = 61,
	MercenarySensorDish = 62,
	MercenaryShield = 63,
	Scryer = 64,
	StunRoundInitialBehavior = 65,
	BuildingShield = 66,
	LaserSight = 67,
	ProtectiveBarrier = 68,
	CorruptorGroundAttackDebuff = 69,
	BattlecruiserAntiAirDisable = 70,
	BuildingStasis = 71,
	Stasis = 72,
	ResourceStun = 73,
	MaximumThrust = 74,
	ChargeUp = 75,
	CloakUnit = 76,
	NullField = 77,
	Rescue = 78,
	Benign = 79,
	LaserTargeting = 80,
	Engage = 81,
	CapResource = 82,
	BlindingCloud = 83,
	DoomDamageDelay = 84,
	EyeStalk = 85,
	BurrowCharge = 86,
	Hidden = 87,
	MineDroneDOT = 88,
	MedivacSpeedBoost = 89,
	ExtendBridgeExtendingBridgeNEWide8Out = 90,
	ExtendBridgeExtendingBridgeNWWide8Out = 91,
	ExtendBridgeExtendingBridgeNEWide10Out = 92,
	ExtendBridgeExtendingBridgeNWWide10Out = 93,
	ExtendBridgeExtendingBridgeNEWide12Out = 94,
	ExtendBridgeExtendingBridgeNWWide12Out = 95,
	PhaseShield = 96,
	Purify = 97,
	VoidSiphon = 98,
	OracleWeapon = 99,
	AntiAirWeaponSwitchCooldown = 100,
	ArbiterMPStasisField = 101,
	ImmortalOverload = 102,
	CloakingFieldTargeted = 103,
	LightningBomb = 104,
	OraclePhaseShift = 105,
	ReleaseInterceptorsCooldown = 106,
	ReleaseInterceptorsTimedLifeWarning = 107,
	ReleaseInterceptorsWanderDelay = 108,
	ReleaseInterceptorsBeacon = 109,
	ArbiterMPCloakFieldEffect = 110,
	PurificationNova = 111,
	CorruptionBombDamage = 112,
	CorsairMPDisruptionWeb = 113,
	DisruptorPush = 114,
	LightofAiur = 115,
	LockOn = 116,
	Overcharge = 117,
	OverchargeDamage = 118,
	OverchargeSpeedBoost = 119,
	SeekerMissile = 120,
	TemporalField = 121,
	VoidRaySwarmDamageBoost = 122,
	VoidMPImmortalReviveSupressed = 123,
	DevourerMPAcidSpores = 124,
	DefilerMPConsume = 125,
	DefilerMPDarkSwarm = 126,
	DefilerMPPlague = 127,
	QueenMPEnsnare = 128,
	OracleStasisTrapTarget = 129,
	SelfRepair = 130,
	AggressiveMutation = 131,
	ParasiticBomb = 132,
	ParasiticBombUnitKU = 133,
	ParasiticBombSecondaryUnitSearch = 134,
	AdeptDeathCheck = 135,
	LurkerHoldFire = 136,
	LurkerHoldFireB = 137,
	TimeStopStun = 138,
	SlaynElementalGrabStun = 139,
	PurificationNovaPost = 140,
	DisableInterceptors = 141,
	BypassArmorDebuffOne = 142,
	BypassArmorDebuffTwo = 143,
	BypassArmorDebuffThree = 144,
	ChannelSnipeCombat = 145,
	TempestDisruptionBlastStunBehavior = 146,
	GravitonPrison = 147,
	InfestorDisease = 148,
	SSLightningProjector = 149,
	PurifierPlanetCrackerCharge = 150,
	SpectreCloaking = 151,
	WraithCloak = 152,
	PsytrousOxide = 153,
	BansheeCloakCrossSpectrumDampeners = 154,
	SSBattlecruiserHunterSeekerTimeout = 155,
	SSStrongerEnemyBuff = 156,
	SSTerraTronArmMissileTargetCheck = 157,
	SSMissileTimeout = 158,
	SSLeviathanBombCollisionCheck = 159,
	SSLeviathanBombExplodeTimer = 160,
	SSLeviathanBombMissileTargetCheck = 161,
	SSTerraTronCollisionCheck = 162,
	SSCarrierBossCollisionCheck = 163,
	SSCorruptorMissileTargetCheck = 164,
	SSInvulnerable = 165,
	SSLeviathanTentacleMissileTargetCheck = 166,
	SSLeviathanTentacleMissileTargetCheckInverted = 167,
	SSLeviathanTentacleTargetDeathDelay = 168,
	SSLeviathanTentacleMissileScanSwapDelay = 169,
	SSPowerUpDiagonal2 = 170,
	SSBattlecruiserCollisionCheck = 171,
	SSTerraTronMissileSpinnerMissileLauncher = 172,
	SSTerraTronMissileSpinnerCollisionCheck = 173,
	SSTerraTronMissileLauncher = 174,
	SSBattlecruiserMissileLauncher = 175,
	SSTerraTronStun = 176,
	SSVikingRespawn = 177,
	SSWraithCollisionCheck = 178,
	SSScourgeMissileTargetCheck = 179,
	SSScourgeDeath = 180,
	SSSwarmGuardianCollisionCheck = 181,
	SSFighterBombMissileDeath = 182,
	SSFighterDroneDamageResponse = 183,
	SSInterceptorCollisionCheck = 184,
	SSCarrierCollisionCheck = 185,
	SSMissileTargetCheckVikingDrone = 186,
	SSMissileTargetCheckVikingStrong1 = 187,
	SSMissileTargetCheckVikingStrong2 = 188,
	SSPowerUpHealth1 = 189,
	SSPowerUpHealth2 = 190,
	SSPowerUpStrong = 191,
	SSPowerupMorphToBomb = 192,
	SSPowerupMorphToHealth = 193,
	SSPowerupMorphToSideMissiles = 194,
	SSPowerupMorphToStrongerMissiles = 195,
	SSCorruptorCollisionCheck = 196,
	SSScoutCollisionCheck = 197,
	SSPhoenixCollisionCheck = 198,
	SSScourgeCollisionCheck = 199,
	SSLeviathanCollisionCheck = 200,
	SSScienceVesselCollisionCheck = 201,
	SSTerraTronSawCollisionCheck = 202,
	SSLightningProjectorCollisionCheck = 203,
	ShiftDelay = 204,
	BioStasis = 205,
	PersonalCloakingFree = 206,
	EMPDrain = 207,
	MindBlastStun = 208,
	_330mmBarrageCannons = 209,
	VoodooShield = 210,
	SpectreCloakingFree = 211,
	UltrasonicPulseStun = 212,
	Irradiate = 213,
	NydusWormLavaInstantDeath = 214,
	PredatorCloaking = 215,
	PsiDisruption = 216,
	MindControl = 217,
	QueenKnockdown = 218,
	ScienceVesselCloakField = 219,
	SporeCannonMissile = 220,
	ArtanisTemporalRiftUnit = 221,
	ArtanisCloakingFieldEffect = 222,
	ArtanisVortexBehavior = 223,
	Incapacitated = 224,
	KarassPsiStorm = 225,
	DutchMarauderSlow = 226,
	JumpStompStun = 227,
	JumpStompFStun = 228,
	RaynorMissileTimedLife = 229,
	PsionicShockwaveHeightAndStun = 230,
	ShadowClone = 231,
	AutomatedRepair = 232,
	Slimed = 233,
	RaynorTimeBombMissile = 234,
	RaynorTimeBombUnit = 235,
	TychusCommandoStimPack = 236,
	ViralPlasma = 237,
	Napalm = 238,
	BurstCapacitorsDamageBuff = 239,
	ColonyInfestation = 240,
	Domination = 241,
	EMPBurst = 242,
	HybridCZergyRoots = 243,
	HybridFZergyRoots = 244,
	LockdownB = 245,
	SpectreLockdownB = 246,
	VoodooLockdown = 247,
	ZeratulStun = 248,
	BuildingScarab = 249,
	VortexBehaviorEradicator = 250,
	GhostBlast = 251,
	HeroicBuff03 = 252,
	CannonRadar = 253,
	SSMissileTargetCheckViking = 254,
	SSMissileTargetCheck = 255,
	SSMaxSpeed = 256,
	SSMaxAcceleration = 257,
	SSPowerUpDiagonal1 = 258,
	Water = 259,
	DefensiveMatrix = 260,
	TestAttribute = 261,
	TestVeterancy = 262,
	ShredderSwarmDamageApply = 263,
	CorruptorInfesting = 264,
	MercGroundDropDelay = 265,
	MercGroundDrop = 266,
	MercAirDropDelay = 267,
	SpectreHoldFire = 268,
	SpectreHoldFireB = 269,
	ItemGravityBombs = 270,
	CarryMineralFieldMinerals = 271,
	CarryHighYieldMineralFieldMinerals = 272,
	CarryHarvestableVespeneGeyserGas = 273,
	CarryHarvestableVespeneGeyserGasProtoss = 274,
	CarryHarvestableVespeneGeyserGasZerg = 275,
	PermanentlyCloaked = 276,
	RavenScramblerMissile = 277,
	RavenShredderMissileTimeout = 278,
	RavenShredderMissileTint = 279,
	RavenShredderMissileArmorReduction = 280,
	ChronoBoostEnergyCost = 281,
	NexusShieldRechargeOnPylonBehavior = 282,
	NexusShieldRechargeOnPylonBehaviorSecondaryOnTarget = 283,
	InfestorEnsnare = 284,
	InfestorEnsnareMakePrecursorReheightSource = 285,
	NexusShieldOvercharge = 286,
	ParasiticBombDelayTimedLife = 287,
	Transfusion = 288,
	AccelerationZoneTemporalField = 289,
	AccelerationZoneFlyingTemporalField = 290,
	InhibitorZoneFlyingTemporalField = 291,
	DummyBuff000 = 292,
	InhibitorZoneTemporalField = 293,
	ResonatingGlaivesPhaseShift = 294,
	NeuralParasiteChildren = 295,
	AmorphousArmorcloud = 296,
	RavenShredderMissileArmorReductionUISubtruct = 297,
	BatteryOvercharge = 298,
	DummyBuff001 = 299,
	DummyBuff002 = 300,
	DummyBuff003 = 301,
	DummyBuff004 = 302,
	DummyBuff005 = 303,
	OnCreepQueen = 304,
	LoadOutSprayTracker = 305,
	CloakField = 306,
	TakenDamage = 307,
	RavenScramblerMissileCarrier = 308,
}
