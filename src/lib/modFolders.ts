// Tree operations for mod folders in the custom mod order.
//
// The mod list is a tree of top-level entries (mods and folders), where folders contain mods.
// Folders can't be nested. The tree is flattened into rows for display, and converted
// to/from the backend's `LayoutEntry[]` representation.
//
// When the list is sorted in descending order, the tree is displayed reversed (both the
// top-level entries and each folder's children). All operations here work on the tree as
// it is displayed; `toLayout` converts it back to ascending order.

import type { Folder, LayoutEntry, ListItem, Mod, TreeEntry } from './types';

export function entryId(entry: TreeEntry | ListItem): string {
	return entry.type === 'folder' ? entry.folder.id : entry.mod.uuid;
}

function reverseTree(tree: TreeEntry[]): TreeEntry[] {
	return [...tree].reverse().map((entry) =>
		entry.type === 'folder'
			? {
					type: 'folder',
					folder: { ...entry.folder, children: [...entry.folder.children].reverse() }
				}
			: entry
	);
}

function cloneTree(tree: TreeEntry[]): TreeEntry[] {
	return tree.map((entry) =>
		entry.type === 'folder'
			? { type: 'folder', folder: { ...entry.folder, children: [...entry.folder.children] } }
			: entry
	);
}

export function buildTree(layout: LayoutEntry[], mods: Mod[], reversed: boolean): TreeEntry[] {
	const byUuid = new Map(mods.map((mod) => [mod.uuid, mod]));
	const used = new Set<string>();
	const tree: TreeEntry[] = [];

	for (const entry of layout) {
		if (entry.type === 'mod') {
			const mod = byUuid.get(entry.uuid);
			if (mod && !used.has(mod.uuid)) {
				used.add(mod.uuid);
				tree.push({ type: 'mod', mod });
			}
			continue;
		}

		const children: Mod[] = [];
		const unresolved: string[] = [];

		for (const uuid of entry.mods) {
			const mod = byUuid.get(uuid);
			if (!mod) {
				// e.g. mods that aren't found on Thunderstore, keep them in the folder
				unresolved.push(uuid);
			} else if (!used.has(uuid)) {
				used.add(uuid);
				children.push(mod);
			}
		}

		tree.push({
			type: 'folder',
			folder: {
				id: entry.id,
				name: entry.name,
				collapsed: entry.collapsed,
				children,
				unresolved
			}
		});
	}

	for (const mod of mods) {
		if (!used.has(mod.uuid)) {
			tree.push({ type: 'mod', mod });
		}
	}

	return reversed ? reverseTree(tree) : tree;
}

export function toLayout(tree: TreeEntry[], reversed: boolean): LayoutEntry[] {
	if (reversed) tree = reverseTree(tree);

	return tree.map((entry) =>
		entry.type === 'mod'
			? { type: 'mod', uuid: entry.mod.uuid }
			: {
					type: 'folder',
					id: entry.folder.id,
					name: entry.folder.name,
					collapsed: entry.folder.collapsed,
					mods: [...entry.folder.children.map((mod) => mod.uuid), ...entry.folder.unresolved]
				}
	);
}

/**
 * Flattens the tree into rows. Children of collapsed folders are omitted.
 *
 * @param collapseAll Treat every folder as collapsed, used while dragging a folder.
 */
export function flattenTree(tree: TreeEntry[], collapseAll = false): ListItem[] {
	const rows: ListItem[] = [];

	for (const entry of tree) {
		if (entry.type === 'mod') {
			rows.push({ type: 'mod', mod: entry.mod, folderId: null });
			continue;
		}

		rows.push(entry);

		if (!collapseAll && !entry.folder.collapsed) {
			for (const mod of entry.folder.children) {
				rows.push({ type: 'mod', mod, folderId: entry.folder.id });
			}
		}
	}

	return rows;
}

export function folders(tree: TreeEntry[]): Folder[] {
	return tree.flatMap((entry) => (entry.type === 'folder' ? [entry.folder] : []));
}

export function folderOf(tree: TreeEntry[], uuid: string): Folder | null {
	return folders(tree).find((folder) => folder.children.some((mod) => mod.uuid === uuid)) ?? null;
}

/** Index of the top-level entry that is or contains the item with the given id. */
function topLevelIndex(tree: TreeEntry[], id: string): number {
	return tree.findIndex(
		(entry) =>
			entryId(entry) === id ||
			(entry.type === 'folder' && entry.folder.children.some((mod) => mod.uuid === id))
	);
}

