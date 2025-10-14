# Using src to generate exact sources for compilation

Generally with existing purescript tooling you give the compiler a list of source globs to compile, which is usually all dependencies for a project, and it goes away and compiles the lot. This is convenient but can end up compiling tonnes more than necessary.

This command compiles the exact sources necessary by looking at the imports in /src (plus /test if the flag is on) and following those recursively.
