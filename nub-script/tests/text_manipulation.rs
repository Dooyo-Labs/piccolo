use anyhow::Result;
use nub_script::{Error, NubScript, Response, ScriptError};
use std::ops::ControlFlow;
use test_log::test;

#[test(tokio::test)]
async fn test_text_manipulation() -> Result<()> {
    const INPUT_BLOCK: &str = include_str!("text_manipulation_test.py");
    const SCRIPT: &str = include_str!("text_manipulation_test.lua");

    let mut nub = NubScript::default();
    nub.add_data_block_content("repo.namespace", "script.py", INPUT_BLOCK);

    let response: Response = match nub
        .eval(SCRIPT, || async { ControlFlow::Continue(()) })
        .await
    {
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
    assert_eq!(change.mode, "0644");

    const EXPECTED_OUTPUT: &str = r#"#!/usr/bin/env python3

def say_hello():
    print("Hello, world!")

def main():
    say_hello()

if __name__ == "__main__":
    main()
"#;

    assert_eq!(change.content, EXPECTED_OUTPUT);

    Ok(())
}

#[test(tokio::test)]
async fn test_accurate_patching() -> Result<()> {
    const INPUT_BLOCK: &str = r#"-- line 1
-- line 2
-- line 3
-- line 4
-- line 5
-- line 6
-- line 7
-- line 8
-- line 9
-- remove me
-- remove me
-- line 10
-- remove me
"#;

    const SCRIPT: &str = r#"
local patcher = TextPatcher.new(get_data_block("repo.namespace", "lines.txt"))

patcher:replace_selected([[
-- line 0
]])

patcher:move_forward_to_context(
[[
-- line 4
-- line 5
]],
-- Cursor in front of line 6
[[
-- line 6
-- line 7
]]
)
patcher:start_selection_empty()

patcher:move_forward_to_context(
[[
-- line 5
-- line 6
]],
-- Cursor in front of line 7
[[
-- line 7
-- line 8
]]
)
patcher:end_selection()

patcher:replace_selected([[
-- line 6
-- line 6.5
]])

patcher:move_forward_to_context(
[[
-- line 9
]],
-- Cursor after line 9
[[
-- remove me
]]
)
patcher:start_selection_empty()

patcher:move_forward_to_context(
[[
-- remove me
]],
-- Cursor in front of line 10
[[
-- line 10
]]
)
patcher:end_selection()

-- Delete the 'remove me' lines
patcher:replace_selected("")

patcher:move_forward_to_context(
[[
-- line 10
]],
-- Cursor after line 10
[[
-- remove me
]]
)
patcher:start_selection_empty()

patcher:move_forward_to_context(
[[
-- remove me
]],
-- Cursor at end of file
nil
)
patcher:end_selection()

-- Delete the 'remove me' line at the end of the file
patcher:delete_selected()


local new_content = patcher:apply()

local response = Response.new("CHANGE")
local commit = Commit.new("repo.namespace")
commit:write_file("lines.txt", new_content)
commit:set_message("Apply text manipulations")
response:add_commit(commit)
return response
"#;

    let mut nub = NubScript::default();
    nub.add_data_block_content("repo.namespace", "lines.txt", INPUT_BLOCK);

    let response: Response = match nub
        .eval(SCRIPT, || async { ControlFlow::Continue(()) })
        .await
    {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error during evaluation: {:#}", e);
            return Err(e.into());
        }
    };
    let Response::Change { commits } = response else {
        panic!("Expected a Change response, got: {:?}", response);
    };
    let commit = &commits[0];
    let change = &commit.files_created[0];
    println!("Final content:\n{}", change.content);
    const EXPECTED_OUTPUT: &str = r#"-- line 0
-- line 1
-- line 2
-- line 3
-- line 4
-- line 5
-- line 6
-- line 6.5
-- line 7
-- line 8
-- line 9
-- line 10
"#;

    assert_eq!(change.content, EXPECTED_OUTPUT);

    Ok(())
}

#[test(tokio::test)]
async fn test_pattern_not_found() {
    const SOURCE_RS: &str = "fn main() {}";
    const SCRIPT: &str = r#"
        local patcher = TextPatcher.new(get_data_block("repo", "source.rs"))
        patcher:move_forward_to_context("foo", "baz")
        return Response.new("CHANGE")
    "#;

    let mut nub = NubScript::default();
    nub.add_data_block_content("repo", "source.rs", SOURCE_RS);

    let result = nub
        .eval(SCRIPT, || async { ControlFlow::Continue(()) })
        .await;
    log::trace!("Result: {:?}", result);

    assert!(matches!(&result, Err(Error::Script { .. })));

    if let Err(Error::Script { source, .. }) = result {
        if let piccolo::ExternError::Runtime(re) = source {
            if let Some(nub_err) = re.downcast::<ScriptError>() {
                if let ScriptError::PatternNotFound {
                    before_context,
                    after_context,
                    before_found,
                } = nub_err
                {
                    assert_eq!(before_context, "foo");
                    assert_eq!(after_context, "baz");
                    assert!(!before_found);
                    return;
                }
            }
        }
    }

    panic!("did not find expected error");
}
