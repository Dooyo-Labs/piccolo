local original_script = get_data_block("repo.namespace", "script.py")
local patcher = TextPatcher.new(original_script)

-- A new patcher has an empty selection at the start of the file that
-- can be used to insert new content.
patcher:replace_selected(
[[
#!/usr/bin/env python3

]]
)

patcher:move_forward_to_context(
[[
def say_hello():
]],
-- Move cursor in front of the `print` line
[[
    print("Hello!")

def main():
]]
)
patcher:start_selection_empty()

patcher:move_forward_to_context(
[[
def say_hello():
    print("Hello!")
]],
-- Move cursor after the `print` line
[[

def main():
]]
)
patcher:end_selection()

patcher:replace_selected(
[[
    print("Hello, world!")
]]
)

local new_content = patcher:apply()

local response = Response.new("CHANGE")
local commit = Commit.new("repo.namespace")
commit:write_file("script.py", new_content)
commit:set_message("Apply text manipulations")
response:add_commit(commit)
return response