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

Internally the TextPatcher tracks one active selection: Updated via `:select_next_lines()`

Each call to `:select_next_lines()` makes a new selection that comes after the previous selection.

### `TextPatcher.new(content)`
Creates a new `TextPatcher` with the initial content that will be patched.

The new TextPatcher has an empty selection positioned at the top of the content, which can be replaced to insert text at the start of the content.

- **`content`** (string): The initial text content.
- **Returns**: A new `TextPatcher` object.


### `TextPatcher:select_next_lines(tagged_lines)`

Moves the selection forwards by searching for a quoted sequence of existing lines, where each line is tagged as `B` (before), `S` (selection) or `A` (after).

For an input file with the following contents:

```
Unrelated content
Unrelated content
Before context 1
Before context 2
Before context 3
Selection line 1
Selection line 2
After context 1
After context 2
After context 3
Unrelated content
Unrelated content
```

Here is an example of how a sequence of input lines might be tagged, including a grounding comment:

```lua
-- PATCH: Select some example lines from this chunk
--
-- ```
-- Before context 1
-- Before context 2
-- Before context 3
-- Selection line 1
-- Selection line 2
-- After context 1
-- After context 2
-- After context 3
-- ```
patcher:select_next_lines( {
    {"B", "Before context 1"}, -- Before context, to disambiguate which lines we are selecting
    {"B", "Before context 2"},
    {"B", "Before context 3"},
    {"S", "Selection line 1"}, -- Selection lines to delete or replace
    {"S", "Selection line 2"},
    {"A", "After context 1"}, -- After context, to disambiguate which lines we are selecting
    {"A", "After context 2"},
    {"A", "After context 3"},
})
```

### Tags:

"B" - means the line comes before any line you want to select
"S" - means that the line will become part of the new selection
"A" - means the line comes after any line you want to select

IMPORTANT: You can create an empty selection if no lines are tagged with "S", which is useful for inserting new text between "B" and "A" lines.
IMPORTANT: The "S", selection lines can not overlap with any previous selection lines

This is designed so that you can use lots of unambiguous surrounding context to uniquely identify the lines to select.

IMPORTANT: Consider the hierarchy of the input content when making large movements:
- Aim to move to the nearest top-level item (such as a header or class definition or function name) before moving forwards to more-specific context.

- **`tagged_lines`** (table): A multi-line table that quotes and tags a verbatim sequence of, existing, input lines.
- **Returns**: The `TextPatcher` object, allowing for method chaining. Errors if the lines, are not all found in the input.


IMPORTANT: Always add a grounding comment that quotes the unpatched lines of interest from the input without any tags (including before and after context).

IMPORTANT: The given lines must be a verbatim sequence of lines from the (unpatched) input content with no gaps.

This API will concatenate all of the given lines and search for an exact match within the content passed to `TextPatcher.new()` before using the tags to update the selection.

IMPORTANT: Remember to escape characters in the line if needed, to maintain valid Lua string literals.



#### Example

Replace a print statement so it says "Hello, world!"

```lua
-- PATCH - Update print to say "Hello, world!", within this chunk:
--
-- ```
--
-- def some_function():
--     print("Hello")
--
-- def some_other_function:
-- ```

patcher:select_next_lines({
    {"B", ""},
    {"B", "def some_function():"},
    {"S", "    print(\"Hello\")"},
    {"A", ""},
    {"A", "def some_other_function:"},
})

patcher:replace_selected(
[[
    print("Hello, world!")
]]
)
```

### `TextPatcher:select_end()`

Creates an empty selection that can be used to insert text at the end of the content

IMPORTANT: Remember that the selection can only move forwards so `:select_end()` can only be used once, for the last patch.

Use `:replace_selected()` afterwards to insert lines at the end, like:

```lua
-- Create an empty selection at the end of the script
patcher:select_end()
-- Add a blank line and a comment at the end of this Python script
patcher:replace_selected([[

# This is the end of this script
]])
```

### `TextPatcher:replace_selected(replacement)`
Stages a patch that will replace the currently selected ("S" tagged) lines with the `replacement` lines.

"\n" represents one blank line.

