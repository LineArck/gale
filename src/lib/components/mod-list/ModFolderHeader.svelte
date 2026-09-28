<script lang="ts">
	import type { ContextItem, Folder } from '$lib/types';
	import Icon from '@iconify/svelte';
	import { ContextMenu } from 'bits-ui';
	import ContextMenuContent from '$lib/components/ui/ContextMenuContent.svelte';
	import IconButton from '$lib/components/ui/IconButton.svelte';
	import { activeContextMenu } from '$lib/context';
	import { m } from '$lib/paraglide/messages';

	type Props = {
		folder: Folder;
		/** Whether the name is being edited. */
		editing?: boolean;
		/** Render without any interactivity, e.g. in the drag overlay. */
		preview?: boolean;
		ontoggle?: () => void;
		onrename?: (name: string) => void;
		ondelete?: () => void;
	};

	let {
		folder,
		editing = $bindable(false),
		preview = false,
		ontoggle,
		onrename,
		ondelete
	}: Props = $props();

	let contextMenuOpen = $state(false);
	let draftName = $state('');

	let modCount = $derived(folder.children.length + folder.unresolved.length);
	let enabledCount = $derived(folder.children.filter((mod) => mod.enabled ?? true).length);

	const contextItems: ContextItem[] = [
		{
			label: m.modFolders_rename(),
			icon: 'mdi:pencil',
			onclick: () => (editing = true)
		},
		{
			label: m.modFolders_delete(),
			icon: 'mdi:folder-remove',
			onclick: () => ondelete?.()
		}
	];

	$effect(() => {
		if ($activeContextMenu !== null && $activeContextMenu !== folder.id) {
			contextMenuOpen = false;
		}
	});

	$effect(() => {
		if (editing) draftName = folder.name;
	});

	function focusInput(input: HTMLInputElement) {
		// wait for the context menu to close, which would otherwise steal focus back
		setTimeout(() => {
			input.focus();
			input.select();
		}, 0);
	}

	function commit() {
		if (!editing) return;
		editing = false;

		const name = draftName.trim();
		if (name.length > 0 && name !== folder.name) {
			onrename?.(name);
		}
	}

	function onkeydown(evt: KeyboardEvent) {
		evt.stopPropagation();

		if (evt.key === 'Enter') {
			commit();
		} else if (evt.key === 'Escape') {
			editing = false;
		}
	}
</script>

{#snippet header()}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		onclick={() => !editing && ontoggle?.()}
		role="button"
		tabindex="-1"
		aria-expanded={!folder.collapsed}
		class={[
			'group text-primary-600 dark:text-primary-300 mx-2 flex h-[58px] items-center rounded-lg border p-2',
			preview
				? 'dark:bg-primary-700 bg-primary-200 dark:border-primary-500 border-primary-300'
				: 'dark:hover:bg-primary-700 hover:bg-primary-200 border-transparent'
		]}
	>
		<div
			class="bg-accent-100 dark:bg-accent-900/50 relative mr-3 flex size-12 shrink-0 items-center justify-center rounded-md"
		>
			<Icon
				icon={folder.collapsed ? 'mdi:folder' : 'mdi:folder-open'}
				class="text-accent-600 dark:text-accent-400 size-7"
			/>
		</div>

		<Icon
			icon="mdi:chevron-down"
			class={[
				'text-primary-400 mr-1 size-5 shrink-0 transition-transform duration-100',
				folder.collapsed && '-rotate-90'
			]}
		/>

		{#if editing}
			<input
				{@attach focusInput}
				bind:value={draftName}
				onblur={commit}
				{onkeydown}
				onclick={(evt) => evt.stopPropagation()}
				onpointerdown={(evt) => evt.stopPropagation()}
				maxlength={100}
				placeholder={m.modFolders_namePlaceholder()}
				class="bg-primary-100 dark:bg-primary-900 text-primary-800 focus:ring-accent-500 mr-2 min-w-0 grow rounded-md px-2 py-1 font-medium focus:ring-2 focus:outline-none dark:text-white"
			/>
		{:else}
			<span class="text-primary-800 mr-2 truncate font-medium dark:text-white">
				{folder.name}
			</span>
		{/if}

		<span
			class="bg-primary-200 dark:bg-primary-700 text-primary-600 dark:text-primary-300 shrink-0 rounded-full px-2 text-sm"
			title={m.modFolders_enabledCount({ enabled: enabledCount, total: modCount })}
		>
			{#if enabledCount < folder.children.length}
				{enabledCount}/{modCount}
			{:else}
				{modCount}
			{/if}
		</span>

		{#if !preview && !editing}
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="ml-auto hidden shrink-0 gap-0.5 group-hover:flex"
				onclick={(evt) => evt.stopPropagation()}
			>
				<IconButton
					icon="mdi:pencil"
					label={m.modFolders_rename()}
					showTooltip
					onclick={() => (editing = true)}
				/>
				<IconButton
					icon="mdi:folder-remove"
					label={m.modFolders_delete()}
					color="red"
					showTooltip
					onclick={() => ondelete?.()}
				/>
			</div>
		{/if}
	</div>
{/snippet}

{#if preview}
	{@render header()}
{:else}
	<ContextMenu.Root
		bind:open={contextMenuOpen}
		onOpenChange={(open) => ($activeContextMenu = open ? folder.id : null)}
	>
		<ContextMenu.Trigger class="contents">
			{@render header()}
		</ContextMenu.Trigger>
		<ContextMenuContent type="context" items={contextItems} />
	</ContextMenu.Root>
{/if}
