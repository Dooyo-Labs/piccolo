nub-script implements a scripting runtime for the nubby LLM coding runtime,
used to apply generated LLM outputs as changes to the project.

The model is is provided context for a project that may be comprised of
multiple repositories that each have a unique "repo.*" namespace.

The model will typically be provided the existing contents of files in each
repository as input context, as well as other attachments, like:

```pre
==== HEADER "README.md" ==== {marker}
namespace: attachment.template
==== BEGIN "README.md" ==== {marker}
This is a test project written in $LANG
==== END "README.md" ==== {marker}

==== HEADER "filename.rs" ==== {marker}
namespace: repo.my_repo0
==== BEGIN "filename.rs" ==== {marker}
// File contents
fn old_function() {
    println!("hello");
}
==== END "filename.rs" ==== {marker}

==== HEADER "hello.py" ==== {marker}
namespace: repo.my_repo1
mode: "0755"
==== BEGIN "hello.py" ==== {marker}
#!/usr/bin/env python3

def say_hello():
    print("Hello!")

def main():
    say_hello()

if __name__ == "__main__":
    main()
==== END "hello.py" ==== {marker}
```

(Where the model understands that only files in the "repo.my_repo0" and
"repo.my_repo1" namespaces belong to the project, and other files are just
read-only files for reference)

The model is then responsible for outputting data blocks and writing a SCRIPT
block that will modify the "repo.my_repo0" and "repo.my_repo1" repository
namespaces.

For example, based on the input blocks above:

```pre
==== BEGIN PLAN ==== {marker}
I will add a README to repo.my_repo0 and repo.my_repo1 based on the example
template and patch hello.py to print "Hello, world!"
==== END PLAN ==== {marker}

==== HEADER "README.md" ==== {marker}
namespace: repo.my_repo0
==== BEGIN "README.md" ==== {marker}
This is a test project written in Rust
==== END "README.md" ==== {marker}

==== HEADER "README.md" ==== {marker}
namespace: repo.my_repo1
==== BEGIN "README.md" ==== {marker}
This is a test project written in Python
==== END "README.md" ==== {marker}

==== BEGIN SCRIPT ==== {marker}
local response = Response.new("CHANGE")

local commit0 = Commit.new("repo.my_repo0")
commit0:write_file("README.md", get_data_block("repo.my_repo0", "README.md"))
commit0:set_message("Add README for repo0")

local commit1 = Commit.new("repo.my_repo1")
commit1:write_file("README.md", get_data_block("repo.my_repo1", "README.md"))

local original_hello_py = get_data_block("repo.my_repo1", "hello.py")
local patcher = TextPatcher.new(original_script)

patcher:move_to_context(
[[
def say_hello():
]],
-- Move the selection cursor to the start of the `print` line
[[
    print("Hello!")

def main():
]]
)
patcher:move_to_context(
[[
def say_hello():
    print("Hello!")
]],
-- Extend the selection to the start of the blank line after the `print` statement
[[

def main():
]]
)
patcher:replace_selection(
[[
    print("Hello, world!")
]]
)

local new_hello_py = patcher:apply()

commit1:write_file("hello.py", new_hello_py, { mode = "0755" })
commit1:set_message("Add README and print 'Hello, world!'")

response:add_commit(commit0)
response:add_commit(commit1)
return response
==== END SCRIPT ==== {marker}

==== BEGIN SUMMARY ==== {marker}
I have added a READMEs and update hello.py in repo.my_repo1 to print "Hello, world!"
==== END SUMMARY ==== {marker}
```

When the model needs to acknowledge context without changing anything then
the script block would look something like:

```pre
==== BEGIN SCRIPT ==== {marker}
local response = Response.new("NOTIFY")
return response
==== END SCRIPT ==== {marker}
```