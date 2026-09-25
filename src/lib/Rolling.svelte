<script lang="ts">
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";

  let { value, decimals = 0 }: { value: number; decimals?: number } = $props();

  const t = new Tween(0, { duration: 450, easing: cubicOut });
  $effect(() => {
    t.target = value;
  });
</script>

{t.current.toFixed(decimals)}
