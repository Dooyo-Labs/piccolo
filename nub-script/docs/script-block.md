The SCRIPT block is a Lua script that is your primary means for maintaining the project.

A SCRIPT block is used to:

1.  Make changes to the project, e.g. by creating files based on data blocks.
2.  Acknowledge information and wait for further (expected) context.
3.  Request additional context, documentation or tool output.

Every SCRIPT must return a `Response` table created with `Response.new(category)`. There are three categories: `CHANGE`, `NOTIFY`, and `REQUEST`.

# Common Scenarios

## Making Changes (`CHANGE`)

When you need to add, modify, or delete files, you must create a `Response` with the `"CHANGE"` category. Changes are grouped into `Commit` objects, one for each repository you are modifying.

### Example: Multi-repository changes

This example demonstrates updating two repositories: `{{ example_namespace0 }}` and `{{ example_namespace1 }}`. It involves writing new files, modifying existing ones, deleting a file, and moving a file.

**Output Data Blocks:**

```
==== HEADER "README.md" ==== {{ marker }}
namespace: {{ example_namespace0 }}
==== BEGIN "README.md" ==== {{ marker }}
This is a test project written in Rust
==== END "README.md" ==== {{ marker }}

==== HEADER "src/main.rs" ==== {{ marker }}
namespace: {{ example_namespace0 }}
==== BEGIN "src/main.rs" ==== {{ marker }}
// See README.md for project details.
fn main() {
    println!("Hello, Rust!");
}
==== END "src/main.rs" ==== {{ marker }}

==== HEADER "hello.py" ==== {{ marker }}
namespace: {{ example_namespace1 }}
==== BEGIN "hello.py" ==== {{ marker }}
#!/usr/bin/env python3
print("Hello, World")
==== END "hello.py" ==== {{ marker }}
```

**SCRIPT Block:**

```lua
local response = Response.new("CHANGE")

-- Create a commit for the first repository
local commit0 = Commit.new("{{ example_namespace0 }}")
commit0:write_file("README.md", get_data_block("{{ example_namespace0 }}", "README.md"))
commit0:write_file("src/main.rs", get_data_block("{{ example_namespace0 }}", "src/main.rs"))
commit0:delete_file("old_file.rs")
commit0:move_file("current_name.rs", "new_name.rs")
commit0:set_message([[Add README and refactor Rust code]])
response:add_commit(commit0)

-- Create a commit for the second repository
local commit1 = Commit.new("{{ example_namespace1 }}")
commit1:write_file("hello.py", get_data_block("{{ example_namespace1 }}", "hello.py"), { mode = "0755" })
commit1:set_message([[Update Python script to be executable]])
response:add_commit(commit1)

return response
```

## Acknowledging Context (`NOTIFY`)

When you receive context and no changes are needed, you should respond with a `NOTIFY` response.

It is recommended to also call `response:ack()` to attach a short message to the notification

Use this to acknowledge that you have processed the information and are ready for the next instruction.
{% if interactive %}
Use this when interactive CHAT requests do not involve making changes
{% endif %}

**SCRIPT Block:**

```lua
local response = Response.new("NOTIFY")
{% if interactive %}
reponse:ack("Responded in 'CHAT'")
{% else %}
reponse:ack("Acknowledged added context")
{% endif %}
return response
```

## Requesting Information (`REQUEST`)

If you lack the necessary information to complete a request, you can use the `"REQUEST"` category to ask for it. For example, you can request documentation for a specific library.

*(Note: The full capabilities of `REQUEST` are still under development.)*

**SCRIPT Block:**

```lua
local response = Response.new("REQUEST")
-- response:request_docs("Rust", "serde", "1.0") -- Example of a potential future API
return response
```

# API Reference

This section provides a detailed reference for all available objects and functions in the SCRIPT environment.

## `Response`

A `Response` represents your decision on how to progress. It holds a list of `Commit` objects that describe the changes to be made.

### `Response.new(category)`

Constructs a response.

-   **`category`** (string): The kind of response. Must be one of `"CHANGE"`, `"NOTIFY"`, or `"REQUEST"`.
-   **Returns**: A new `Response` object.

```lua
local response = Response.new("CHANGE")
```

### `Response:ack(message)`

Attaches a notification message to a response. This is only effective for a `Response` of category `"NOTIFY"`

-   **`message`** (string): The acknowledgement message to attach to the NOTIFY response.

