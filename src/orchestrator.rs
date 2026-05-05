//! Top-level daemon wiring.

use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::{CascadeDispatcher, EventCursor, EventSequence, GcClient, Result};

#[derive(Debug, Clone)]
pub struct OrchestratorConfiguration {
    city_path: PathBuf,
    state_path: PathBuf,
    idle_sleep: Duration,
    run_once: bool,
}

impl OrchestratorConfiguration {
    pub fn new(
        city_path: impl Into<PathBuf>,
        state_path: impl Into<PathBuf>,
        idle_sleep: Duration,
        run_once: bool,
    ) -> Self {
        Self {
            city_path: city_path.into(),
            state_path: state_path.into(),
            idle_sleep,
            run_once,
        }
    }

    pub fn with_default_state_path(
        city_path: impl Into<PathBuf>,
        idle_sleep: Duration,
        run_once: bool,
    ) -> Self {
        let city_path = city_path.into();
        let state_path = city_path.join(".gc").join("orchestrator.redb");
        Self::new(city_path, state_path, idle_sleep, run_once)
    }

    pub fn city_path(&self) -> &Path {
        &self.city_path
    }

    pub fn state_path(&self) -> &Path {
        &self.state_path
    }

    pub fn idle_sleep(&self) -> Duration {
        self.idle_sleep
    }

    pub fn run_once(&self) -> bool {
        self.run_once
    }
}

pub struct Orchestrator {
    configuration: OrchestratorConfiguration,
    event_cursor: EventCursor,
    gc_client: GcClient,
    dispatcher: CascadeDispatcher,
}

impl Orchestrator {
    pub fn new(configuration: OrchestratorConfiguration) -> Result<Self> {
        let gc_client = GcClient::new(configuration.city_path().to_owned());
        let dispatcher = CascadeDispatcher::new(gc_client.clone());
        let event_cursor = EventCursor::open(configuration.state_path().to_owned())?;
        Ok(Self {
            configuration,
            event_cursor,
            gc_client,
            dispatcher,
        })
    }

    pub fn run(&self) -> Result<()> {
        self.initialize_cursor()?;

        if self.configuration.run_once() {
            self.process_once()?;
            return Ok(());
        }

        loop {
            let processed_events = self.process_once()?;
            if processed_events == 0 {
                thread::sleep(self.configuration.idle_sleep());
            }
        }
    }

    pub fn process_once(&self) -> Result<usize> {
        let cursor = self.current_cursor()?;
        let batch = self.gc_client.events_after(cursor)?;
        let mut processed_events = 0;

        for event in batch.into_events() {
            let record = self.dispatcher.dispatch(&event, &self.event_cursor)?;
            self.event_cursor.record_dispatch(&record)?;
            self.event_cursor.advance(event.sequence())?;
            processed_events += 1;
        }

        Ok(processed_events)
    }

    fn initialize_cursor(&self) -> Result<()> {
        if self.event_cursor.current()?.is_none() {
            let sequence = self.gc_client.current_sequence()?;
            self.event_cursor.advance(sequence)?;
            eprintln!("orchestrator: initialized cursor at event sequence {sequence}");
        }
        Ok(())
    }

    fn current_cursor(&self) -> Result<EventSequence> {
        self.event_cursor
            .current()
            .map(|cursor| cursor.unwrap_or_else(|| EventSequence::new(0)))
    }
}
