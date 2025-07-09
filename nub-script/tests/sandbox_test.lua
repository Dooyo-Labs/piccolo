-- We expect these to fail because the `io` and `os` libraries are not fully loaded.
-- The functions that could perform file system access or execute commands
-- should not be available.

local success = true
local errors = {}

-- `dofile` is not part of the base library loaded by `Lua::full()`
local dofile_ok, dofile_err = pcall(function() return dofile("test.txt") end)
if dofile_ok or not string.find(tostring(dofile_err), "could not call a nil value") then
    success = false
    table.insert(errors, "dofile did not fail as expected: " .. tostring(dofile_err))
end

-- `loadfile` is not part of the base library loaded by `Lua::full()`
local loadfile_ok, loadfile_err = pcall(function() return loadfile("test.txt") end)
if loadfile_ok or not string.find(tostring(loadfile_err), "could not call a nil value") then
    success = false
    table.insert(errors, "loadfile did not fail as expected: " .. tostring(loadfile_err))
end

-- `os` library is not loaded at all
local os_ok, os_err = pcall(function() return os.execute("echo 'hello'") end)
if os_ok or not string.find(tostring(os_err), "could not index into a nil value") then
    success = false
    table.insert(errors, "os.execute did not fail as expected: " .. tostring(os_err))
end

-- The `io` table is not loaded.
local io_ok, io_err = pcall(function() return io.open("test.txt", "w") end)
if io_ok or not string.find(tostring(io_err), "could not index into a nil value") then
    success = false
    table.insert(errors, "io.open did not fail as expected: " .. tostring(io_err))
end


if success then
    local response = Response.new("CHANGE")
    local commit = Commit.new("repo")
    commit:write_file("result.txt", "success")
    response:add_commit(commit)
    return response
else
    error(table.concat(errors, "\n"))
end