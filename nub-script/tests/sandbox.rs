use anyhow::Result;
use nub_script::{NubScript, Response};
use std::ops::ControlFlow;
use test_log::test;

#[test(tokio::test)]
async fn test_sandbox_no_io() -> Result<()> {
    const SCRIPT: &str = include_str!("sandbox_test.lua");

    let mut nub = NubScript::default();
    let response: Response = nub
        .eval(SCRIPT, || async { ControlFlow::Continue(()) })
        .await?;

    let Response::Change { commits } = response else {
        panic!("Expected a Change response, got: {:?}", response);
    };
    assert_eq!(commits.len(), 1);
    let commit = &commits[0];
    assert_eq!(commit.files_created.len(), 1);
    let change = &commit.files_created[0];
    assert_eq!(change.content, "success");

    Ok(())
}
