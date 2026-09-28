<script lang="ts">
	import type { Folder } from '$lib/types';
	import { createSortable } from '@dnd-kit/svelte/sortable';
	import type { Snippet } from 'svelte';

	type Props = {
		folder: Folder;
		index: number;
		disabled?: boolean;
		children?: Snippet;
	};

	let { folder, index, disabled, children }: Props = $props();

	const sortable = createSortable({
		get id() {
			return folder.id;
		},
		get index() {
			return index;
		},
		get data() {
			return { folder };
		},
		get disabled() {
			return disabled;
		},
		transition: {
			duration: 150,
			easing: 'cubic-bezier(0.33, 1, 0.68, 1)'
		}
	});
</script>

<div
	{@attach sortable.attach}
	id={folder.id}
	class={['select-none', sortable.isDragging && 'opacity-40']}
>
	{@render children?.()}
</div>
