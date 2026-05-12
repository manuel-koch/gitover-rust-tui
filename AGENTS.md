# Gitover Rust UI

A rust based terminal UI to track git status of multiple repositories.

Check [features](./docs/features.md) for list of implemented features.

Check [todo](./docs/todo.md) for recent implemenation tasks and remaining tasks.

## Cleanup Todo and merge to Features

Only perform cleanup of todo document when user requests it explicitly !

Force re-reading features and todo document to fully grasp their current content !

Merge finished todo tasks with the features document:

- Check if there is an existing feature that matches task content fully/almost/partly
  - If feature is matched fully, just remove the task from todo
  - If feature is matched partly/almost, check whats the diff to task content and decide if feature
    text should be updated or a new distinct feature be introduced with the task content
  - If feature is not matched, introduce a new distinct feature with tasks content.
    If needed check if the new feature belongs to a new section/heading within the documents
    to group features by topics.
  - if in doubt if a task matches a feature, ask the user how to proceed, provide proposal what you think would fit best.
- Don't remove empty todo sections - we might add new tasks to it, just add a placeholder "- [ ]" task instead.

For updated features document, consult the sources/implementation to check if features are actually implemented
the way they are currently stated in the feature description.
Update the feature descriptions to matc the current implementation.
