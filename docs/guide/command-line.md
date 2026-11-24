# The command line and agents

`manuscript-cli` does the app's work without its window.

| Command | What it does |
| --- | --- |
| `manuscript-cli convert <input> <output>` | A document, or a folder as a document set, into a project folder or `.wsmz`; or a project into any export (by the output's extension) |
| `manuscript-cli export <project> <file> [--format F] [--preset NAME]` | Export; formats: `docx`, `odt`, `rtf`, `markdown`, `text`, `html`, `manuscript-docx`, `manuscript-pdf`, `book` |
| `manuscript-cli counts <project> [--item ID]` | Words and characters, target and progress |
| `manuscript-cli verify <source> <result>` | Word-for-word check of two documents or projects; exits with 2 and shows the first difference when they differ |
| `manuscript-cli report <project>` | Sections, words, targets, labels and statuses, and missing book details |
| `manuscript-cli run <project> <command> [JSON]` | Any command (saved when it changes the project) |
| `manuscript-cli commands` | Every command with its parameters |
| `manuscript-cli mcp <project>` | Serve the commands to an agent over standard input and output |

## Agents

`manuscript-cli mcp` serves every Manuscript command as a tool (`manuscript_binder_new`,
`manuscript_text_set`, `manuscript_export_file` and so on) beside the core's general tools. Start
with `manuscript_binder_list`. Every change is one undo step (`manuscript_edit_undo`), and a project
with a folder is saved after each change.
