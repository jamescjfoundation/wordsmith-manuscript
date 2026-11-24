# The binder

The binder holds everything in the project as a tree, in five fixed areas:

| Area | For |
| --- | --- |
| Manuscript | The book itself: what exports include |
| Story Bible | Characters, places, objects, groups, lore and plot threads |
| Research | Research notes, earlier drafts, rough work |
| Notes | Your plans and notes |
| Trash | Deleted items, until you empty it |

## Items and roles

Items are documents and folders (and bible entries in the Story Bible). A document can hold other
documents, so a chapter can be a document with its scenes under it.

Each manuscript item has a **role**: part, chapter, scene, front matter, back matter, or none.
Roles decide how exports title and number things, so you never type "Chapter 3" yourself. New
items take a role from where they go: inside a part they are chapters, inside a chapter scenes.
Right-click an item to change its role.

**Include in export** (right-click) leaves an item and everything under it out of exports
without deleting it; it shows dimmed.

## Moving things

Drag an item before, after or inside another; a line or box shows where it will land. You can
also use Mod+Alt+Up and Down to move an item among its neighbours, and Mod+Alt+Right and Left to
move it into the item before it or out of its folder. Every move is one undo step.

Rename an item with F2 or a double-click on its title.

## The Trash

Delete (or Move to Trash) puts an item and its children in the Trash. Restore puts it back where it
came from. **Empty Trash** asks first, saying how many items and words go, and removes them for
good once the project is saved.

## Bookmarks

Mod+D (or right-click › Bookmark) bookmarks the selected item; its title then sits in the row of
bookmarks above the binder, one click away from anywhere in the project. Press Mod+D again, or
right-click › Remove Bookmark, to take it off. Items in the Trash leave the row until restored.

## Pictures, PDFs and web links

Research can hold more than notes. Right-click Research (or a folder in it) › Add Image or PDF…
and type the file's path: PNG, JPEG, GIF and WebP pictures and PDFs up to 64 MB are copied into
the project's `media` folder, so they travel with it, in packed `.wsmz` files and backups too.
Add Web Link… keeps a web address; nothing is downloaded. Select one to see it in the editor
area: a picture fits the width, a PDF shows the text of each page (Open in Another App shows the
pages themselves once the project is saved), and a link shows its address with Open in Browser.

## Screenplays

A project can hold screenplays, for an adaptation or a book with a script in it. Add one with
Add Image or PDF… and choose a Wordsmith Script file (`.wss`): it goes into the Manuscript as a
screenplay item, its file kept exactly as Wordsmith Script wrote it. Select it to read it here,
with its word and scene counts; give it a synopsis, label and status like any item, and it shows
on the corkboard and in the outline. To change it, Open in Another App opens it in Wordsmith
Script; when Script saves, the changes show here. Screenplays are not part of the manuscript's
exports.
