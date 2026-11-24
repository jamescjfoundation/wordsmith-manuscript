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

## Synced folders

A project can live in a folder a sync service keeps in step between your computers. Every few
seconds the app looks for changes made elsewhere: a document changed only on the other computer is
read in (the editor shows it); a document changed on both keeps the version on this computer, and
the other one becomes a snapshot named "Changed elsewhere" that you can compare word by word and
restore; a binder changed elsewhere reloads the project when nothing here is unsaved, and otherwise
is kept as a copy. A save never writes over a change it has not seen without copying it first to
the project's `conflicts` folder. Nothing is lost either way; the status bar says what happened.

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
