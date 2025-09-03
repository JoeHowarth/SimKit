use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use simkit_core::{ids::SimId, impl_simid};

impl_simid!(PawnId);
impl_simid!(ItemId);
impl_simid!(TaskId);
impl_simid!(FixtureId);
