use std::collections::HashMap;
use std::ops::ControlFlow;
use std::{cell::RefCell, sync::Mutex};

use anyhow::anyhow;
use futures_util::Future;
use gc_arena::{Collect, Rootable};
use log::debug;
use piccolo::{
    thread::BadThreadMode, Callback, CallbackReturn, ExternError, Fuel, Lua, RuntimeError,
    Singleton, String as PiccoloString, Table, UserData, Value,
};
use thiserror::Error;

/// Various errors that can occur within the Nub callbacks that will become Piccolo extern errors.
#[derive(Debug, Clone, Error)]
pub enum ScriptError {
    #[error("namespace '{0}' not found")]
    NamespaceNotFound(String),
    #[error("data block '{1}' not found in namespace '{0}'")]
    DataBlockNotFound(String, String),
    #[error("bad string from lua: {0}")]
    BadString(String),
    #[error("pattern not found:\n-- Before Context (found={before_found}) --\n{before_context}\n-- After Context --\n{after_context}\n")]
    PatternNotFound {
        before_context: String,
        select_context: String,
        after_context: String,
        before_found: bool,
        select_found: bool,
    },
    #[error("Inconsistent patch state: {0}")]
    BadPatch(String),
}

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("Script Evaluation Failed: {message}")]
    Script {
        message: String,
        source: ExternError,
    },
    #[error("Script Evaluation aborted")]
    Aborted,
    #[error("Invalid Script Pesponse: {0}")]
    InvalidResponse(String),
    #[error("Script Runtime Failure: {0}")]
    BadThreadMode(#[from] BadThreadMode),
}

#[derive(Collect)]
#[collect(no_drop)]
struct FilesTable<'gc>(Table<'gc>);

impl<'gc> Singleton<'gc> for FilesTable<'gc> {
    fn create(ctx: piccolo::Context<'gc>) -> Self {
        Self(Table::new(&ctx))
    }
}

/// NubScript is a Lua SCRIPT block execution environment for the Nubby coding agent
/// that allows Lua scripts to interact with a set of data blocks and output a
/// response.
///
/// # Example
///
/// ```no_run
/// use nub_script::{NubScript, Response};
/// use std::ops::ControlFlow;
/// #[tokio::main]
/// async fn main() {
///     let mut nub = NubScript::default();
///     nub.add_data_block_content("repo.namespace", "script.py", "print('Hello, world!')");
///     let script = r#"
///         local response = Response.new("CHANGE")
///         local commit = Commit.new("repo.namespace")
///         commit:write_file("script.py", get_data_block("repo.namespace", "script.py"))
///         commit:set_message("Apply text manipulations")
///         response:add_commit(commit)
///         return response
///     "#;
///     let response: Response = nub.eval(script, || async {
///         ControlFlow::Continue(())
///     }).await.unwrap();
///     let Response::Change { commits } = response else {
///        panic!("Expected a Change response, got: {:?}", response);
///     };
///     assert_eq!(commits.len(), 1);
///     let commit = &commits[0];
///     assert_eq!(commit.namespace, "repo.namespace");
///     assert_eq!(commit.message, Some("Apply text manipulations".to_string()));
///     assert_eq!(commit.files_created.len(), 1);
///     let change = &commit.files_created[0];
///     assert_eq!(change.filename, "script.py");
///     assert_eq!(change.content, "print('Hello, world!')");
/// }
/// ```
pub struct NubScript {
    files: HashMap<String, HashMap<String, String>>,
}

