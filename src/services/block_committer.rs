use async_trait::async_trait;
use fuels::types::block::Block as FuelBlock;
use tokio::sync::{mpsc::Receiver, Mutex};
use tracing::{error, info};

use crate::{
    adapters::{
        ethereum_adapter::EthereumAdapter,
        runner::Runner,
        storage::{BlockSubmission, Storage},
    },
    errors::Result,
};

#[allow(dead_code)]
pub struct BlockCommitter {
    rx_block: Mutex<Receiver<FuelBlock>>,
    ethereum_rpc: Box<dyn EthereumAdapter>,
    storage: Box<dyn Storage>,
}

impl BlockCommitter {
    pub fn new(
        rx_block: Receiver<FuelBlock>,
        ethereum_rpc: impl EthereumAdapter + 'static,
        storage: impl Storage + 'static,
    ) -> Self {
        Self {
            rx_block: Mutex::new(rx_block),
            ethereum_rpc: Box::new(ethereum_rpc),
            storage: Box::new(storage),
        }
    }

    async fn dequeue(&self) -> Option<FuelBlock> {
        self.rx_block.lock().await.recv().await
    }

    async fn submit_block(&self, fuel_block: FuelBlock) -> Result<()> {
        // let submitted_at_height = match self.ethereum_rpc.get_latest_eth_block().await {
        //     Ok(submitted_at_height) => submitted_at_height,
        //     Err(error) => {
        //         error!("{error}");
        //         continue;
        //     }
        // };
        let submitted_at_height = 0.into();

        let fuel_block_height = fuel_block.header.height;

        let submission = BlockSubmission {
            fuel_block_height,
            submitted_at_height,
            fuel_block_hash: fuel_block.id,
            completed: false,
        };

        self.storage.insert(submission).await?;

        // if we have a network failure the DB entry will be left at completed:false.
        self.ethereum_rpc.submit(fuel_block).await?;

        Ok(())
    }
}

#[async_trait]
impl Runner for BlockCommitter {
    async fn run(&self) -> Result<()> {
        // listen for new blocks
        while let Some(fuel_block) = self.dequeue().await {
            let block_hash = fuel_block.id;
            let block_height = fuel_block.header.height;
            if let Err(error) = self.submit_block(fuel_block).await {
                error!("{error}");
            } else {
                info!(
                    "Submitted fuel block! (block_hash: {}, block_height: {})",
                    block_hash, block_height
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ethers::types::{H256, U64};
    use fuels::{tx::Bytes32, types::block::Header as FuelBlockHeader};
    use mockall::predicate;

    use super::*;
    use crate::adapters::{ethereum_adapter::MockEthereumAdapter, storage::sqlite_db::SqliteDb};

    #[tokio::test]
    async fn block_committer_will_submit_and_write_block() {
        // given
        let block_height = 5;
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let block = given_a_block(block_height);
        let storage = SqliteDb::temporary().await.unwrap();
        let eth_rpc_mock = given_eth_rpc_that_expects(block.clone());
        tx.try_send(block.clone()).unwrap();

        // when
        spawn_committer_and_run_until_timeout(rx, eth_rpc_mock, storage.clone()).await;

        //then
        let last_submission = storage.submission_w_latest_block().await.unwrap().unwrap();
        assert_eq!(block_height, last_submission.fuel_block_height);
    }

    fn given_a_block(block_height: u32) -> FuelBlock {
        let header = FuelBlockHeader {
            id: Bytes32::zeroed(),
            da_height: 0,
            transactions_count: 0,
            message_receipt_count: 0,
            transactions_root: Bytes32::zeroed(),
            message_receipt_root: Bytes32::zeroed(),
            height: block_height,
            prev_root: Bytes32::zeroed(),
            time: None,
            application_hash: Bytes32::zeroed(),
        };

        FuelBlock {
            id: Bytes32::default(),
            header,
            transactions: vec![],
        }
    }

    fn given_tx_hash() -> H256 {
        "0x049d33c83c7c4115521d47f5fd285ee9b1481fe4a172e4f208d685781bea1ecc"
            .parse()
            .unwrap()
    }

    fn given_eth_rpc_that_expects(block: FuelBlock) -> MockEthereumAdapter {
        let mut eth_rpc_mock = MockEthereumAdapter::new();
        eth_rpc_mock
            .expect_submit()
            .with(predicate::eq(block))
            .return_once(move |_| Ok(()));

        eth_rpc_mock
            .expect_get_latest_eth_block()
            .return_once(move || Ok(U64::default()));

        eth_rpc_mock
    }

    async fn spawn_committer_and_run_until_timeout(
        rx: Receiver<FuelBlock>,
        eth_rpc_mock: MockEthereumAdapter,
        storage: impl Storage + 'static,
    ) {
        let _ = tokio::time::timeout(Duration::from_millis(250), async move {
            let block_committer = BlockCommitter::new(rx, eth_rpc_mock, storage);
            block_committer
                .run()
                .await
                .expect("Errors are handled inside of run");
        })
        .await;
    }
}
