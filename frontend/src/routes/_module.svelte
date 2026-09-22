<script lang="ts">
    import Sidebar from "@/components/Sidebar.svelte"
</script>

<div class="shell">
    <Sidebar />
    <main class="content">
        <slot></slot>
    </main>
</div>

<style lang="scss">
    .shell {
        display: flex;
        height: 100vh;
        overflow: hidden;
    }

    .content {
        flex: 1;
        min-width: 0;
        height: 100%;
        overflow-y: auto;
        padding: var(--content-padding);
    }
</style>