### `Response:add_commit(commit)`

Adds a `Commit` object to the response. This is only effective for a `Response` of category `"CHANGE"`.

-   **`commit`** (Commit): The commit object to add.

## `Commit`

A `Commit` object represents a set of changes to a single repository, along with a summary for the commit message.

### `Commit.new(namespace)`

Creates a new `Commit` for a specific repository namespace.

-   **`namespace`** (string): The repository namespace for this commit (e.g., `"{{ example_namespace0 }}`).
-   **Returns**: A new `Commit` object.

```lua
local commit = Commit.new("{{ example_namespace0 }}")
```

### `Commit:write_file(filename, contents, opt)`

Stages a file to be written in this commit. If the file doesn't exist, it will be created. If it exists, it will be overwritten.

-   **`filename`** (string): The path to the file within the repository.
-   **`contents`** (string): The new contents for the file.
-   **`opt`** (table, optional): A table of options.
    -   `mode` (string, optional): The file mode in octal format (e.g., `"0755"` for executable). Defaults to `"0644"`.

### `Commit:delete_file(filename)`

Stages a file to be deleted in this commit.

-   **`filename`** (string): The path to the file to be deleted.

### `Commit:move_file(source, destination)`

Stages a file to be moved/renamed in this commit. This is for moving files within the same repository.

-   **`source`** (string): The original path of the file.
-   **`destination`** (string): The new path for the file.

### `Commit:set_message(summary)`

Sets the commit message summary for this commit.

-   **`summary`** (string): The commit message.

