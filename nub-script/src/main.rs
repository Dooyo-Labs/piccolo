use std::ops::ControlFlow;

use nub_script::{NubScript, Response};

#[tokio::main]
async fn main() -> Result<(), nub_script::Error> {
    env_logger::init();

    // Test that NubScript is Send + Sync
    let response = tokio::spawn(async {
        const INPUT_BLOCK: &str = include_str!("../tests/text_manipulation_test.py");
        const SCRIPT: &str = include_str!("../tests/text_manipulation_test.lua");

        let mut nub = NubScript::default();
        nub.add_data_block_content("repo.namespace", "script.py", INPUT_BLOCK);
        nub.eval(SCRIPT, || async {
            tokio::task::yield_now().await;
            ControlFlow::Continue(())
        })
        .await
    })
    .await
    .unwrap();

    let response = match response {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error during evaluation: {:#}", e);
            return Err(e.into());
        }
    };

    let Response::Change { commits } = response else {
        panic!("Expected a Change response, got: {:?}", response);
    };
    assert_eq!(commits.len(), 1);
    let commit = &commits[0];
    assert_eq!(commit.namespace, "repo.namespace");
    assert_eq!(commit.message, Some("Apply text manipulations".to_string()));
    assert_eq!(commit.files_created.len(), 1);
    let change = &commit.files_created[0];
    assert_eq!(change.filename, "script.py");
    println!("Final content:\n{}", change.content);

    Ok(())
}