fn load_nub_script<'gc>(ctx: piccolo::Context<'gc>) {
    // Commit API
    let commit_methods = Table::new(&ctx);
    commit_methods.set_field(
        ctx,
        "write_file",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let (commit_ud, filename, content, opt): (
                UserData,
                PiccoloString,
                PiccoloString,
                Option<Table>,
            ) = stack.consume(ctx)?;

            let mode = if let Some(opt) = opt {
                opt.get::<_, Option<PiccoloString>>(ctx, "mode")?
                    .map(|s| s.to_str().map(|s| s.to_string()))
                    .transpose()
                    .map_err(|e| ScriptError::BadString(e.to_string()))?
            } else {
                None
            }
            .unwrap_or_else(|| "0644".to_string());

            let mut commit = commit_ud
                .downcast_static::<RefCell<GcCommit>>()?
                .borrow_mut();
            commit.files_created.push(File {
                filename: filename
                    .to_str()
                    .map_err(|e| ScriptError::BadString(e.to_string()))?
                    .to_string(),
                content: content
                    .to_str()
                    .map_err(|e| ScriptError::BadString(e.to_string()))?
                    .to_string(),
                mode,
            });
            Ok(CallbackReturn::Return)
        }),
    );

    commit_methods.set_field(
        ctx,
        "delete_file",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let (commit_ud, filename): (UserData, PiccoloString) = stack.consume(ctx)?;
            let mut commit = commit_ud
                .downcast_static::<RefCell<GcCommit>>()?
                .borrow_mut();
            commit.files_deleted.push(
                filename
                    .to_str()
                    .map_err(|e| ScriptError::BadString(e.to_string()))?
                    .to_string(),
            );
            Ok(CallbackReturn::Return)
        }),
    );

    commit_methods.set_field(
        ctx,
        "move_file",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let (commit_ud, source, destination): (UserData, PiccoloString, PiccoloString) =
                stack.consume(ctx)?;
            let mut commit = commit_ud
                .downcast_static::<RefCell<GcCommit>>()?
                .borrow_mut();
            commit.files_moved.push(FileMove {
                source: source
                    .to_str()
                    .map_err(|e| ScriptError::BadString(e.to_string()))?
                    .to_string(),
                destination: destination
                    .to_str()
                    .map_err(|e| ScriptError::BadString(e.to_string()))?
                    .to_string(),
            });
            Ok(CallbackReturn::Return)
        }),
    );

    commit_methods.set_field(
        ctx,
        "set_message",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let (commit_ud, summary): (UserData, PiccoloString) = stack.consume(ctx)?;
            let mut commit = commit_ud
                .downcast_static::<RefCell<GcCommit>>()?
                .borrow_mut();
            commit.message = Some(
                summary
                    .to_str()
                    .map_err(|e| ScriptError::BadString(e.to_string()))?
                    .to_string(),
            );
            Ok(CallbackReturn::Return)
        }),
    );

    let commit_metatable = Table::new(&ctx);
    commit_metatable.set_field(ctx, "__index", commit_methods);

    let commit_table = Table::new(&ctx);
    commit_table.set_field(
        ctx,
        "new",
        Callback::from_fn_with(&ctx, commit_metatable, |metatable, ctx, _, mut stack| {
            let namespace: PiccoloString = stack.consume(ctx)?;
            let commit = UserData::new_static(
                &ctx,
                RefCell::new(GcCommit::new(
                    namespace
                        .to_str()
                        .map_err(|e| ScriptError::BadString(e.to_string()))?,
                )),
            );
            commit.set_metatable(&ctx, Some(*metatable));
            stack.replace(ctx, commit);
            Ok(CallbackReturn::Return)
        }),
    );
    ctx.globals().set_field(ctx, "Commit", commit_table);

    // Response API
    let response_methods = Table::new(&ctx);
    response_methods.set_field(
        ctx,
        "add_commit",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let (response_ud, commit_ud): (UserData, UserData) = stack.consume(ctx)?;

            let commit = commit_ud
                .downcast_static::<RefCell<GcCommit>>()?
                .borrow()
                .clone();

            let mut response = response_ud
                .downcast_static::<RefCell<GcResponse>>()?
                .borrow_mut();
            response.commits.push(commit);
            Ok(CallbackReturn::Return)
        }),
    );
    response_methods.set_field(
        ctx,
        "ack",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let (response_ud, notification): (UserData, Option<PiccoloString>) =
                stack.consume(ctx)?;
            let mut response = response_ud
                .downcast_static::<RefCell<GcResponse>>()?
                .borrow_mut();
            response.notification = notification
                .map(|s| s.to_str())
                .transpose()
                .map_err(|e| ScriptError::BadString(e.to_string()))?
                .map(|s| s.to_string());

            Ok(CallbackReturn::Return)
        }),
    );

    let response_metatable = Table::new(&ctx);
    response_metatable.set_field(ctx, "__index", response_methods);

    let response_table = Table::new(&ctx);
    response_table.set_field(
        ctx,
        "new",
        Callback::from_fn_with(&ctx, response_metatable, |metatable, ctx, _, mut stack| {
            let category: PiccoloString = stack.consume(ctx)?;
            let response = UserData::new_static(
                &ctx,
                RefCell::new(GcResponse::new(
                    category
                        .to_str()
                        .map_err(|e| ScriptError::BadString(e.to_string()))?,
                )),
            );
            response.set_metatable(&ctx, Some(*metatable));
            stack.replace(ctx, response);
            Ok(CallbackReturn::Return)
        }),
    );
    ctx.globals().set_field(ctx, "Response", response_table);

    // TextPatcher API
    let patcher_methods = Table::new(&ctx);
    patcher_methods.set_field(
        ctx,
        "select_next_lines",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let (patcher_ud, lines): (UserData, Table) = stack.consume(ctx)?;
            let mut patcher = patcher_ud
                .downcast_static::<RefCell<TextPatcher>>()?
                .borrow_mut();
            let mut before = vec![];
            let mut select = vec![];
            let mut after = vec![];
            for (_k, line) in lines {
                let Value::Table(ref line) = line else {
                    return Err(ScriptError::BadString(
                        "Lines table must contain tables with 'B', 'S', or 'A' keys".to_string(),
                    )
                    .into());
                };

                let code = match line.get_value(ctx, 1).into_string(ctx) {
                    Some(s) => s.to_str().map_err(|_e| {
                        ScriptError::BadString("Invalid key in lines table".to_string())
                    })?,
                    None => {
                        return Err(ScriptError::BadString(
                            "Key in lines table must be a string".to_string(),
                        )
                        .into());
                    }
                };
                let value = match line.get_value(ctx, 2).into_string(ctx) {
                    Some(s) => s.to_str().map_err(|_e| {
                        ScriptError::BadString("Invalid value in lines table".to_string())
                    })?,
                    None => {
                        return Err(ScriptError::BadString(
                            "Value in lines table must be a string".to_string(),
                        )
                        .into());
                    }
                };
                match code {
                    "B" => before.push(value),
                    "S" => select.push(value),
                    "A" => after.push(value),
                    _ => {
                        return Err(ScriptError::BadString(format!(
                            "Unknown key in lines table: {code}"
                        ))
                        .into());
                    }
                }
            }
            patcher.move_to_context(&before, &select, &after, true)?;
            stack.replace(ctx, patcher_ud);
            Ok(CallbackReturn::Return)
        }),
    );
    patcher_methods.set_field(
        ctx,
        "move_forward_to_context",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let (patcher_ud, before, after): (
                UserData,
                Option<PiccoloString>,
                Option<PiccoloString>,
            ) = stack.consume(ctx)?;
            let mut patcher = patcher_ud
                .downcast_static::<RefCell<TextPatcher>>()?
                .borrow_mut();
            let before = before
                .map(|s| {
                    s.to_str()
                        .map_err(|e| ScriptError::BadString(e.to_string()))
                })
                .transpose()?
                .unwrap_or_default();
            let after = after
                .map(|s| {
                    s.to_str()
                        .map_err(|e| ScriptError::BadString(e.to_string()))
                })
                .transpose()?
                .unwrap_or_default();

            let before_lines: Vec<_> = if before == "" {
                vec![]
            } else {
                before.lines().collect()
            };
            let after_lines: Vec<_> = if after == "" {
                vec![]
            } else {
                after.lines().collect()
            };
            patcher.move_to_context(&before_lines, &[], &after_lines, false)?;
            stack.replace(ctx, patcher_ud);
            Ok(CallbackReturn::Return)
        }),
    );
    patcher_methods.set_field(
        ctx,
        "move_empty",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let patcher_ud: UserData = stack.consume(ctx)?;
            let mut patcher = patcher_ud
                .downcast_static::<RefCell<TextPatcher>>()?
                .borrow_mut();
            patcher.move_empty()?;
            stack.replace(ctx, patcher_ud);
            Ok(CallbackReturn::Return)
        }),
    );
    patcher_methods.set_field(
        ctx,
        "start_selection_empty",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let patcher_ud: UserData = stack.consume(ctx)?;
            let mut patcher = patcher_ud
                .downcast_static::<RefCell<TextPatcher>>()?
                .borrow_mut();
            patcher.start_selection_empty()?;
            stack.replace(ctx, patcher_ud);
            Ok(CallbackReturn::Return)
        }),
    );
    patcher_methods.set_field(
        ctx,
        "end_selection",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let patcher_ud: UserData = stack.consume(ctx)?;
            let mut patcher = patcher_ud
                .downcast_static::<RefCell<TextPatcher>>()?
                .borrow_mut();
            patcher.end_selection()?;
            stack.replace(ctx, patcher_ud);
            Ok(CallbackReturn::Return)
        }),
    );
    patcher_methods.set_field(
        ctx,
        "select_end",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let patcher_ud: UserData = stack.consume(ctx)?;
            let mut patcher = patcher_ud
                .downcast_static::<RefCell<TextPatcher>>()?
                .borrow_mut();
            patcher.select_end()?;
            stack.replace(ctx, patcher_ud);
            Ok(CallbackReturn::Return)
        }),
    );
    patcher_methods.set_field(
        ctx,
        "replace_selected",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let (patcher_ud, replacement): (UserData, Option<PiccoloString>) =
                stack.consume(ctx)?;
            let mut patcher = patcher_ud
                .downcast_static::<RefCell<TextPatcher>>()?
                .borrow_mut();
            let replacement = replacement
                .map(|s| {
                    s.to_str()
                        .map_err(|e| ScriptError::BadString(e.to_string()))
                })
                .transpose()?
                .unwrap_or_default();
            patcher.replace_selected(replacement)?;
            stack.replace(ctx, patcher_ud);
            Ok(CallbackReturn::Return)
        }),
    );
    patcher_methods.set_field(
        ctx,
        "delete_selected",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let patcher_ud: UserData = stack.consume(ctx)?;
            let mut patcher = patcher_ud
                .downcast_static::<RefCell<TextPatcher>>()?
                .borrow_mut();
            patcher.replace_selected("")?;
            stack.replace(ctx, patcher_ud);
            Ok(CallbackReturn::Return)
        }),
    );
    patcher_methods.set_field(
        ctx,
        "apply",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let patcher_ud: UserData = stack.consume(ctx)?;
            let patcher = patcher_ud
                .downcast_static::<RefCell<TextPatcher>>()?
                .borrow();
            let new_content = patcher.apply()?;
            stack.replace(ctx, ctx.intern(new_content.as_bytes()));
            Ok(CallbackReturn::Return)
        }),
    );

    let patcher_metatable = Table::new(&ctx);
    patcher_metatable.set_field(ctx, "__index", patcher_methods);

    let patcher_table = Table::new(&ctx);
    patcher_table.set_field(
        ctx,
        "new",
        Callback::from_fn_with(&ctx, patcher_metatable, |metatable, ctx, _, mut stack| {
            let content: PiccoloString = stack.consume(ctx)?;
            let patcher = UserData::new_static(
                &ctx,
                RefCell::new(TextPatcher::new(
                    content
                        .to_str()
                        .map_err(|e| ScriptError::BadString(e.to_string()))?,
                )),
            );
            patcher.set_metatable(&ctx, Some(*metatable));
            stack.replace(ctx, patcher);
            Ok(CallbackReturn::Return)
        }),
    );
    ctx.globals().set_field(ctx, "TextPatcher", patcher_table);

    let get_data_block = Callback::from_fn(&ctx, |ctx, _, mut stack| {
        let files_table = ctx.singleton::<Rootable![FilesTable<'_>]>().0;
        let (namespace, filename): (PiccoloString, PiccoloString) = stack.consume(ctx)?;

        let content = if let Ok(ns_table) = files_table.get::<_, Table>(ctx, namespace) {
            match ns_table.get::<_, Value>(ctx, filename).unwrap_or_default() {
                Value::String(s) => s,
                _ => {
                    return Err(ScriptError::DataBlockNotFound(
                        namespace
                            .to_str()
                            .map_err(|e| ScriptError::BadString(e.to_string()))?
                            .to_string(),
                        filename
                            .to_str()
                            .map_err(|e| ScriptError::BadString(e.to_string()))?
                            .to_string(),
                    )
                    .into());
                }
            }
        } else {
            return Err(ScriptError::NamespaceNotFound(
                namespace
                    .to_str()
                    .map_err(|e| ScriptError::BadString(e.to_string()))?
                    .to_string(),
            )
            .into());
        };

        stack.replace(ctx, content);
        Ok(CallbackReturn::Return)
    });
    ctx.globals()
        .set_field(ctx, "get_data_block", get_data_block);

    let error_callback = Callback::from_fn(&ctx, |_, _, stack| {
        let value = stack.get(0);
        if let Value::String(s) = value {
            Err(RuntimeError::new(anyhow!(s.display_lossy().to_string())).into())
        } else {
            Err(RuntimeError::new(anyhow!(value.display().to_string())).into())
        }
    });
    ctx.globals().set(ctx, "error", error_callback).unwrap();
}

