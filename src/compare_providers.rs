use std::{
    collections::HashSet,
    io::{stdout, Write},
};

use color_eyre::{eyre::eyre, Report, Result};
use itertools::Itertools;
use tokio::sync::{
    mpsc,
    mpsc::{Receiver, Sender},
};

use crate::{
    cli,
    types::TxHash,
    watch::{watch_transactions, Event, NodeConfig},
};

pub async fn compare_providers(config: &cli::Config) -> Result<()> {
    let node_config = NodeConfig::from(config);

    for (i, url) in node_config.execution_ws_urls.iter().enumerate() {
        println!("{}: {}", i, url);
    }
    println!();

    let n = node_config.execution_ws_urls.len();
    let (tx, mut rx): (Sender<Event>, Receiver<Event>) = mpsc::channel(1000);

    for k in 1..(n + 1) {
        for combination in (0..n).combinations(k) {
            let parts = combination.iter().map(|i| i.to_string()).clone();
            let s: String = itertools::Itertools::intersperse(parts, String::from("|")).collect();
            print!("{:>13}", s);
        }
    }
    println!();

    let watch_handle = tokio::spawn(async move {
        watch_transactions(node_config, tx).await?;
        Err::<(), Report>(eyre!("watch task ended unexpectedly"))
    });

    let process_handle = tokio::spawn(async move {
        process_transactions(&mut rx, n).await?;
        Err::<(), Report>(eyre!("process task ended unexpectedly"))
    });

    tokio::select! {
        r = watch_handle => r,
        r = process_handle => r,
    }??;

    Ok(())
}

async fn process_transactions(rx: &mut Receiver<Event>, n: usize) -> Result<()> {
    let mut seen_by: Vec<HashSet<TxHash>> = Vec::new();
    for _ in 0..n {
        seen_by.push(HashSet::new());
    }
    let mut i = 0;
    while let Some(event) = rx.recv().await {
        match event {
            Event::NewTransaction {
                node,
                hash,
                timestamp: _,
            } => {
                seen_by[node].insert(hash);
            }
            _ => {
                return Err(eyre!("received non-transaction event"));
            }
        }

        i += 1;
        if i % 10 == 0 {
            print!("\r");
            for k in 1..(n + 1) {
                for combination in (0..n).combinations(k) {
                    let mut union = seen_by[combination[0]].clone();
                    let mut intersection = seen_by[combination[0]].clone();
                    for j in &combination[1..] {
                        union.extend(&seen_by[*j]);
                        intersection.retain(|e| seen_by[*j].contains(e));
                    }
                    print!(
                        "{:>8} {:.2}",
                        intersection.len(),
                        (intersection.len() as f64) / (union.len() as f64),
                    );
                }
            }
            stdout().flush().unwrap();
        }
    }

    Ok(())
}
