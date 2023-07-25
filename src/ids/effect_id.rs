#![allow(deprecated)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, FromPrimitive, ToPrimitive, Copy, Clone, PartialEq, Eq, Hash)]
pub enum EffectId {
	Null = 0,
	PsiStormPersistent = 1,
	GuardianShieldPersistent = 2,
	TemporalFieldGrowingBubbleCreatePersistent = 3,
	TemporalFieldAfterBubbleCreatePersistent = 4,
	ThermalLancesForward = 5,
	ScannerSweep = 6,
	NukePersistent = 7,
	LiberatorTargetMorphDelayPersistent = 8,
	LiberatorTargetMorphPersistent = 9,
	BlindingCloudCP = 10,
	RavagerCorrosiveBileCP = 11,
	LurkerMP = 12,
}
