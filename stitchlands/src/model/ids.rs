use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use simkit_core::{ids::SimId, impl_simid};

// #[derive(
//     Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Reflect,
// Serialize, Deserialize, )]
// pub struct PawnId(pub u64);

// #[derive(
//     Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Reflect,
// Serialize, Deserialize, )]
// pub struct ItemId(pub u64);

// #[derive(
//     Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Reflect,
// Serialize, Deserialize, )]
// pub struct ZoneId(pub u64);

// #[derive(
//     Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Reflect,
// Serialize, Deserialize, )]
// pub struct BlueprintId(pub u64);

// #[derive(
//     Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Reflect,
// Serialize, Deserialize, )]
// pub struct BedId(pub u64);

// #[derive(
//     Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Reflect,
// Serialize, Deserialize, )]
// pub struct TaskId(pub u64);

// #[derive(
//     Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Reflect,
// Serialize, Deserialize, )]
// pub struct FixtureId(pub u64);

impl_simid!(PawnId);
impl_simid!(ItemId);
impl_simid!(ZoneId);
impl_simid!(BlueprintId);
impl_simid!(BedId);
impl_simid!(TaskId);
impl_simid!(FixtureId);
