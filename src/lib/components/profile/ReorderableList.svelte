<script lang="ts">
	import { DragDropProvider, DragOverlay } from '@dnd-kit/svelte';
	import { isSortable } from '@dnd-kit/svelte/sortable';
	import { PointerActivationConstraints, PointerSensor } from '@dnd-kit/dom';
	import ReorderableMod from './ReorderableMod.svelte';
	import ReorderableFolder from './ReorderableFolder.svelte';
	import type { Folder, Mod, TreeEntry } from '$lib/types';
	import type { Snippet } from 'svelte';
	import VirtualList from '../ui/VirtualList.svelte';
	import ProfileModListItemNoContext from '../mod-list/ProfileModListItemNoContext.svelte';
	import ModFolderHeader from '../mod-list/ModFolderHeader.svelte';
	import { entryId, flattenTree, moveItem } from '$lib/modFolders';

	type Props = {
		tree: TreeEntry[];
		mod: Snippet<[{ mod: Mod; index: number }]>;
		folder: Snippet<[{ folder: Folder; index: number }]>;
		/** Called with the new tree when a drag operation finishes and changed the order. */
		onchange?: (tree: TreeEntry[]) => void;
		reorderable?: boolean;
	};

	let { tree = $bindable(), mod, folder, onchange, reorderable = true }: Props = $props();

	// while a folder is dragged, every folder is shown collapsed so folders move as a whole
	let draggingFolder = $state(false);
	let treeBeforeDrag: TreeEntry[] | null = null;

	let items = $derived(flattenTree(tree, draggingFolder));

	let virtualList: VirtualList<(typeof items)[number], string> | undefined = $state();

	export function scrollToTop() {
		virtualList?.scrollTo(0);
	}

	function onDragStart(event: any) {
		treeBeforeDrag = tree;
		draggingFolder = event.operation.source?.data?.folder !== undefined;
	}

	function onDragOver(event: any) {
		const { source, target } = event.operation;

		if (!isSortable(source) || !isSortable(target) || source.id === target.id) {
			return;
		}

		const fromIndex = items.findIndex((item) => entryId(item) === source.id);
		const toIndex = items.findIndex((item) => entryId(item) === target.id);

		if (fromIndex === -1 || toIndex === -1) {
			return;
		}

		const newTree = moveItem(tree, String(source.id), items[toIndex], toIndex > fromIndex);

		if (newTree !== null) {
			tree = newTree;
		}
	}

	function onDragEnd(event: any) {
		draggingFolder = false;

		const previous = treeBeforeDrag;
		treeBeforeDrag = null;

		if (previous === null || previous === tree) return;

		if (event.canceled) {
			tree = previous;
		} else {
			onchange?.(tree);
		}
	}
</script>

<DragDropProvider
	{onDragStart}
	{onDragOver}
	{onDragEnd}
	sensors={(defaults) => [
		...defaults.filter((sensor) => sensor !== PointerSensor),
		PointerSensor.configure({
			activationConstraints: [new PointerActivationConstraints.Distance({ value: 6 })]
		})
	]}
>
	<VirtualList bind:this={virtualList} {items} rowId={(item) => entryId(item)} itemHeight={58}>
		{#snippet children({ item, index })}
			{#if item.type === 'folder'}
				<ReorderableFolder folder={item.folder} {index} disabled={!reorderable}>
					{@render folder({ folder: item.folder, index })}
				</ReorderableFolder>
			{:else}
				<ReorderableMod mod={item.mod} {index} hovered={false} disabled={!reorderable}>
					{#if item.folderId !== null}
						<div class="border-accent-500/40 ml-6 border-l-2">
							{@render mod({ mod: item.mod, index })}
						</div>
					{:else}
						{@render mod({ mod: item.mod, index })}
					{/if}
				</ReorderableMod>
			{/if}
		{/snippet}
	</VirtualList>

	<DragOverlay dropAnimation={{ duration: 150, easing: 'cubic-bezier(0.33, 1, 0.68, 1)' }}>
		{#snippet children(source)}
			{#if source.data.folder}
				<ModFolderHeader folder={{ ...source.data.folder, collapsed: true }} preview />
			{:else}
				<ProfileModListItemNoContext mod={source.data.mod} />
			{/if}
		{/snippet}
	</DragOverlay>
</DragDropProvider>
