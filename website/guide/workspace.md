# Workspace and tabs

MD View opens one folder as a workspace and one tab per document. The explorer is lazy: a folder
loads its children the first time you expand it, and the watcher refreshes only the directories
that are already loaded.

## Open a folder

Use **File → Open Folder…** or pass a folder on the command line:

```sh
mdview ~/notes
mdview ~/notes ~/notes/today.md
```

The title bar shows the document name. The tab strip belongs to the editor column, with the
active tab merged into the document surface.

## Tabs follow the explorer

Selecting a tab expands the file's parent folders, scrolls it into view, and marks the row with
the active accent. The selection only follows a tab change, so clicking a folder does not steal
the active-file marker.

![Explorer and tabs](/images/explorer.png)

## Files outside the workspace

Documents opened from outside the workspace appear in a fixed **External files** group at the end
of the tree. The group always stays expanded and cannot be renamed, deleted, or used as a parent
for new entries. External documents can be renamed; they cannot be deleted from the explorer or
with **Delete File**. Closing the last external tab removes the group.

![External files group](/images/external-files.png)

## Session restore

The app remembers open files, the active tab, per-tab scroll positions and split ratios, the
workspace folder, sidebar width, and recent files. Reopening the app restores the workspace and
tabs without writing anything to the document.

## File changes on disk

A native watcher reports changes in loaded directories. If a clean file changes on disk it is
reloaded; if the buffer is dirty, MD View shows a conflict bar with **Reload from disk**, **Keep my
changes**, and **Compare**. A missing file gets its own bar with **Save As…** and **Keep editing**.

## Keyboard

| Shortcut | Action |
|---|---|
| `Ctrl/Cmd+O` | Open file |
| `Ctrl/Cmd+Shift+O` | Open folder |
| `Ctrl/Cmd+S` | Save |
| `Ctrl/Cmd+Shift+S` | Save As |
| `Ctrl/Cmd+W` | Close tab |
| `Ctrl/Cmd+P` | Quick Open |
| `Ctrl/Cmd+Shift+P` | Command palette |
| `Ctrl/Cmd+B` | Toggle sidebar (Bold in a focused Markdown editor) |
| `Alt+Left` / `Alt+Right` | Back / forward |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous tab |
| `F2` | Rename the active file |
| `F10` | Keyboard menu navigation |
