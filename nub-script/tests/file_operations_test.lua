local response = Response.new("CHANGE")

local commit1 = Commit.new("repo1")
commit1:delete_file("file_to_delete.txt")
commit1:move_file("old_name.txt", "new_name.txt")
commit1:set_message("Delete and move files in repo1")
response:add_commit(commit1)

local commit2 = Commit.new("repo2")
commit2:delete_file("another_file.txt")
commit2:set_message("Delete a file in repo2")
response:add_commit(commit2)

return response