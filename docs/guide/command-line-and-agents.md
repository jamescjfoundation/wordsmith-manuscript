# The command line and agents

`wordsmith-manuscript-cli` does the app's work without its window.

| Command | What it does |
| --- | --- |
| `wordsmith-manuscript-cli convert <input> <output>` | A document, or a folder as a document set, into a project folder or `.wsmz`; or a project into any export (by the output's extension) |
| `wordsmith-manuscript-cli export <project> <file> [--format F] [--preset NAME]` | Export; formats: `docx`, `odt`, `rtf`, `markdown`, `text`, `html`, `epub`, `manuscript-docx`, `manuscript-pdf`, `book` |
| `wordsmith-manuscript-cli counts <project> [--item ID]` | Words and characters, target and progress |
| `wordsmith-manuscript-cli verify <source> <result>` | Word-for-word check of two documents or projects; shows the first difference when they differ |
| `wordsmith-manuscript-cli report <project>` | Sections, words, targets, labels and statuses, and missing book details |
| `wordsmith-manuscript-cli run <project> <command> [JSON]` | Any command (saved when it changes the project); the parameters are one JSON object |
| `wordsmith-manuscript-cli commands` | Every command with its parameters |
| `wordsmith-manuscript-cli mcp <project> [--read-root <folder>] [--write-root <folder>]` | Serve the commands to an agent over standard input and output |
| `wordsmith-manuscript-cli --help`, `-h`, `help` | The list above |
| `wordsmith-manuscript-cli --version`, `-V` | `wordsmith-manuscript-cli` and its version, on one line |

A project is a project folder (`.wsm`), its `project.toml`, or a packed project (`.wsmz`). A
packed project is opened in memory: a command that changes it does not write the change back.

## Results, errors and exit codes

A command's result is written to standard output, as JSON. What went wrong is written to
standard error, as `wordsmith-manuscript-cli: <reason>`. The exit code says which it was:

| Code | Means |
| --- | --- |
| 0 | It did what was asked. For `verify`: every word matches |
| 1 | It could not: a file could not be read or written, a command refused, an export's word check stopped the export |
| 2 | The command line was wrong, and nothing was done: no command, an unknown command or option, a missing or extra argument, an option without its value, parameters that are not a JSON object. The list of commands follows the reason |
| 3 | `verify` ran and the two texts differ. The answer is on standard output, as it is when they match |

So a script reads `verify` as 0 "the same", 3 "they differ", and 1 or 2 "the check did not
run". An option is spelled exactly: `--fromat` is refused, not passed over. With no argument at
all the program does nothing and ends with 2; ask for the list with `--help`.

The same rules hold for the command lines of the other Wordsmith apps.

## Agents

`wordsmith-manuscript-cli mcp <project>` serves every Manuscript command to an agent as a tool, named
`manuscript_` and the command's id (`manuscript_binder_new`, `manuscript_text_set`,
`manuscript_export_file` and so on). Those are all the tools there are: nothing works on a
document outside the project. Start with `manuscript_binder_list`. Every change is one undo step
(`manuscript_edit_undo`), and a project with a folder is saved after each change. Each tool says
whether it only reads; an export leaves the project alone but writes a file, so it does not
count as only reading.

### The folders an agent may use

Some commands take the path of a file or folder: the imports read one, the exports and a new
series bible write one. Two options say where those paths may point:

| Option | Meaning |
| --- | --- |
| `--read-root <folder>` | The only folder the agent's commands may read files in |
| `--write-root <folder>` | The only folder the agent's commands may write files in |

- The folder must exist. A path is inside it when it lies under it once links are followed.
- An option given alone grants only its own kind of access: with `--read-root` alone the agent
  can export nothing, and with `--write-root` alone it can import nothing.
- With neither option nothing is confined, and the program says so on standard error when it
  starts.
- The project you name is your choice, not the agent's. It is opened before the folders are put
  in force, read where it lives and saved there, wherever that is. The same holds for a series
  bible the project already shares.
- `manuscript_export_convert` runs a converter program the caller names. A program cannot be
  kept to a folder, so that tool is refused whenever either option is given.

| Tool | Reads | Writes |
| --- | --- | --- |
| `manuscript_import_document`, `manuscript_import_preview`, `manuscript_import_notes`, `manuscript_media_import` | `path` | |
| `manuscript_import_set` | `paths`, `folder` | |
| `manuscript_offers_list`, `manuscript_offers_accept`, `manuscript_offers_reject` | `path` | |
| `manuscript_export_file`, `manuscript_export_convert` | | `path` |
| `manuscript_series_create` | | `path` |
| `manuscript_series_join` | `path` | `path` |

The folders keep an agent to the files you gave it. They are not a guard against other programs
on the computer.
