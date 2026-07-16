# Release Notes Guidelines

## Context

When requested by user, create brief release notes for recent commits ( HEAD vs last tag,
or an explicit range ).
Check the content of every commit, omit commits that are clearly not related to a
feature or bugfix.

Write the release notes to `docs/release-notes-<FROM>-to-<TO>.md`, e.g. `docs/release-notes-v0.2-to-v0.3.md` (overwrite if it already exists).
Do NOT print the content to the conversation — only confirm the file was written.
Prefix the release notes with a one-sentence title that denotes most important aspect of the release.

## Entry Format

Use the following format for each applicable commit ( curly braces denote dynamic
content based on the commit ).
Output entries one after another with NOTHING between them — no `---` separators,
no blank lines between entries, no numbered lists, no extra headings.
The trailing blank line inside each entry is the only spacing.

<commit-format>
**{title}**

{Brief topic ( 1-2 sentences max ) of feature or bugfix change}

</commit-format>

## Example

Example file content for two consecutive entries ( do not deviate from this ):

<commit-format-example>
**Fix: Wrong Activity Indicator**

Running a status-pane action incorrectly showed "fetching" in the Activity spinner. These operations now correctly show "working".

**Case-Insensitive Path Sorting**

File paths in the Status Details pane and History are now sorted case-insensitively by default, configurable via `general.case_sensitive_path_sorting`.

</commit-format-example>
