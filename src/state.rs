use chrono::{DateTime, Duration, Utc};
use ethers::types::Transaction;

use crate::{
    analyze::{analyze, Analysis},
    cli::Config,
    head_history::HeadHistory,
    nonce_cache::NonceCache,
    pool::Pool,
    types::{BeaconBlock, NodeKey, TxHash, TxpoolContent},
    watch::{Event, NodeConfig},
};

const PRUNE_DELAY_SECONDS: i64 = 16 * 12;

pub struct State {
    pool: Pool,
    head_history: HeadHistory,
    nonce_cache: NonceCache,

    analysis_queue: Vec<BeaconBlock<Transaction>>,

    quorum: usize,
    propagation_time: chrono::Duration,
}

impl State {
    pub fn new(config: &Config) -> Self {
        let pool = Pool::new();
        let head_history = HeadHistory::new();

        let node_config = NodeConfig::from(config);
        let nonce_cache_provider = node_config.execution_http_provider();
        let nonce_cache = NonceCache::new(nonce_cache_provider, config.nonce_cache_size);

        State {
            pool,
            head_history,
            nonce_cache,

            analysis_queue: Vec::new(),

            quorum: node_config.execution_ws_urls.len(),
            propagation_time: chrono::Duration::seconds(config.propagation_time),
        }
    }

    pub async fn process_event(&mut self, event: Event) -> Vec<Analysis> {
        match event {
            Event::NewTransaction {
                node,
                hash,
                timestamp,
            } => {
                self.process_new_transaction_event(node, hash, timestamp)
                    .await
            }
            Event::NewHead {
                beacon_block,
                timestamp,
            } => self.process_new_head_event(beacon_block, timestamp).await,
            Event::TxpoolContent {
                node,
                content,
                timestamp,
            } => {
                self.process_txpool_content_event(node, content, timestamp)
                    .await
            }
        }
    }

    async fn process_new_transaction_event(
        &mut self,
        node: NodeKey,
        hash: TxHash,
        t: DateTime<Utc>,
    ) -> Vec<Analysis> {
        self.pool.observe_transaction(node, t, hash);
        Vec::new()
    }

    async fn process_txpool_content_event(
        &mut self,
        node: NodeKey,
        content: TxpoolContent,
        t: DateTime<Utc>,
    ) -> Vec<Analysis> {
        self.pool.observe_pool(node, t, content);
        self.pool.prune(t - Duration::seconds(PRUNE_DELAY_SECONDS));

        let beacon_blocks = self.analysis_queue.clone();
        self.analysis_queue.clear();

        let mut analyses = Vec::new();
        for beacon_block in beacon_blocks {
            let analysis = self.analyse_beacon_block(&beacon_block).await;
            if let Some(analysis) = analysis {
                analyses.push(analysis);
            }
        }
        analyses
    }

    async fn process_new_head_event(
        &mut self,
        beacon_block: BeaconBlock<Transaction>,
        t: DateTime<Utc>,
    ) -> Vec<Analysis> {
        self.head_history.observe(t, beacon_block.clone());
        self.head_history
            .prune(t - Duration::seconds(PRUNE_DELAY_SECONDS));
        self.analysis_queue.push(beacon_block);
        Vec::new()
    }

    async fn analyse_beacon_block(
        &mut self,
        beacon_block: &BeaconBlock<Transaction>,
    ) -> Option<Analysis> {
        self.nonce_cache.apply_block(beacon_block.clone());

        let proposal_time = beacon_block.proposal_time();
        let head_obs = self.head_history.at(proposal_time);
        match head_obs {
            None => {
                log::info!(
                    "skipping analysis of {} as head block at proposal time {} is unknown",
                    beacon_block,
                    proposal_time,
                );
                return None;
            }
            Some(head_obs) => {
                if head_obs.head.root != beacon_block.parent_root {
                    log::info!(
                        "skipping analysis of {} due to head mismatch at proposal time (parent: \
                         {}, head at proposal time: {})",
                        beacon_block,
                        beacon_block.parent_root,
                        head_obs.head,
                    );
                    return None;
                }
            }
        }

        let analysis = analyze(
            beacon_block,
            &self.pool,
            &mut self.nonce_cache,
            self.quorum,
            self.propagation_time,
        )
        .await;
        match analysis {
            Ok(a) => Some(a),
            Err(e) => {
                log::error!("error analyzing block: {}", e);
                None
            }
        }
    }
}
