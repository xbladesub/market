use std::sync::Arc;

use apca::{ApiInfo, Client as AlpacaClient};
use config::{Config, ConfigError, File};
use serde::Deserialize;
use thiserror::Error as ThisError;
use tokio::time::{sleep, Duration};
use tracing::{error, info};
use uuid::Uuid;
use uuid7::uuid7;

use crate::{
    api::alert::{AlertType, TradeSignal, WebhookAlertData},
    trade_executor::TradeExecutor,
    App,
};

#[derive(Debug, ThisError)]
pub enum StrategyManagerError {
    #[error(transparent)]
    ConfigError(#[from] ConfigError),
    #[error(transparent)]
    AlpacaClientError(#[from] apca::Error),
    #[error("Unknown strategy - {0}")]
    UnknownStrategy(String),
    #[error("Unknown exchange - {0}")]
    UnknownExchange(String),
    #[error("Strategy {0} with id {1} is disabled")]
    StrategyDisabled(String, String),
}

// #[derive(Debug, ThisError)]
// pub enum TradeError {
//     #[error("{0}")]
//     InsufficientFunds(String),
//     #[error("Order max retries reached. {0}")]
//     MaxRetriesReached(Order),
// }

pub struct StrategyManager {
    app_state: Arc<App>,
    trade_executor: TradeExecutor,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Broker {
    Alpaca,
}

#[derive(Debug)]
pub struct Order {
    pub id: Uuid,
    pub ticker: String,
    pub order_type: AlertType,
}

impl StrategyManager {
    pub fn new(
        app_state: Arc<App>,
        trade_executor: TradeExecutor,
    ) -> Result<Self, StrategyManagerError> {
        Ok(Self {
            app_state,
            trade_executor,
        })
    }

    pub async fn process_trade_signal(
        &self,
        trade_signal: TradeSignal,
    ) -> Result<(), StrategyManagerError> {
        let order = Order {
            id: uuid7::new_v7(),
            ticker: trade_signal.ticker,
            order_type: trade_signal.alert_type,
        };

        // TODO: Retry
        let result = self
            .trade_executor
            .execute_order(&order, &trade_signal.strategy.broker)
            .await;

        Ok(())
    }
}

impl std::fmt::Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ id: {} }}", self.id)
    }
}
