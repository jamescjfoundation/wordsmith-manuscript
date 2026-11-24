# Files, saving and backups

## The project folder

A project is a folder of plain text files you can read in any text editor:

| File | Holds |
| --- | --- |
| `project.toml` | The book's details, label, status and keyword lists, targets, settings |
| `binder.toml` | The binder: every item's id, area, place, role, title, label and status |
| `text/<id>.md` | Each document's text, in a Markdown dialect, with its synopsis and other details at the top |
| `bible/<id>.md` | Each bible entry |
| `snapshots/<id>/<time>.md` | Snapshots |
| `media/` | Pictures and other files |

A change to one scene changes one file, so projects work well with version control and synced
folders. `docs/format.md` in the source specifies every file.

## Saving

Your work is saved automatically as you write, every file safely (a save that is cut short leaves
the previous file whole). File › Save saves at once.

## Recovery

While you work, unsaved changes are kept in the project's `.recovery` folder. If the app stops
without saving, the next time you open the project it offers the unsaved work back: Recover It, or
Keep the Saved Version.

## Backups

When you close a project, and every 30 minutes while you write, a packed copy (`.wsmz`) goes into a
`Backups` folder beside the project; the newest ten are kept. Open a backup like any packed
project.

## One file

File › Save As › Save as One File writes the project as a single `.wsmz`, for sending or archiving.
Opening one unpacks it into a folder.
