use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use simkit_core::{ids::SimId, impl_simid};

use crate::{
    model::components::{Fixture, Item, Pawn},
    tasks::Task,
};

impl_simid!(PawnId, Pawn);
impl_simid!(ItemId, Item);
impl_simid!(TaskId, Task);
impl_simid!(FixtureId, Fixture);
