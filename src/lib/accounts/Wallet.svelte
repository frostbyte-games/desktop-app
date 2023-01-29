<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { activeAccount } from "./accounts";

  let account: string;
  activeAccount.subscribe(async (value: string) => {
    await setActiveAccount(value);
    account = value;
  });

  type Wallet = {
    address: string;
    balance: string;
  };

  let wallet: Promise<Wallet>;

  async function getBalance(account: string): Promise<Wallet> {
    return await invoke("balance", { account });
  }

  async function setActiveAccount(account: string): Promise<any> {
    return await invoke("set_active_account", { accountName: account });
  }

  let amount = "";
  let to = "";
  let successMessage = "";
  let errorMessage = "";

  async function transfer() {
    await invoke("transfer", { amount, to })
      .then(() => {
        successMessage = "successfully transfered snowflakes";
        wallet = getBalance(account);
      })
      .catch((err) => (errorMessage = "failed to transfer snowflakes"));
  }

  $: wallet = getBalance(account);
</script>

<div class="col">
  {#await wallet then wallet}
    <p>Address: {wallet.address}</p>
    <p>Snowflakes: {wallet.balance}</p>
  {/await}

  <input type="text" bind:value={amount} placeholder="Amount to send" />
  <input type="text" bind:value={to} placeholder="Destination address" />
  <button id="transfer" on:click={transfer}>Send</button>
  <p>{successMessage}</p>
  <p>{errorMessage}</p>
</div>

<style>
  p {
    color: lightblue;
    font-family: "Dank Mono", cursive;
    font-size: 2em;
  }
</style>
