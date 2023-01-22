<script lang="ts">
  export let account: string;

  import { invoke } from "@tauri-apps/api/tauri";
  import { onMount } from "svelte";

  let balance = "";

  async function getBalance(): Promise<string> {
    return await invoke("balance", { account });
  }

  onMount(async () => {
    balance = await getBalance();
  });
</script>

<div class="col">
  <p>{account} {balance} snowflakes!</p>
</div>

<style>
  p {
    color: lightblue;
    font-family: "Dank Mono", cursive;
    font-size: 2em;
  }
</style>
