<script lang="ts">
  export let account: string;

  import { invoke } from "@tauri-apps/api/tauri";
  import { onMount } from "svelte";

  type Wallet = {
    address: string;
    balance: string;
  };

  let wallet: Promise<Wallet>;

  async function getBalance(account: string): Promise<Wallet> {
    return await invoke("balance", { account });
  }

  onMount(async () => {
    wallet = getBalance(account);
  });

  $: wallet = getBalance(account);
</script>

<div class="col">
  {#await wallet then wallet}
    <p>{account} ({wallet.address}) have {wallet.balance} snowflakes</p>{/await}
</div>

<style>
  p {
    color: lightblue;
    font-family: "Dank Mono", cursive;
    font-size: 2em;
  }
</style>