/** Removes a mod from the (cloned) tree, returning it and where it was. */
function takeMod(
	tree: TreeEntry[],
	uuid: string
): { mod: Mod; topIndex: number; inFolder: boolean } | null {
	const topIndex = topLevelIndex(tree, uuid);
	if (topIndex === -1) return null;

	const entry = tree[topIndex];

	if (entry.type === 'mod') {
		tree.splice(topIndex, 1);
		return { mod: entry.mod, topIndex, inFolder: false };
	}

	const index = entry.folder.children.findIndex((mod) => mod.uuid === uuid);
	const [mod] = entry.folder.children.splice(index, 1);
	return { mod, topIndex, inFolder: true };
}

/**
 * Moves the item with `sourceId` onto the row `target`, as happens continuously while dragging.
 *
 * - Folders only move between top-level entries (they move past whole folders).
 * - A mod takes the place, and folder, of the mod it's moved onto.
 * - A mod moved down onto an expanded folder becomes its first child; moved up onto a
 *   folder, it's placed above the folder.
 *
 * @param down Whether the source is being moved down the list (towards the target).
 * @returns The new tree, or `null` if nothing changed.
 */
export function moveItem(
	tree: TreeEntry[],
	sourceId: string,
	target: ListItem,
	down: boolean
): TreeEntry[] | null {
	const targetId = entryId(target);
	if (sourceId === targetId) return null;

	const next = cloneTree(tree);
	const offset = down ? 1 : 0;

	const folderIndex = next.findIndex(
		(entry) => entry.type === 'folder' && entry.folder.id === sourceId
	);

	if (folderIndex !== -1) {
		if (topLevelIndex(next, targetId) === folderIndex) return null;

		const [folder] = next.splice(folderIndex, 1);
		next.splice(topLevelIndex(next, targetId) + offset, 0, folder);
		return next;
	}

	const taken = takeMod(next, sourceId);
	if (taken === null) return null;
	const { mod } = taken;

	const targetIndex = topLevelIndex(next, targetId);
	if (targetIndex === -1) return null;
	const targetEntry = next[targetIndex];

	if (target.type === 'folder' && targetEntry.type === 'folder') {
		if (down && !targetEntry.folder.collapsed) {
			targetEntry.folder.children.unshift(mod);
		} else {
			next.splice(targetIndex + offset, 0, { type: 'mod', mod });
		}
	} else if (targetEntry.type === 'folder') {
		const index = targetEntry.folder.children.findIndex((child) => child.uuid === targetId);
		targetEntry.folder.children.splice(index + offset, 0, mod);
	} else {
		next.splice(targetIndex + offset, 0, { type: 'mod', mod });
	}

	return next;
}

/** Moves a mod to the end of a folder, or out of its folder if `folderId` is null. */
export function moveModToFolder(
	tree: TreeEntry[],
	uuid: string,
	folderId: string | null
): TreeEntry[] {
	const next = cloneTree(tree);
	const taken = takeMod(next, uuid);
	if (taken === null) return tree;

	if (folderId === null) {
		// place it right after the folder it was in
		const index = taken.inFolder ? taken.topIndex + 1 : taken.topIndex;
		next.splice(index, 0, { type: 'mod', mod: taken.mod });
		return next;
	}

	const folder = folders(next).find((folder) => folder.id === folderId);
	if (!folder) return tree;

	folder.children.push(taken.mod);
	return next;
}

export function createFolder(tree: TreeEntry[], name: string): { tree: TreeEntry[]; id: string } {
	const id = crypto.randomUUID();
	const folder: Folder = { id, name, collapsed: false, children: [], unresolved: [] };

	return { tree: [{ type: 'folder', folder }, ...tree], id };
}

export function updateFolder(
	tree: TreeEntry[],
	id: string,
	update: Partial<Pick<Folder, 'name' | 'collapsed'>>
): TreeEntry[] {
	return tree.map((entry) =>
		entry.type === 'folder' && entry.folder.id === id
			? { type: 'folder', folder: { ...entry.folder, ...update } }
			: entry
	);
}

/** Deletes a folder, moving its mods to where the folder was. */
export function deleteFolder(tree: TreeEntry[], id: string): TreeEntry[] {
	return tree.flatMap((entry): TreeEntry[] =>
		entry.type === 'folder' && entry.folder.id === id
			? entry.folder.children.map((mod) => ({ type: 'mod', mod }))
			: [entry]
	);
}
