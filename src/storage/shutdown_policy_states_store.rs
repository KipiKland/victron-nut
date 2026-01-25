use std::{error::Error, io::ErrorKind, path::PathBuf};

use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::{AsyncReadExt, AsyncWriteExt}};

use crate::{config::Configuration, logic_manager::ShutdownPolicyState};

#[derive(Debug, PartialEq, Clone)]
#[derive(Serialize, Deserialize)]
struct StoredShutdownPolicyState {
    policy_name: String,
    triggered_shutdown: bool,
	restart_required: bool
}

#[derive(Debug, PartialEq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct StoreData {
    states: Vec<StoredShutdownPolicyState>
}
impl StoreData {
    pub fn new() -> Self {
        Self {
            states: Vec::new()
        }
    }

    pub fn get_shutdown_policy_states(&self, config: &Configuration) -> Vec<ShutdownPolicyState> {
        let mut result = Vec::new();

        for policy_config in config.shutdown_policies.iter() {
            match self.states.iter().find(|s| s.policy_name.eq(&policy_config.name)) {
                Some(stored_state) => {
                    result.push(ShutdownPolicyState {
                        policy_config: policy_config.clone(),
                        triggered_shutdown: stored_state.triggered_shutdown,
                        restart_required: stored_state.restart_required
                    });
                },
                None => {
                    result.push(ShutdownPolicyState {
                        policy_config: policy_config.clone(),
                        triggered_shutdown: false,
                        restart_required: false
                    });
                }
            }
        }

        result
    }
}

pub struct ShutdownPolicyStatesStore {
    filepath: PathBuf
}
impl ShutdownPolicyStatesStore {
    pub fn new(filepath: PathBuf) -> Self {
        Self {
            filepath: filepath
        }
    }

    fn map_state_to_stored(state: &ShutdownPolicyState) -> StoredShutdownPolicyState {
        StoredShutdownPolicyState {
            policy_name: state.policy_config.name.clone(),
            triggered_shutdown: state.triggered_shutdown,
            restart_required: state.restart_required
        }
    }

    pub fn create_snapshot(&self, shutdown_policy_states: &Vec<ShutdownPolicyState>) -> StoreData {
        StoreData {
            states: shutdown_policy_states.iter().map(Self::map_state_to_stored).collect()
        }
    }

    pub async fn store(&self, snapshot: &StoreData) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string(&snapshot)?;

        let mut file = File::create(&self.filepath).await?;
        file.write_all(json.as_bytes()).await?;

        Ok(())
    }

    pub async fn read(&self) -> Result<StoreData, Box<dyn Error>> {
        let mut json = String::new();

        match File::open(&self.filepath).await {
            Ok(mut file) => {
                file.read_to_string(&mut json).await?;

                let data = serde_json::from_str(&json)?;
                Ok(data)
            },
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(StoreData::new()),
            Err(e) => Err(Box::new(e))
        }
    }
}