"" represents an empty replacement with no lines which can be used to deleted the deleted lines (but it's recommended to use `:delete_selected()` instead)

When `replacement` is split into multiple lines, a trailing `\n` is optional for the last line, so "line1\nline2\n" and "line1\nline2" will both split into two lines.

If you want the last line to be blank then you should add an extra trailing "\n", e.g. "line1\nline2\n\n" will split into three lines and "\n\n" will split into a single empty line.

Note: the input text is not modified on-the-fly, the patches are are recorded internally and only applied when `apply()` is called.

- **`replacement`** (string): The text to insert.
- **Returns**: The `TextPatcher` object, allowing for method chaining.

### Coding Style

You should use `[[` and `]]` to quote the replacement or insertions lines and place the quotes on their own lines to avoid issues with leading or trailing whitespace and make it easy to review patches.

Remember that Lua will automatically swallow any newline after a `[[` open quote, and a trailing newline is optional for the last line, so these are equivalent:

```lua
patcher:replace_selected(
[[
one line
]]
)

patcher:replace_selected(
[[
one line]]
)

patcher:replace_selected([[one line]])
```

The Lua string should end with two new lines if you want context that ends with a blank line, e.g.:

```lua
patcher:replace_selected[[
line 1
line 2

]]
```

### `TextPatcher:delete_selected(replacement)`
Stages a patch that will delete the currently selected, 'S' lines.

- **Returns**: The `TextPatcher` object for further chaining.

### `TextPatcher:apply()`
Retrieves the final, modified content after all staged patches have been applied in sequence.
- **Returns**: The final content as a string.

### `TextPatcher` Grounding Comments

It is strongly recommended to add grounding comments before each pair of select and replacement API calls that include a full quote of the chunk of input text that will be modified, like:

```lua
-- PATCH - Update print to say "Hello, world!", within this chunk:
--
-- ```
--
-- def some_function():
--     print("Hello")
--
-- def some_other_function:
-- ```

patcher:select_next_lines({
    {"B", ""},
    {"B", "def some_function():"},
    {"S", "    print(\"Hello\")"},
    {"A", ""},
    {"A", "def some_other_function:"},
})

patcher:replace_selected(
[[
    print("Hello, world!")
]]
)
```

The grounding comment should include surrounding the same surrounding context that will be passed to `:select_next_lines()`

The only difference should be the addition of tags when calling `:select_next_lines()`


### `TextPatcher` Common Mistakes

You MUST remember that the TextPatcher selection can only move forwards, so you are required to define a sequence of non-overlapping patches. If you find you need to go backwards you will have to first `apply()` the changes for the current patcher and create a new `TextPatcher` for a second pass, but this is not recommended.

You MUST remember that full sequence of tagged lines passed to `:select_next_lines(tagged_lines)` must match a verbatim match for a sequence of lines from the original content passed to `TextPatcher.new()`, with no gaps.


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
#!/usr/bin/env python3

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
#!/usr/bin/env python3

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

# This is an example python script

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

-- PATCH: Update print statement to say "Hello, world!", within this chunk:
--
-- ```
-- # This is an example python script
--
-- def say_hello():
--     print("Hello!")
--
-- def main():
--     say_hello()
-- ```


-- Select the print statement we want to replace, and provide surrounding before/after context
patcher:select_next_lines({
    {"B", "# This is an example python script"},
    {"B", ""},
    {"B", "def say_hello():"},
    {"S", "    print(\"Hello\")"},
    {"A", ""},
    {"A", "def main():"},
    {"A", "    say_hello()"},
})

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

# This is an example python script

def say_hello():
    print("Hello, world!")

def main():
    say_hello()

if __name__ == "__main__":
    main()
```

### Example: Inserting a comment without replacing any lines
This example demonstrates how to add a comment to a Python script, without deleting or replacing any existing lines

**Original, input `script.py` block:**

==== HEADER "script.py" ==== {{marker}}
namespace: {{ example_namespace0 }}
==== BEGIN "script.py" ==== {{marker}}
#!/usr/bin/env python3

# This is an example python script

def say_hello():
    print("Hello!")

def main():
    say_hello()

if __name__ == "__main__":
    main()
==== END "hello.py" ==== {{marker}}

**SCRIPT block:**

```lua
local original_script = get_data_block("{{ example_namespace0 }}", "script.py")
local patcher = TextPatcher.new(original_script)

-- Quote the lines around the position we want to insert new text
-- Tag lines before the insertion position with "B" and the lines after the insertion position with "A"
patcher:select_next_lines({
    {"B", ""},
    {"B", "# This is an example python script"},
    {"B", ""},
    {"A", "def say_hello():"},
    {"A", "    print(\"Hello!\")"},
    {"A", ""},
})
-- Replace the empty selection to insert new text
patcher:replace_selected(
[[
# This function prints "Hello!"
]]
)

local new_script = patcher:apply()
local commit = Commit.new("{{ example_namespace0 }}")
commit:write_file("script.py", new_script, { mode = "0755" })
commit:set_message([[Patch script.py to add a comment']])
local response = Response.new("CHANGE")
response:add_commit(commit)
return response
```

**Resulting `script.py`:**
```python
#!/usr/bin/env python3

# This is an example python script

# This function prints "Hello!"
def say_hello():
    print("Hello!")

def main():
    say_hello()

if __name__ == "__main__":
    main()
```

### Example: Inserting a comment at the end

This example demonstrates how to use `select_end()` to insert text at the end of the content.

**Original, input `script.py` block:**

==== HEADER "script.py" ==== {{marker}}
namespace: {{ example_namespace0 }}
==== BEGIN "script.py" ==== {{marker}}
#!/usr/bin/env python3

def main():
    print("Hello, world!")

if __name__ == "__main__":
    main()
==== END "hello.py" ==== {{marker}}

**SCRIPT block:**

```lua
local original_script = get_data_block("{{ example_namespace0 }}", "script.py")
local patcher = TextPatcher.new(original_script)

-- Jump to the end of the file and create an empty selection
patcher:select_end()
-- Insert a blank line and comment by replacing the empty selection at the end of the file
patcher:replace_selected([[

# This is an example Python script
]])

local new_script = patcher:apply()
local commit = Commit.new("{{ example_namespace0 }}")
commit:write_file("script.py", new_script, { mode = "0755" })
commit:set_message([[Patch script.py to add a comment']])
local response = Response.new("CHANGE")
response:add_commit(commit)
return response
```

**Resulting `script.py`:**
```python
#!/usr/bin/env python3

def main():
    print("Hello, world!")

if __name__ == "__main__":
    main()

# This is an example Python script
```
