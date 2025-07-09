use std::{
    ops::ControlFlow,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Result;
use nub_script::{Error, NubScript};
use test_log::test;
use tokio::{sync::watch, time};

#[test(tokio::test)]
async fn test_cancellation() -> Result<()> {
    const SCRIPT: &str = "while true do end";

    let mut nub = NubScript::default();

    let (cancel_tx, cancel_rx) = watch::channel(false);
    let counter = Arc::new(AtomicU32::new(0));

    let counter_clone = counter.clone();
    let timer_task = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(10));
        for i in 0..5 {
            interval.tick().await;
            assert_eq!(counter_clone.fetch_add(1, Ordering::SeqCst), i);
        }
        cancel_tx.send(true).unwrap();
    });

    let result = nub
        .eval(SCRIPT, || async {
            let mut cancel_rx = cancel_rx.clone();
            tokio::select! {
                biased;
                res = cancel_rx.changed() => {
                    if res.is_err() || *cancel_rx.borrow() {
                        return ControlFlow::Break(());
                    }
                },
                _ = tokio::task::yield_now() => {},
            }
            ControlFlow::Continue(())
        })
        .await;

    timer_task.await?;

    assert_eq!(counter.load(Ordering::SeqCst), 5);
    assert!(matches!(result, Err(Error::Aborted)));

    Ok(())
}
