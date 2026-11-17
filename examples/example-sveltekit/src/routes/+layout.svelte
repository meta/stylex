<script lang="ts">
	import favicon from '#lib/assets/favicon.svg';
	import "../app.css";
	if (import.meta.env.DEV) {
		// @ts-expect-error // Virtual modules are not typed
		$effect(() => import('virtual:stylex:runtime'))
	}

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	{#if import.meta.env.DEV}
		<link rel="stylesheet" href="/virtual:stylex.css" />
	{/if}
</svelte:head>

{@render children()}
