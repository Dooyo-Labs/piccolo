nub-script
==========

This goal of this project is to implement a Lua SCRIPT block runtime for the
Nubby coding agent.

All the code under `piccolo/` and `piccolo-util/` is for context, to understand
the implementation details of the Lua VM that will be used by the runtime.

Your main focus is the `nub-script/` crate.

Avoid editing the `piccolo` or `piccolo-util` crates under `src/` or `util/`
unless strictly necessary or you have been explicitly asked to.

If you find yourself wanting to edit code under `util/` or `src/` then you
should first re-consider the design of the code that is leading to this and see
if it's possible to change the design of `nub-script` to avoid needing changes
to `piccolo` or `piccolo-util`.


docs/script-block.md
====================

This is the documentation that will become part of the system-instructions for
the Nubby coding agent.

It should be written as instructions as to how the SCRIPT block API must be
used to respond to requests.

The system instructions will already assert that a SCRIPT block must always
be output, before a final SUMMARY block.

The documentation needs to make it very clear how to write a SCRIPT in these situations:

1. When no changes are needed and the model needs to just acknowledge new context
2. When no changes are needed and the model has already written a CHAT block that responds to a user request
3. When files in one or more repositories need to be added, removed or changed

The documentation should show canonical examples of how the model can write
a SCRIPT block for each of these cases.

The examples should also show example data blocks that are inputs for the script.

The script-block.md documentation is templated using the Rust Tera crate, with
the following variables:

- {{ marker}} should be used as a marker when printing example data blocks
- {{ example_namespace0 }} and {{ example_namespace1 }} should be used as example repository namespaces (not including quotes)
- {% if interactive %} {% endif %} - should be used around APIs related to CHAT usage that is only relevant while in an interactive chat session


test-log 0.2
============

When writing any Rust unit tests, make sure to annotate tests with
`#[test_log::test]` or for tokio tests use `#[test_log::test(tokio::test)]`
because this will enable more verbose logging within the tests.

Whenever you see from the git log context that there are repeated,
similar changes trying to fix the same test failures, make sure that
logging is enabled for the unit test and also add additional logging
to any related code that might give more insight into the failure
in case the issue comes up again in the future.


INCEPTION WARNING
=================

Beware that the documentation for this project has a lot of overlap with your
existing system-instructions but that MUST NOT distract you from your
existing system instructions.

Although the nub-script Lua runtime may replace your SCRIPT block implementation
in the future, you can not yet use the features documented here in your own
SCRIPT block output.
