use std::{
    io::{BufRead, Write},
    path::PathBuf,
};

use bevy::{prelude::*, reflect::Reflect};
use serde::{Deserialize, Serialize};

use crate::{simulation::Simulation, Tick, POD};

#[derive(Resource)]
pub struct JournalConfig {
    pub path: PathBuf,
    pub enabled: bool,
}

#[derive(Reflect, Clone, Serialize, Deserialize)]
#[serde(bound(
    serialize = "S::State: serde::Serialize, S::Actions: serde::Serialize, \
                 S::Event: serde::Serialize",
    deserialize = "S::State: serde::de::DeserializeOwned, S::Actions: \
                   serde::de::DeserializeOwned, S::Event: \
                   serde::de::DeserializeOwned"
))]
pub struct Journal<S: Simulation>(
    pub Vec<JournalLine<S::State, S::Actions, S::Event>>,
);

#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[serde(bound(
    serialize = "S: serde::Serialize, A: serde::Serialize, E: serde::Serialize",
    deserialize = "S: serde::de::DeserializeOwned, A: \
                   serde::de::DeserializeOwned, E: serde::de::DeserializeOwned"
))]
pub struct JournalLine<S: POD, A: POD, E: POD> {
    pub tick: Tick,
    pub state: S,
    pub actions: Vec<A>,
    pub events: Vec<E>,
}

impl JournalConfig {
    pub fn init_journal_file<S: Simulation>(
        journal_config: Res<JournalConfig>,
        init_state: Res<S::State>,
    ) {
        if !journal_config.enabled {
            return;
        }

        let file = std::fs::File::create(journal_config.path.clone()).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        let line = JournalLine::<S::State, S::Actions, S::Event> {
            tick: Tick(0),
            state: init_state.clone(),
            actions: Vec::new(),
            events: Vec::new(),
        };
        serde_json::to_writer(&mut writer, &line).unwrap();
        writer.write_all(b"\n").unwrap();
        writer.flush().unwrap();
    }

    pub fn write_update<'a, S: Simulation>(
        journal_config: Res<JournalConfig>,
        state: &S::State,
        actions: &[&S::Actions],
        events: impl IntoIterator<Item = &'a S::Event>,
    ) {
        if !journal_config.enabled {
            trace!("Journal is disabled");
            return;
        }

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(journal_config.path.clone())
            .unwrap();

        let mut writer = std::io::BufWriter::new(file);

        let line = JournalLine::<S::State, S::Actions, S::Event> {
            tick: Tick(0),
            state: state.clone(),
            actions: actions
                .iter()
                .map(|a: &&S::Actions| (*a).clone())
                .collect::<Vec<_>>(),
            events: events.into_iter().cloned().collect(),
        };

        info!(tick = line.tick.0, "Writing update to journal");
        serde_json::to_writer(&mut writer, &line).unwrap();

        writer.write_all(b"\n").unwrap();
        writer.flush().unwrap();
    }

    pub fn load_journal<S: Simulation>(&self) -> Journal<S> {
        let file = std::fs::File::open(self.path.clone()).unwrap();
        let reader = std::io::BufReader::new(file);
        let mut lines = Vec::new();
        for line in reader.lines() {
            let line = line.unwrap();
            let line = serde_json::from_str(&line).unwrap();
            lines.push(line);
        }
        debug!("Loaded {} lines from journal", lines.len());
        Journal(lines)
    }
}

impl Default for JournalConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("journal.json"),
            enabled: true,
        }
    }
}
