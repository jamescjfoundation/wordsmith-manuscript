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
folders. A file in the project that is a symbolic link is not followed: the app does not read it,
pack it or write over it. `docs/format.md` in the source specifies every file.

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
project. Each backup is named after the project and the time it was made; a second one in the
same second gets a number after the time, and a file you put in the folder under another name is
never counted or removed.

## Other versions of the app

The project's files say which version of the format they are in (`schema` in `project.toml`).
This version of the app writes format 3.

- **A project from an older version always opens.** Formats 1 and 2 are read as they are, and
  nothing is written when a project is only opened. The first save writes the project in
  format 3, with every word, id and setting kept.
- **A project saved by a newer version than yours is not opened.** The app says that the
  project was saved by a newer version, with both format numbers, and changes nothing in the
  folder. Update the app ([Installing and updating](installing.md)) and open it again.
- The same holds for a packed project (`.wsmz`) and for a backup: its format is that of the
  project inside it.

Wordsmith Book reads the same formats; a Book older than the Manuscript that saved a project
says so in its own words.

## One file

File › Save As › Save as One File writes the project as a single `.wsmz`, for sending or archiving.
Opening one unpacks it into a folder.
