use anyhow::Result;
use nub_script::{Commit, FileMove, NubScript, Response};
use std::ops::ControlFlow;
use test_log::test;

#[test(tokio::test)]
async fn test_file_operations() -> Result<()> {
    const SCRIPT: &str = include_str!("file_operations_test.lua");

    let mut nub = NubScript::default();
    let response: Response = nub
        .eval(SCRIPT, || async { ControlFlow::Continue(()) })
        .await?;

    let Response::Change { commits } = response else {
        panic!("Expected a Change response, got: {:?}", response);
    };
    assert_eq!(commits.len(), 2);

    let expected_commit1 = Commit {
        namespace: "repo1".to_string(),
        message: Some("Delete and move files in repo1".to_string()),
        files_created: vec![],
        files_deleted: vec!["file_to_delete.txt".to_string()],
        files_moved: vec![FileMove {
            source: "old_name.txt".to_string(),
            destination: "new_name.txt".to_string(),
        }],
    };
    assert_eq!(commits[0], expected_commit1);

    let expected_commit2 = Commit {
        namespace: "repo2".to_string(),
        message: Some("Delete a file in repo2".to_string()),
        files_created: vec![],
        files_deleted: vec!["another_file.txt".to_string()],
        files_moved: vec![],
    };
    assert_eq!(commits[1], expected_commit2);

    Ok(())
}
