<script lang="ts">
    import { colorThemeStore } from '$lib/stores/colorThemeStore';
    import Button from '../common/Button.svelte';

    function toggleColorTheme() {
        colorThemeStore.update(theme => theme === 'dark' ? 'light' : 'dark');
    }
</script>

<div class="theme-settings">
    <div class="setting-group">
        <h3>Color Theme</h3>
        <div class="button-group">
            <Button
                variant="yellow"
                size="medium"
                state={$colorThemeStore === 'light' ? 'selected' : 'default'}
                onClick={() => colorThemeStore.set('light')}
            >
                Light
            </Button>
            <Button
                variant="yellow"
                size="medium"
                state={$colorThemeStore === 'dark' ? 'selected' : 'default'}
                onClick={() => colorThemeStore.set('dark')}
            >
                Dark
            </Button>
        </div>
    </div>
</div>

<style lang="postcss">
    .theme-settings {
        @apply flex flex-col gap-6 p-4;
    }

    .setting-group {
        @apply flex flex-col gap-3;
    }

    .setting-group h3 {
        @apply text-lg font-semibold text-gray-200;
    }

    .button-group {
        @apply flex gap-2;
    }

    :global(.light-theme) .setting-group h3 {
        @apply text-gray-800;
    }
</style>