impl Default for NubScript {
    fn default() -> Self {
        NubScript {
            //lua: Mutex::new(lua),
            files: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct Patch {
    start: usize,
    end: usize,
    replacement: Vec<String>,
}

#[derive(Debug, Clone)]
struct TextPatcher {
    original_lines: Vec<String>,
    patches: Vec<Patch>,
    selection: (usize, usize),
    cursor: usize,
}

impl TextPatcher {
    fn new(content: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in content.as_bytes().iter().enumerate() {
            if *b == b'\n' {
                line_starts.push(i + 1);
            }
        }

        Self {
            original_lines: content.lines().map(String::from).collect(),
            patches: Vec::new(),
            selection: (0, 0),
            cursor: 0,
        }
    }

    /// "Move" the cursor to position of the current cursor, effectively clearing the selection.
    ///
    /// This is useful when you want to insert new content at the end of the current selection, without replacing any existing content.
    fn move_empty(&mut self) -> Result<(), ScriptError> {
        self.selection = (self.selection.1, self.selection.1);
        Ok(())
    }

    fn move_to_context(
        &mut self,
        before_lines: &[&str],
        select_lines: &[&str],
        after_lines: &[&str],
        with_selection: bool,
    ) -> Result<(), ScriptError> {
        log::trace!(
            "Looking for context, before: {:?}, after: {:?}",
            before_lines,
            after_lines
        );
        let pattern_len = before_lines.len() + after_lines.len();
        if pattern_len == 0 {
            return Ok(());
        }

        // Allow the before_context to be found before the last match end
        let search_start_line = self.selection.1.saturating_sub(before_lines.len());

        if self.original_lines.len() < pattern_len + search_start_line {
            return Err(ScriptError::PatternNotFound {
                before_context: before_lines.join("\n"),
                select_context: select_lines.join("\n"),
                after_context: after_lines.join("\n"),
                before_found: false,
                select_found: false,
            });
        }

        let mut before_found = false;
        let mut select_found = false;

        let search_lines = &self.original_lines[search_start_line..];
        for (i, window) in search_lines
            .windows(before_lines.len() + select_lines.len() + after_lines.len())
            .enumerate()
        {
            //log::trace!("checking window {i}: {window:?}");
            if window[..before_lines.len()] == *before_lines {
                before_found = true;
            } else {
                continue;
            }
            if window[before_lines.len()..before_lines.len() + select_lines.len()] == *select_lines
            {
                select_found = true;
            } else {
                continue;
            }
            if window[before_lines.len() + select_lines.len()..] != *after_lines {
                continue;
            }

            let sel_start = search_start_line + i + before_lines.len();
            let sel_end = search_start_line + i + before_lines.len() + select_lines.len();
            self.cursor = sel_end;
            log::trace!("Updated cursor to {sel_end}");
            if with_selection {
                self.selection = (sel_start, sel_end);
                log::trace!("Updated selection: {:?}", self.selection);
            }

            return Ok(());
        }

        Err(ScriptError::PatternNotFound {
            before_context: before_lines.join("\n"),
            select_context: select_lines.join("\n"),
            after_context: after_lines.join("\n"),
            before_found,
            select_found,
        })
    }

    fn start_selection_empty(&mut self) -> Result<(), ScriptError> {
        self.selection = (self.cursor, self.cursor);
        Ok(())
    }

    fn end_selection(&mut self) -> Result<(), ScriptError> {
        self.selection = (self.selection.0, self.cursor);
        Ok(())
    }

    fn select_end(&mut self) -> Result<(), ScriptError> {
        let n_lines = self.original_lines.len();
        self.selection = (n_lines, n_lines);
        self.cursor = n_lines;
        Ok(())
    }

    fn replace_selected(&mut self, replacement: &str) -> Result<(), ScriptError> {
        self.patches.push(Patch {
            start: self.selection.0,
            end: self.selection.1,
            replacement: replacement.lines().map(String::from).collect(),
        });
        Ok(())
    }

    fn apply(&self) -> Result<String, ScriptError> {
        let mut result = String::new();

        let mut skip = 0;
        let mut patches = self.patches.iter().peekable();

        // TODO: Implement a fuzzy matching algorithm to try any automatically handle the most common
        // LLM mistakes.
        for (cursor, line) in self.original_lines.iter().enumerate() {
            if let Some(patch) = patches.peek() {
                if cursor == patch.start {
                    for replacement_line in &patch.replacement {
                        result.push_str(replacement_line);
                        result.push('\n');
                    }
                    if patch.end > patch.start {
                        skip = patch.end - patch.start; // Skip the lines that are replaced
                    }
                    patches.next();
                }
            }
            if skip > 0 {
                skip -= 1;
                continue;
            }
            result.push_str(line);
            result.push('\n');
        }

        if let Some(last) = patches.next() {
            if last.start == self.original_lines.len() {
                // If the last patch starts at the end, we append it
                for replacement_line in &last.replacement {
                    result.push_str(replacement_line);
                    result.push('\n');
                }
            } else {
                return Err(ScriptError::BadPatch(
                    "Last patch does not start at the end of the original content".to_string(),
                ));
            }
        }
        if !patches.peek().is_none() {
            return Err(ScriptError::BadPatch(
                "There are unprocessed patches left".to_string(),
            ));
        }

        Ok(result)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub filename: String,
    pub mode: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMove {
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Commit {
    pub namespace: String,
    pub message: Option<String>,
    pub files_created: Vec<File>,
    pub files_deleted: Vec<String>,
    pub files_moved: Vec<FileMove>,
}

#[derive(Debug, Clone, Collect)]
#[collect(require_static)]
struct GcCommit {
    namespace: String,
    message: Option<String>,
    files_created: Vec<File>,
    files_deleted: Vec<String>,
    files_moved: Vec<FileMove>,
}

impl GcCommit {
    fn new(namespace: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            message: None,
            files_created: vec![],
            files_deleted: vec![],
            files_moved: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub enum Response {
    Change { commits: Vec<Commit> },
    Notify { message: String },
}

#[derive(Debug, Default, Collect)]
#[collect(no_drop)]
struct GcResponse {
    pub category: String,
    pub notification: Option<String>,
    pub commits: Vec<GcCommit>,
}

impl GcResponse {
    pub fn new(category: &str) -> Self {
        Self {
            category: category.to_string(),
            notification: None,
            commits: vec![],
        }
    }
}

impl NubScript {
    /// Register the content of a data block under the given namespace and filename.
    ///
    /// This content can be accessed later using the `get_data_block` function in Lua scripts.
    pub fn add_data_block_content(&mut self, namespace: &str, name: &str, content: &str) {
        self.files
            .entry(namespace.to_string())
            .or_default()
            .insert(name.to_string(), content.to_string());
    }

    /// Evaluate the given Lua SCRIPT block with the registered data blocks, and return the script's response.
    pub async fn eval<'a, F, Fut>(
        &'a mut self,
        script: &'a str,
        mut on_yield: F,
    ) -> Result<Response, Error>
    where
        F: FnMut() -> Fut + 'a,
        Fut: Future<Output = ControlFlow<()>> + 'a,
    {
        fn error_message(err: &ExternError, script: &str) -> String {
            let mut msg = String::new();
            msg.push_str("Script:\n");
            let lines = script
                .lines()
                .enumerate()
                .map(|(i, line)| format!("{:>4}: {}", i + 1, line));
            msg.push_str(&lines.collect::<Vec<_>>().join("\n"));
            msg.push_str("\nError:\n");
            let mut source_map = HashMap::new();
            source_map.insert("<nub-script>".to_string(), script.to_string());
            let _ = err.pretty_print(&mut msg, Some(&source_map));
            msg
        }

        let vm = {
            struct VMState {
                lua: Lua,
                executor: piccolo::StashedExecutor,
            }
            struct VM {
                state: Box<Mutex<VMState>>,
            }
            /// Safety: The Lua VM doesn't depend on any global or TLS state,
            /// and so it should be safe to move between threads if we ensure
            /// that the Lua context is not used concurrently.
            unsafe impl Send for VM {}

            let mut lua = Lua::full();
            lua.enter(|ctx| {
                load_nub_script(ctx);
            });

            let files = self.files.clone();
            let executor = match lua.try_enter(move |ctx| {
                let files_table = ctx.singleton::<Rootable![FilesTable<'_>]>().0;
                for (k, _) in files_table.iter() {
                    files_table.set(ctx, k, Value::Nil)?;
                }

                for (namespace, files) in files {
                    let ns_table = Table::new(&ctx);
                    for (filename, content) in files {
                        ns_table.set(ctx, filename, content)?;
                    }
                    files_table.set(ctx, namespace, ns_table)?;
                }

                let closure = piccolo::Closure::load(ctx, Some("<nub-script>"), script.as_bytes())?;
                let executor = piccolo::Executor::start(ctx, closure.into(), ());
                Ok(ctx.stash(executor))
            }) {
                Ok(executor) => executor,
                Err(e) => {
                    let msg = error_message(&e, script);
                    return Err(Error::Script {
                        message: format!("Failed to create executor: {msg}"),
                        source: e.into(),
                    });
                }
            };

            VM {
                state: Box::new(Mutex::new(VMState { lua, executor })),
            }
        };

        const FUEL_PER_GC: i32 = 4096;
        loop {
            {
                let mut vm = vm.state.lock().unwrap();
                let executor = vm.executor.clone();

                let mut fuel = Fuel::with(FUEL_PER_GC);
                let res = vm.lua.enter(|ctx| {
                    let exec = ctx.fetch(&executor);
                    exec.step(ctx, &mut fuel)
                });

                if let Ok(finished) = res {
                    if finished {
                        break;
                    }
                } else {
                    return Err(res.unwrap_err().into());
                }
            }

            if on_yield().await.is_break() {
                return Err(Error::Aborted);
            }
        }

        let mut vm = vm.state.lock().unwrap();
        let executor = vm.executor.clone();
        let stashed_response = match vm.lua.try_enter(|ctx| {
            let ud = ctx.fetch(&executor).take_result::<UserData>(ctx)??;
            Ok(ctx.stash(ud))
        }) {
            Ok(stashed_response) => stashed_response,
            Err(e) => {
                let msg = error_message(&e, script);
                return Err(Error::Script {
                    message: format!("Failed to get response: {msg}"),
                    source: e.into(),
                });
            }
        };

        let final_response = vm.lua.enter(|ctx| {
            let ud = ctx.fetch(&stashed_response);
            let response = ud
                .downcast_static::<RefCell<GcResponse>>()
                .unwrap()
                .borrow();

            let commits = response
                .commits
                .iter()
                .map(|gc_commit| {
                    debug!("commit for ns `{}`", gc_commit.namespace);
                    for file in &gc_commit.files_created {
                        debug!("file `{}` with mode `{}` created", file.filename, file.mode);
                    }
                    Commit {
                        namespace: gc_commit.namespace.clone(),
                        message: gc_commit.message.clone(),
                        files_created: gc_commit.files_created.clone(),
                        files_deleted: gc_commit.files_deleted.clone(),
                        files_moved: gc_commit.files_moved.clone(),
                    }
                })
                .collect();

            Ok(match response.category.as_str() {
                "CHANGE" => Response::Change { commits },
                "NOTIFY" => Response::Notify {
                    message: response
                        .notification
                        .clone()
                        .unwrap_or_else(|| "No message".to_string()),
                },
                _ => {
                    return Err(Error::InvalidResponse(format!(
                        "Unknown response category: {}",
                        response.category
                    )))
                }
            })
        });

        final_response
    }
}