You should follow "The seven rules of a great Git commit message" (Chris Beam's) when writing the commit message, except for trivial changes that may have a one line message.

For example:

```lua
commit:set_message([[
Summarize changes in around 50 characters or less

More detailed explanatory text, if necessary. Wrap it to about 72
characters or so. In some contexts, the first line is treated as the
subject of the commit and the rest of the text as the body. The
blank line separating the summary from the body is critical (unless
you omit the body entirely); various tools like `log`, `shortlog`
and `rebase` can get confused if you run the two together.

Explain the problem that this commit is solving. Focus on why you
are making this change as opposed to how (the code explains that).
Are there side effects or other unintuitive consequences of this
change? Here's the place to explain them.

Use markdown formatting if helpful.
]])
```

Never say "as requested" to explain why you made a change, instead focus on why the change is good for the repository and project.

## Data Functions

### `get_data_block(namespace, filename)`

Retrieves the content of a named data block. Data blocks can be provided as input context or as output blocks from you.

-   **`namespace`** (string): The namespace of the data block.
-   **`filename`** (string): The name of the data block.
-   **Returns**: The content of the data block as a string, or `nil` if not found.

## Text Manipulation (`TextPatcher`)

For making line-based changes (patches) to existing files, you can use the `TextPatcher` object. It provides a interface for performing a series of modifications in a single forward pass.

TextPatcher is line-oriented in how it patches content, so it can only be used to insert, replace or delete whole lines.

Patches are built sequentially and can not overlap.

Internally the TextPatcher tracks:

1. The latest cursor position (a position in-between two lines): Updated via `:move_forward_to_context()`
2. An active selection: Updated via `:start_selection_empty()` and `:end_selection()`

The cursor can only move forwards, forcing you to build non-overlapping, in-order patches.

### Grounding Comments

To help reduce mistakes, it is strongly recommended to write a grounding comment that quotes the existing chunk of text being changed, with `-- >8 --` scissor marks to mark the lines being replaced, like:

```lua
-- PATCH: Replace print to say "Hello, world!"
--
-- Optional longer description that may span over multiple lines to
-- describe this patch
--
-- -- BEGIN --
-- #!/usr/bin/env python3
--
-- def main():
-- -- >8 --
--     print("Hello!")
-- -- >8 --
--
-- if __name__ == "__main__":
--     main()
-- -- END --
patcher:move_forward_to_context(
-- BEGIN --
[[
#!/usr/bin/env python3

def main():
]]
-- >8 -- Move cursor in front of print statement
[[
    print("Hello!")

    if __name__ == "__main__":
]]
)
patcher:start_selection_empty()

patcher:move_forward_to_context(
[[
def main():
    print("Hello!")
]]
-- >8 -- Move cursor after print statement
[[

    if __name__ == "__main__":
        main()
]]
)
patcher:end_selection()
patcher:replace_selected([[
    print("Hello, world")
]])
-- END --
```

The scissor marks should help make the boundaries for before and after context clearer before making calls to `:move_forward_to_context()`. The First call to `:move_forward_to_context()` should look at the first `-- >8 --` scissor mark, and the second call to `:move_forward_co_context()` should look at the second `-- >8 --` scissor mark.

### `TextPatcher.new(content)`
Creates a new `TextPatcher` with the initial content to be modified.

The new TextPatcher has the cursor positioned in-front of the first line.

The new TextPatcher has an empty selection positioned at the top of the content, which can be replaced to insert text at the start of the content.

- **`content`** (string): The initial text content.
- **Returns**: A new `TextPatcher` object.


### `TextPatcher:move_forward_to_context(before_context, after_context)`

Moves the cursor forwards from the current position by searching for context.

Splits the `before_context` and `after_context` strings into lines and then concatenates those to create one sequence of lines to match in the input.

If the combined sequences of lines are found (they must be found in the input without a gap) then the cursor is moved forwards to be in between the `before_context` lines and `after_context` lines.

This is designed so that you can use lots of unambiguous surrounding context to uniquely position the cursor.

IMPORTANT: Always aim to include three or more lines of context before and after the position you want to move the cursor to (six lines in total).

The `before_context` and `after_context` lines must appear back-to-back in the input content for the cursor to move!

The `before_context` and `after_context` are used to uniquely identify the location but are not part of the cursor or selection itself. It is recommended to use at least three lines of context where possible.

The cursor is logically positioned on an invisible, empty line between `before_context` and `after_context`.

IMPORTANT: The cursor can only move forwards!

IMPORTANT: This API does not affect the current selection, you must use `:start_selection_empty()` and `:end_selection()` to select text in sync with the cursor.

IMPORTANT: Consider the hierarchy of the input content when making large movements:
- Aim to move to the nearest top-level item (such as a header or class definition or function name) before moving forwards to more-specific context.

- **`before_context`** (string): A multi-line string that must appear before the cursor position, or `nil` to match the start of the file
- **`after_context`** (string): A multi-line string that must appear after the cursor position, or `nil` to match the end of the file
- **Returns**: The `TextPatcher` object, allowing for method chaining. Errors if the `before_context` lines, followed by the `after_context` lines, are not found in the input.


#### Example & Coding Style

You should use `[[` and `]]` to quote the context lines and place the quotes on their own lines to avoid issues with leading or trailing whitespace and make it easy to review patches.

Remember that Lua will automatically swallow any newline after a `[[` open quote, and a trailing newline is optional for the last line, so these are equivalent:

```lua
a = [[
one line
]]
b = [[
one line]]
c = [[one line]]
```

The Lua string should end with two new lines if you want context that ends with a blank line, e.g.:

```lua
[[
line 1
line 2

]]
```

will split into three lines like `["line 1", "line 2", ""]`


You should add a `--- Move the cursor <comment>` between the context to highlight the new position for the cursor, E.g.

```lua
patcher:move_forward_to_context(
[[
def some_function:
]]
-- >8 -- Move the cursor in front of the print statement
[[
    print("Hello")
]]
)
patcher:start_selection_empty()

cursor1 = patcher:move_forward_to_context(
[[
def some_function:
    print("Hello")
]]
-- >8 -- Move the cursor to after the print statement
[[

def some other_function:
]]
)
patcher:end_selection()

patcher:replace_selected(
[[
    print("Hello, world!")
]]
)
```

### `TextPatcher:start_selection_empty()`

Moves the start _AND_ end of the selection to the current cursor position, creating an empty selection that can be used to insert text at the cursor position without replacing any input lines.

- **Returns**: The `TextPatcher` object, allowing for method chaining.

If you call `:replace_selected()` after calling `:start_selection_empty()` that will result in inserting new lines at the current cursor position without affecting the surrounding lines.

### `TextPatcher:end_selection()`

Extend the selection by moving the end of the selection to the current cursor position.

This enables you to select input lines with two cursor movements, where you call `:start_selection_empty()` after the first move, and then call `:end_selection()` after a second cursor movement.

- **Returns**: The `TextPatcher` object, allowing for method chaining.

You can then call `:replace_selected()` to replace the lines selected between the first call to `start_selection_empty()` and the second call to `:end_selection()`


### `TextPatcher:replace_selected(replacement)`
Stages a patch that will replace the currently selected lines with the `replacement` lines.

"\n" represents one blank line.

"" represents an empty replacement with no lines which can be used to deleted the deleted lines (but it's recommended to use `:delete_selected()` instead)

When `replacement` is split into multiple lines, a trailing `\n` is optional for the last line, so "line1\nline2\n" and "line1\nline2" will both split into two lines.

If you want the last line to be blank then you should add an extra trailing "\n", e.g. "line\nline2\n\n" will split into three lines and "\n\n" will split into a single empty line.

Note: the input text is not modified on-the-fly, the patches are are recorded internally and only applied when `apply()` is called.

- **`replacement`** (string): The text to insert.
- **Returns**: The `TextPatcher` object for further chaining.


### `TextPatcher:delete_selected(replacement)`
Stages a patch that will delete the currently selected lines.

- **Returns**: The `TextPatcher` object for further chaining.

### `TextPatcher:apply()`
Retrieves the final, modified content after all staged patches have been applied in sequence.
- **Returns**: The final content as a string.


### `TextPatcher` Common Mistakes

You MUST remember that the TextPatcher cursor can only move forwards, so you are required to define a sequence of non-overlapping patches. If you find you need to go backwards you will have to first `apply()` the changes for the current patcher and create a new `TextPatcher` for a second pass, but this is not recommended.


### Example: Inserting text at the start of a file
This example demonstrates how to add a comment block at the very beginning of a file.

**Original, input `script.py` block:**

==== HEADER "script.py" ==== {{marker}}
namespace: {{ example_namespace1 }}
==== BEGIN "script.py" ==== {{marker}}
def say_hello():
    print("Hello!")

if __name__ == "__main__":
    say_hello()
==== END "script.py" ==== {{marker}}

**SCRIPT block:**
```lua
local original_script = get_data_block("{{ example_namespace1 }}", "script.py")
local patcher = TextPatcher.new(original_script)

-- The patcher starts with an empty selection at the beginning of the file.
-- Directly replacing this empty selection will insert content at the start.
patcher:replace_selected(
[[
# This is a new header comment.
# It's added at the very beginning of the file.

]]
)

local new_script = patcher:apply()
local commit = Commit.new("{{ example_namespace1 }}")
commit:write_file("script.py", new_script, { mode = "0755" })
commit:set_message([[Add a header comment to script.py]])
local response = Response.new("CHANGE")
response:add_commit(commit)
return response
```

**Resulting `script.py`:**
```python
# This is a new header comment.
# It's added at the very beginning of the file.

def say_hello():
    print("Hello!")

if __name__ == "__main__":
    say_hello()
```

### Example: Updating a print statement in a Python script
This example demonstrates how edit a print statement within a Python script.

**Original, input `script.py` block:**

==== HEADER "script.py" ==== {{marker}}
namespace: {{ example_namespace1 }}
==== BEGIN "script.py" ==== {{marker}}
#!/usr/bin/env python3

def say_hello():
    print("Hello!")

def main():
    say_hello()

if __name__ == "__main__":
    main()
==== END "hello.py" ==== {{marker}}

**SCRIPT block:**
```lua
local original_script = get_data_block("{{ example_namespace1 }}", "script.py")
local patcher = TextPatcher.new(original_script)

patcher:move_forward_to_context(
[[
def say_hello():
]],
-- >8 -- Move the cursor to the start of the `print` line
[[
    print("Hello!")

def main():
]]
)
patcher:start_selection_empty() -- Start a selection we can extend

patcher:move_forward_to_context(
[[
def say_hello():
    print("Hello!")
]],
-- >8 -- Move the cursor after the `print` statement
[[

def main():
]]
)
patcher:end_selection() -- Extend selection to include the print statement

patcher:replace_selected(
[[
    print("Hello, world!")
]]
)

local new_script = patcher:apply()
local commit = Commit.new("{{ example_namespace1 }}")
commit:write_file("script.py", new_script, { mode = "0755" })
commit:set_message([[Patch script.py to say 'Hello, world!']])
local response = Response.new("CHANGE")
response:add_commit(commit)
return response
```

**Resulting `script.py`:**
```python
#!/usr/bin/env python3

def say_hello():
    print("Hello, world!")

def main():
    say_hello()

if __name__ == "__main__":
    main()
```